// SPDX-License-Identifier: MIT
//
// Differential battery: compile the SAME C function through BOTH
//   - the open-sober loader->JIT pipeline (cross-gcc aarch64 -> elfjit), and
//   - a native x86-64 oracle (`gcc` on this host), same source / same -O level,
// then require the JIT's `entry()` return to EXACTLY equal the native result.
//
// A single-value regression only proves "got 42"; a differential run proves
// the translated code SIDESTEPPED no silent miscompile on a whole family of
// inputs. This is the mechanism that surfaced the LogicImm ROR bug, the
// SimdPairAddD collision, the saddw2 upper-half bug, the MSUB direction bug
// etc. — silent miscompiles that matched on symmetric/degenerate cases.
//
// Each `#[test]` names the instruction family it is a *canary* for. When a
// family regresses, jit vs oracle diverge and the test fails with both values.
// Requires `aarch64-linux-gnu-gcc` AND `gcc` on PATH; skips if either is absent
// (so the suite stays green on hosts without a full toolchain).

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use arm64jit::jit::{CpuState, jit_run};
use libloader::elf::load_elf_image;

fn run_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn tools_ok() -> bool {
    for t in ["aarch64-linux-gnu-gcc", "gcc"] {
        if !Command::new(t).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            return false;
        }
    }
    true
}

fn workdir(tag: &str) -> PathBuf {
    let d =
        std::env::temp_dir().join(format!("open-sober-diff-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Build the oracle executable (native x86-64): compile the same `src` plus a
/// `main` that prints the full 64-bit `entry()` result, run it, and return the
/// printed value. `-O` level mirrors the cross build.
fn oracle(src: &str, opt: &str) -> u64 {
    let wd = workdir("oracle");
    std::fs::write(
        wd.join("o.c"),
        format!("{src}\n#include <stdio.h>\nint main(void){{ printf(\"%llu\\n\", (unsigned long long)entry()); return 0; }}\n"),
    )
    .unwrap();
    let out = Command::new("gcc")
        .args([opt, "-w"])
        .arg(wd.join("o.c"))
        .arg("-o")
        .arg(wd.join("o"))
        .output()
        .unwrap_or_else(|e| panic!("oracle gcc failed: {e}"));
    if !out.status.success() {
        panic!("oracle compile failed:\n{}", String::from_utf8_lossy(&out.stderr));
    }
    let run = Command::new(wd.join("o"))
        .output()
        .unwrap_or_else(|e| panic!("oracle run failed: {e}"));
    let s = String::from_utf8_lossy(&run.stdout);
    s.trim().parse::<u64>().unwrap_or_else(|e| panic!("oracle non-numeric output {s:?}: {e}"))
}

/// Cross-compile `src` (+ an ignored `main` for type-completeness) to a static
/// aarch64 ELF with `entry` as its entry point.
fn compile_arm(workdir: &std::path::Path, name: &str, src: &str, opt: &str) -> PathBuf {
    std::fs::write(
        workdir.join(format!("{name}.arm.c")),
        format!("{src}\nint main(){{return 0;}}\n"),
    )
    .unwrap();
    let elf = workdir.join(format!("{name}.elf"));
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-static", "-nostdlib", "-Wl,-e,entry", opt, "-w"])
        .arg(workdir.join(format!("{name}.arm.c")))
        .arg("-o")
        .arg(&elf)
        .output()
        .unwrap_or_else(|e| panic!("cross-gcc failed: {e}"));
    assert!(out.status.success(), "cross-gcc failed:\n{}", String::from_utf8_lossy(&out.stderr));
    elf
}

/// Load + bind + bootstrap + run the aarch64 ELF through jit_run (mirrors
/// `examples/elfjit.rs` and the loader_run harness).
fn run_elf(path: &std::path::Path) -> Result<u64, String> {
    let el = unsafe { load_elf_image(path) }.map_err(|e| format!("load_elf_image: {e:#}"))?;
    let (_n, _u) = arm64jit::plt::bind_image_plt(&el, None);
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .ok_or_else(|| "no executable segment".to_string())?;
    let base = seg.guest_vaddr;
    let full_end =
        el.segments.iter().fold(0u64, |m, s| m.max(s.guest_vaddr + s.memsz));
    let len = (full_end - base) as usize;
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, len) };

    let mut st = CpuState::new();
    let entry = el.info.entry;

    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let stack = Box::leak(vec![0u8; STACK_SIZE].into_boxed_slice());
    let mut auxv = arm64jit::boot::standard_auxv(&el, arm64jit::boot::HWCAP_FP | arm64jit::boot::HWCAP_ASIMD, 0);
    let sp = arm64jit::boot::layout_initial_stack(stack.as_ptr() as *mut u8, STACK_SIZE, Some(&[0u8; 0]), &[], &mut auxv);
    st.set(31, sp);
    let tls = Box::leak(vec![0u8; 64 * 1024].into_boxed_slice());
    st.tpidr = tls.as_ptr() as u64;

    jit_run(image, base, entry, &mut st as *mut CpuState)
}

/// Full differential: cross-compile, oracle-compile, run both, assert equal.
fn assert_diff(tag: &str, opt: &str, src: &str) {
    if !tools_ok() {
        eprintln!("skipping {tag}: toolchain not available");
        return;
    }
    let _g = run_lock().lock().unwrap_or_else(|e| e.into_inner());
    let wd = workdir(tag);
    let expected = oracle(src, opt);
    let elf = compile_arm(&wd, tag, src, opt);
    let got = match run_elf(&elf) {
        Ok(v) => v,
        Err(e) => panic!("{tag}: jit_run failed (oracle = {expected}): {e}"),
    };
    assert_eq!(got, expected,
        "\x1b[31mMISCOMPILE\x1b[0m {tag} (-{opt}): jit returned {got}, native oracle returned {expected}");
    eprintln!("\x1b[32mPASS\x1b[0m {tag} (-{opt}): jit == oracle == {got}");
    let _ = std::fs::remove_dir_all(&wd);
}

// ---- Canaries ----

#[test]
fn diff_simd_ext_unaligned_xor_reduce() {
    // Byte-copy `ext VD.16B, Vn, Vm, #imm` with UNALIGNED imm (4): the SimdExt
    // translate computed the shift as `start%8` BYTES but used it as a BIT count
    // (shr/shl by 4 instead of 32), corrupting every non-{0,8} immediate. gcc's
    // horizontal XOR-reduce over lanes emits `ext #8` + `ext #4`; before the fix
    // the final xor was wrong. Found via the new gen_varshift fuzz campaign.
    assert_diff(
        "simd_ext_unaligned",
        "-O3",
        r#"
typedef int i32;
volatile unsigned long long seedv = 424242ull;
long long entry(void){
    unsigned long long x = seedv;
    i32 a[32];
    for(int i=0;i<32;i++){ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(i32)((x>>8)^(x&0xffff)); }
    i32 s1=0;
    for(int i=0;i<32;i++) s1 ^= a[i];   // gcc -O3 vectorizes + cross-lane ext reduce
    return (long long)(s1*131) ^ 0xf00baaull;
}
"#,
    );
}

#[test]
fn diff_simd_var_reg_shift_2d() {
    // ushl/sshl Vd.2d, Vn.2d, Vm.2d: 64-bit lanes, variable shift by the SIGNED
    // count lane. The old vshift gate `&0x3f000c00 {0x2e,0x4e,0x6e}` wrongly
    // masked-out bit29 (the U bit) AND the 2d size bits, so sshl.2d (masked
    // 0x0e...) never decoded; the translate always used 16/esize lanes (ignored q)
    // and did a plain shl (no negative-count arithmetic right shift for sshl).
    // Bounded counts (<64) so native oracle and ARM agree.
    assert_diff(
        "simd_var_reg_shift_2d",
        "-O3",
        r#"
typedef unsigned long long u64;
volatile unsigned long long seedv = 864209ull;
long long entry(void){
    unsigned long long x = seedv;
    u64 a[24], c[24];
    for(int i=0;i<24;i++){ x=x*6364136223846793005ull+1442695040888963407ull;
        a[i]=(u64)((x>>8)^(x&0xffff)); c[i]=(u64)((x>>37)&7); }
    u64 s1=0, s2=0;
    for(int i=0;i<24;i++){ s1 ^= (u64)(a[i] << c[i]); s2 |= (u64)(a[i] >> c[i]); }
    return (long long)(s1*131 + s2*17);
}
"#,
    );
}

#[test]
fn diff_i64_div_mod_sign() {
    // SDIV/UDIV signedness + `msr`/magic-multiply division-by-constant: i64
    // (incl. negative dividend/divisor) and u64 modulo.
    assert_diff(
        "i64_divmod",
        "-O2",
        r#"
long long q,r;
static long long d(long long a,long long b){ q=a/b; r=a%b; return q*31 + r; }
unsigned long long ud(unsigned long long a,unsigned long long b){ return (a/b)*17 + (a%b); }
long long entry(void){
    long long acc = d(-100,7) + d(100,-7) + d(-100,-7) + d(1234567890123LL,97);
    acc += (long long)ud(0xdeadbeefdeadbeefULL, 1009ULL);
    acc += (-7 % 3) == -1 ? 1 : 0;  // C99 trunc-toward-zero
    acc += (7 % -3) == 1 ? 2 : 0;
    return acc;
}
"#,
    );
}

#[test]
fn diff_float_round_and_conv() {
    // FCVTZS trunc-to-int (/FP->int), FADD/FMUL/FDIV/FSUB, D/S width, int->FP
    // (SCVTF/UCVTF), incl. negative and subnormal/ulp cases. Deliberately NO
    // libm calls (llrint/rint would need a host libm the -nostdlib JIT lacks).
    assert_diff(
        "fp_roundconv",
        "-O2",
        r#"
static long long fc(double x){ return (long long)x; }               // fcvtzs (trunc)
static int fcs(float x){ return (int)x; }                            // fcvtzs s->int
static double fd(double x){ return (x * 2.5) + (x / 3.0) - x; }
static double fadd(double x,double y){ return x + y; }
static double fsub(double x,double y){ return x - y; }
static double fmul(double x,double y){ return x * y; }
static double fdiv(double x,double y){ return x / y; }
long long entry(void){
    long long s = fc(3.7) + fc(-3.7) + fc(2.0000001) + fc(-0.6);
    s += (long long)fd(6.0) * 1000000; // 2.5*6 + 6/3 - 6 = 11.0
    s += fadd(1e15, 1.0) == 1e15 ? 10 : 1;            // ulp rounding
    s += fsub(1.0, 1e-16) != 1.0 ? 20 : 2;            // subnormal handles
    s += fmul(1.5, 0.5) * 10;                          // 7.5
    s += (long long)(fdiv(7.0, 2.0) * 10);             // 35
    s += fcs(3.7f) + fcs(-3.7f);                       // fcvtzs s->int
    s += (long long)(double)12345678 + 1;              // scvtf
    s += (double)0xdeadbeef12345678ULL > 1.0e18 ? 400 : 40; // ucvtf
    s += (double)0x8000000000000000ULL > 9.0e18 ? 800 : 80;  // ucvtf high bit
    return s;
}
"#,
    );
}

#[test]
fn diff_loop_iteration_count() {
    // Does the SIMD vectorized `m[k]=k` loop terminate at the right number of
    // iterations? (isolation for the 6-iteration corruption)
    assert_diff(
        "count6",
        "-O2",
        r#"
long long entry(void){ volatile long long cnt=0; long long m[24]; for(int k=0;k<24;k++){ m[k]=k; cnt++; } return cnt; }
"#,
    );
}

#[test]
fn diff_simd_widen_init_loop() {
    // gcc -O2 vectorized int->i64 widening init loops (movi v.4s + sxtl +
    // sxtl2 + stp q + b.ne back-edge). Regression gate for:
    //  (a) the sxtl2/uxtl2 upper-half source fix (Q bit), and
    //  (b) the ARM `nop` misdecode as scvtf-fixed (system family 0xd5... was
    //      swallowed by the ScvtfFixed gate, converting x0 -> double into a
    //      vector reg; intermittent / address-dependent).
    // maskf: m[k] = k & 0xf ; mod_pow2: (k*7)%16 ; times7: k*7 (shl+sub).
    assert_diff(
        "maskf",
        "-O2",
        r#"
long long entry(void){
    long long m[24]; for(int k=0;k<24;k++) m[k] = k & 0xf;
    return m[3] + m[17]*1000 + m[23]*1000000;
}
"#,
    );
    assert_diff(
        "mod_pow2",
        "-O2",
        r#"
long long entry(void){
    long long m[24]; for(int k=0;k<24;k++) m[k] = (k*7) % 16;
    long long acc = m[3] + m[17]*1000 + m[23]*1000000;
    return acc;
}
"#,
    );
    assert_diff(
        "times7",
        "-O2",
        r#"
long long entry(void){
    long long m[24]; for(int k=0;k<24;k++) m[k] = k*7;
    return m[3] + m[17]*1000 + m[23]*1000000;
}
"#,
    );
    // magic-division `%101`: smull/smull2/uzp2/sshr/mls widen path.
    // KNOWN-OPEN deterministic bug — see diff_magic_div_known_broken.
}

#[test]
fn diff_magic_div() {
    // The gcc magic-division `%101` reducer (smull / smull2 / uzp2 / sshr /
    // mls). Root-caused & fixed: `mls` (multiply-subtract) was decoded as a
    // plain Simd4s SUBTRACT (losing the multiply) and `uzp2` (unpack-high,
    // gathers the product's high 32-bit words) was misdecoded as rev64. Both
    // are now proper ops; the remainder rounds correctly (m[0]=0 for k=0).
    assert_diff(
        "magicdiv",
        "-O2",
        r#"
long long entry(void){
    long long m[24]; for(int k=0;k<24;k++) m[k] = (k*7) % 101;
    long long acc = m[3] + m[17]*1000 + m[23]*1000000;
    return acc;
}
"#,
    );
}

#[test]
fn diff_struct_array_fields() {
    // struct-field offsets + register-offset LDR/STR + 2D-array pointer
    // indexing, plus a real vectorized %101 array — the combined case that
    // regressed under the nop misdecode.
    assert_diff(
        "struct_arr",
        "-O2",
        r#"
struct S { char c; long long a; short s; int i[4]; };
static long long fx(struct S *p){ p->a = p->c * 1000000 + p->s; return p->a + p->i[2]; }
long long entry(void){
    struct S s; s.c = 5; s.s = -7; s.i[0]=1;s.i[1]=2;s.i[2]=3;s.i[3]=4;
    long long m[24]; for(int k=0;k<24;k++) m[k] = (k*7) % 101;
    long long acc = fx(&s);
    acc += m[3]*1 + m[17]*1000 + m[23]*1000000;
    long long t[3][3]; for(int i=0;i<3;i++)for(int j=0;j<3;j++) t[i][j]=i*10+j;
    acc += t[2][2] + t[0][1]*100 + t[1][0]*10000;
    return acc;
}
"#,
    );
}

#[test]
fn diff_mixed_arith_accumulate() {
    // The -O3 optimizer-vectorized i64 reduction (+addp/smulh/mls) — previously
    // `#[ignore]`d as intermittent; the nop fix makes it deterministic.
    assert_diff(
        "mixed",
        "-O3",
        r#"
long long entry(void){
    long long a[32]; for(int i=0;i<32;i++) a[i] = (long long)i*i - 3*i + 7;
    long long s = 0; for(int i=0;i<32;i++) s += a[i];
    long long acc = s;                    // 31*32*63/6... computed
    for(int i=0;i<16;i++) acc = acc*3 + a[i*2];
    int sum = 0; for(int i=0;i<100;i++) sum += i%7;
    acc += sum * 1000000;
    acc %= 1000000007;
    return acc;
}
"#,
    );
}

#[test]
fn diff_bitfield_and_shift() {
    // UBFM/SBFM/BFM/BFC: sign-extract, lsl/lsr/asr by const and var, rotate
    // (EXTR/ROR), bitfield insert, byte/halfword movzx/sx.
    assert_diff(
        "bitshift",
        "-O1",
        r#"
static unsigned long long ror64(unsigned long long x, int n){ return (x>>n) | (x<<(64-n)); }
static long long sxt32(long long x){ return (long long)(int)x; }        // sxtw
static unsigned long long uxt16(unsigned long long x){ return (unsigned short)x; }
static unsigned long long bext(unsigned long long x,int s,int e){ return (x>>s) & ((1ULL<<(e-s+1))-1); }
long long entry(void){
    long long x = 0x123456789abcdef0LL;
    long long acc = 0;
    acc += ror64(x, 12) == 0xef0123456789abcdULL ? 1000000 : 1;
    acc += ror64(x, 47) == 0xcef0123456789abULL ? 2000000 : 2;
    acc += (x << 7) == 0x9a2b3c4d5e6f7800ULL ? 3000000 : 3;
    acc += (x >> 4) == 0x0123456789abcdefLL ? 4000000 : 4;
    acc += sxt32(0xffffffffffffff80ULL) == -128 ? 5000000 : 5;
    acc += uxt16(0x12345678) == 0x5678 ? 6000000 : 6;
    acc += bext(0xffffffffffffffffULL, 8, 15) == 0xff ? 7000000 : 7;
    acc += ((long long)x >> 60) == -2 ? 8000000 : 8; // asr sign-extend
    // var shift
    int n = 21;
    acc += (x >> n) == 0x91a2b3c4d5e6f7LL ? 9000000 : 9;
    acc += ((unsigned long long)x << n) == 0x12345 // shifted value computed at runtime identically
        ? 10000000 : 10;
    return acc;
}
"#,
    );
}

#[test]
fn diff_unsigned_compare_branches() {
    // B.HI/LO (unsigned) vs B.GT/LT (signed) — NZCV decoding from SUBS/ADDS/
    // ANDS-sets, plus csel/cset on the same flags.
    assert_diff(
        "ucompare",
        "-O2",
        r#"
static long long cls(unsigned long long a,unsigned long long b){
    if(a > b) return 1;      // unsigned HI
    if(a < b) return 2;      // unsigned LO
    return 0;
}
long long entry(void){
    long long acc = 0;
    acc += cls(0xffffffffffffffffULL, 1ULL) * 11;    // > -> 1
    acc += cls(1ULL, 0xffffffffffffffffULL) * 3;     // < -> 2
    acc += cls(7ULL, 7ULL);
    long long s = -100, t = 50;
    acc += (s < t) ? 100000 : 1;      // signed LT
    acc += (s > t) ? 2 : 200000;      // signed GT
    acc += ((unsigned)-1) > ((unsigned)0) ? 300000 : 3; // unsigned HI on w
    return acc;
}
"#,
    );
}

#[test]
fn diff_unsigned_magic_div_umull() {
    // Unsigned magic division drives the LONG path gcc picks for divisors that
    // aren't a power of two: it widens with `umull2`+`uzp2` (the unsigned
    // sibling of the signed smull %101 reducer) or `umaddh`. Guards the
    // umull/umaddh variant of the uzp2 quotient gather.
    assert_diff(
        "umagic",
        "-O2",
        r#"
long long entry(void){
    unsigned long long m[24];
    for(int k=0;k<24;k++) m[k] = (k*97u) / 1000u;
    unsigned long long acc = m[3] + m[17]*1000 + m[23]*1000000;
    return (long long)acc;
}
"#,
    );
    assert_diff(
        "umagic2",
        "-O2",
        r#"
long long entry(void){
    unsigned long long m[24];
    for(int k=0;k<24;k++) m[k] = (k*1024u + 31337u) / 65521u;
    unsigned long long acc = m[3] + m[17]*1000 + m[23]*1000000;
    return (long long)acc;
}
"#,
    );
}

#[test]
fn diff_long_accumulate_widening() {
    // Widening long-multiply / long-add accumulation: `umlal`/`smlal`/`uaddl`/
    // `mla` accumulate a 64-bit sum from 32-bit lane products — the family that
    // reuses the mla carry chain and the smull2 upper-half gather.
    assert_diff(
        "longacc",
        "-O3",
        r#"
long long entry(void){
    long long acc = 0;
    for(int i=0;i<300;i++){
        acc += (long long)i * (long long)(i & 0xff);
        acc -= (long long)(i*3) * 2;
    }
    return acc;
}
"#,
    );
    assert_diff(
        "ulongacc",
        "-O3",
        r#"
long long entry(void){
    unsigned long long acc = 7;
    for(int i=0;i<250;i++) acc += (unsigned long long)(i*31u) * 17u;
    return (long long)(acc & 0xffffffff);
}
"#,
    );
}

#[test]
fn diff_byte_scan_strlen() {
    // Byte-scan loops compiler lowers to SIMD compare/branch or scalar ldrb/
    // cbnz + zero-count: exercises load/increment/compare/branch idioms and
    // the vectorized "-1 then count-zeroes" trick.
    assert_diff(
        "strlen",
        "-O2",
        r#"
long long entry(void){
    const unsigned char s[] = "the quick brown fox jumps over the lazy dog";
    long long n=0; while(s[n]) n++;
    long long p=0; while(n--){
        if(s[p]==(unsigned char)0x61) break;   // 'a'
        p++;
    }
    return n*1000 + p;
}
"#,
    );
}

// ---- NEON single-precision (float) vector SIMD ----
// A 3D engine's vertex/matrix math is ~all .4s float-vector SIMD (fadd/fmul/
// fmla/fmaxnm/fcvtzs/scvtf, incl. the by-element FMLA the boot trace cleared).
// Plain -O3 gcc vectorizes elementwise float loops (no reassociation, so it
// stays legal FP). This battery is the canary for that whole register-lane
// family, which the older integer/double batteries did not touch.

#[test]
fn diff_ccmp_cond_compare() {
    // gcc lowers `a < N && b != N` loop guards to `cmp` + `ccmp x,#0,#nzcv,cond`
    // + `b.ge`. Pre-fix these decoded as a logical set-flags op -> wrong NZCV ->
    // the loop never terminated (JIT hang). Also covers ccmn/ccmp register form.
    assert_diff(
        "ccmp_cond",
        "-O2",
        r#"
long long entry(void){
    long long n = 0;
    long long a = 0, b = 5;
    // while (a < 4 && b != 7)  -> the ccmp x1,#0,#1,ne idiom
    while (a < 4 && b != 7) { n++; a++; }
    long long acc = n; // 4
    int x = 3, y = 10;
    if (x < 5 && y > 5) acc += 100;      // ccmp with ge
    if (x == 3 || y == 100) acc += 1000; // ccmn/ccmp with eq
    if (y >= 10 && x <= 3) acc += 10000; // ge/le ccmp
    return acc;
}
"#,
    );
    assert_diff(
        "ccmp_masked_loop",
        "-O3",
        r#"
long long entry(void){
    // gcc may rotate this into cmp/ccmp + branch; 32-bit counter
    long long cnt = 0;
    for (int k = 0; k < 1000 && (k & 3) != 0; k++) cnt++;
    return cnt + 5;
}
"#,
    );
}

#[test]
fn diff_w32_overflow_compare() {
    // 32-bit arithmetic must set NZCV from the 32-bit result (N=bit31), not a
    // 64-bit add with the operand pre-zero-extended (which gave N=0 for
    // 0x7fffffff+1). `s = a+b` wrapping to INT_MIN then `s<0` must be true.
    assert_diff(
        "w32_ovf_cmp",
        "-O2",
        r#"
long long entry(void){
    volatile int a = 0x7fffffff, b = 1;
    int s = a + b;                     // wraps to INT_MIN in w
    unsigned u = (unsigned)a + (unsigned)b; // wraps to 0x80000000
    long long acc = (long long)s;      // -2147483648
    acc += (long long)u;               // +2147483648 = 0
    acc += (s < 0) ? 1000000 : 1;
    acc += (u > 1000000000u) ? 2000000 : 2;
    return acc;
}
"#,
    );
}

#[test]
fn diff_fp_compare_zero_and_cset() {
    // Three coupled correctness gaps flushed out together:
    //  (a) `cset/cinc x0,wzr,eq` (csinc with rn=rm=31) read guest register 31
    //      (the SP slot) instead of XZR=0 — any `x==0.0` result got sp+1.
    //  (b) `fcmp dN, #0.0` was decoded/translated as a compare against vector
    //      reg d0 (garbage) instead of literal zero (bit3 discriminator).
    //  (c) scalar `scvtf Dd,Dn` (int stored in an FP reg back to float) had the
    //      double/single `sng` discriminator inverted, truncating to an f32.
    // fcvt_iso / fcvt_rt2 / eqzero each isolate one; this combines them.
    assert_diff(
        "fp_fcmp_zero_cset",
        "-O2",
        r#"
long long entry(void){
    volatile double z = 0.0;
    volatile double f = 350.0;
    double rt = (double)(long long)f;   // fcvtzs + scvtf via FP reg
    long long acc = (long long)(rt + 0.0);  // scalar add, stays 350
    acc += (z == 0.0) ? 1 : 0;          // fcmp dN,#0.0 + cset (wzr) -> +1
    acc += (z != 0.0) ? 100 : 0;
    acc += (rt < 351.0) ? 10 : 0;
    return acc;                          // 350 + 1 + 10 = 361
}
"#,
    );
    assert_diff(
        "fp_rt_cset_chain",
        "-O3",
        r#"
long long entry(void){
    volatile double q=2.0, r=3.5, mult=100.0;
    volatile double z=0.0;
    double a = r*mult;                          // 350
    double b = (double)(long long)a;            // fcvtzs d,d ; scvtf d,d round-trip
    double c = b + q*mult;                      // 350 + 200 = 550
    long long rv = (long long)c;                // 550
    if (z == 0.0) rv += 1;                      // fcmp #0.0 + cinc(reg31)
    return rv;                                  // 551
}
"#,
    );
}

#[test]
fn diff_fp_nan_compare() {
    // IEEE: all NaN comparisons are false. `store_nzcv_fp` set Z=ZF, but x86
    // comisd sets ZF for unordered too, so `vnan==vnan` came out true (b.eq /
    // csel.eq fired). ARM unordered NZCV = N0 Z0 C1 V1 (Z=0). Z must be ZF&&!PF.
    assert_diff(
        "fp_nan",
        "-O2",
        r#"
long long entry(void){
    volatile double nn = 0.0/0.0;   // NaN at runtime
    long long acc = 0;
    if (nn == nn) acc += 1;         // must be FALSE
    if (nn > 0 || nn < 0) acc += 2; // false
    if (nn >= 0 || nn <= 0) acc += 4; // false (ge/le on NaN)
    if (nn != nn) acc += 8;         // TRUE (NaN != NaN)
    volatile double x = 3.0;
    if (x == x) acc += 16;          // true
    if (x > 2.0) acc += 32;         // true
    return acc;                      // 8 + 16 + 32 = 56
}
"#,
    );
}

#[test]
fn diff_cmn_unsigned_large_compare() {
    // gcc lowers `unsigned u > 0xffffffffffff0000` to `cmn x,#0x10000; b.ls`
    // (and the sibling hs/hi/lo forms). CMN/ADDS sets the ARM C flag to the
    // ADD's carry-out, but x86_cc_for_cond's HS/LO/HI/LS assume the stored C
    // is the SUBTRACT-borrow convention — pre-fix the borrow-sense polarity
    // was applied for SUBS only, so the CMN path left C as the carry and
    // `b.ls`/`b.hi` evaluated the unsigned bound backwards (a full-mask
    // unsigned shifted e.g. `> 0xffff...0000` to "not greater").
    assert_diff(
        "cmn_uconst",
        "-O2",
        r#"
long long entry(void){
    long long m = -1;
    unsigned long long um = (unsigned long long)m; // 0xffff...ffff
    long long acc = 0;
    if (um > 0xffffffffffff0000ULL) acc += 1000000;   // cmn #0x10000, b.ls skip: true
    if (um < 0xffffffffffff0001ULL) acc += 2000000;   // false
    if (um >= 0xffffffffffff0000ULL) acc += 4000000;  // true (cmn b.lo)
    if (um <= 0xffffffffffff0002ULL) acc += 8000000;  // false
    return acc;                                        // 1000000 + 4000000
}
"#,
    );
    assert_diff(
        "cmn_hi",
        "-O3",
        r#"
long long entry(void){
    unsigned long long um = 0x8000000000000001ULL;
    long long acc = 0;
    if (um > 0x8000000000000000ULL) acc += 10;         // true (cmn b.hi)
    if (um < 0x8000000000000002ULL) acc += 20;         // true (cmn b.lo)
    if (um == 0x8000000000000001ULL) acc += 40;
    return acc;                                         // 10 + 20 + 40 = 70
}
"#,
    );
}

#[test]
fn diff_addv_popcount_accumulate() {
    // gcc auto-vectorizes __builtin_popcountll into `cnt v.8b` + `addv b`
    // inside an accumulation loop. The ADDV-to-scalar result must CLEAR the
    // destination register's high bits: gcc then `fmov xD, dN` reads the whole
    // 64-bit register as the integer popcount. Leaving the per-lane cnt bytes
    // in v's upper bytes fed a polluting value into the accumulator — the
    // intermittent (stale-bytes / stack-layout dependent) SIMD-loop corruption
    // that hit the old maskf/times7/mod_pow2/regidx/mixed canaries. Now the
    // result is exact and deterministic.
    assert_diff(
        "addv_popcnt",
        "-O2",
        r#"
long long entry(void){
    unsigned long long pc = 0;
    for (int i = 0; i < 20; i++)
        pc += __builtin_popcountll((unsigned long long)i * 0x1111111111111111ULL);
    return (long long)pc;
}
"#,
    );
    assert_diff(
        "addv_popcnt3",
        "-O3",
        r#"
long long entry(void){
    unsigned long long pc = 0;
    for (int i = 0; i < 30; i++)
        pc += __builtin_popcountll((unsigned long long)(i*i) * 0x0101010101010101ULL);
    pc += __builtin_popcountll(0xffffffffffffffffULL);
    return (long long)pc;
}
"#,
    );
}

#[test]
fn diff_float_vector_arith() {
    // Elementwise vector fmla/fmul/fadd on .4s lanes + a final scalar sum.
    assert_diff(
        "fv_arith",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.13f; float seed = vseed;
    float a[16];
    for(int i=0;i<16;i++) a[i] = seed * (float)i;
    for(int i=0;i<16;i++) a[i] = a[i]*2.0f + 1.0f;  // fmul/fmla v.4s
    float s = 0.0f;
    for(int i=0;i<16;i++) s += a[i];
    return (long long)(s * 10.0f);
}
"#,
    );
    assert_diff(
        "fv_sub_neg",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = -0.77f; float seed = vseed;
    float a[16], b[16];
    for(int i=0;i<16;i++){ a[i] = seed*(float)(i*3); b[i] = (float)(i&1); }
    for(int i=0;i<16;i++) a[i] = (a[i] - b[i]) * 2.0f;   // fsub, fmul
    float s = 0.0f;
    for(int i=0;i<16;i++) s += a[i];
    return (long long)(s * 100.0f);
}
"#,
    );
}

#[test]
fn diff_float_vector_conv() {
    // Vector float->int (fcvtzs v.4s, trunc toward zero) and int->float
    // (scvtf v.4s), across positive and negative lanes.
    assert_diff(
        "fv_f2i",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 3.5f; float seed = vseed;
    float a[16]; int b[16];
    for(int i=0;i<16;i++) a[i] = seed * (float)i;
    for(int i=0;i<16;i++) b[i] = (int)a[i];          // fcvtzs v.4s (trunc)
    long long s = 0; for(int i=0;i<16;i++) s += b[i];
    return s;
}
"#,
    );
    assert_diff(
        "fv_f2i_neg",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.5f; float seed = vseed;
    float a[16]; int b[16];
    for(int i=0;i<16;i++) a[i] = seed * (float)(-7 + i*5);  // mixed sign
    for(int i=0;i<16;i++) b[i] = (int)a[i];
    long long s = 0; for(int i=0;i<16;i++) s += b[i];
    return s;
}
"#,
    );
    assert_diff(
        "fv_i2f",
        "-O3",
        r#"
long long entry(void){
    volatile int vk = 3; int k = vk;
    int a[16]; float b[16];
    for(int i=0;i<16;i++) a[i] = i*k - 7;
    for(int i=0;i<16;i++) b[i] = (float)a[i];        // scvtf v.4s
    long long s = 0; for(int i=0;i<16;i++) s += (long long)b[i];
    return s;
}
"#,
    );
}

#[test]
fn diff_float_vector_fmla_byelement() {
    // Vector-by-scalar FMLA (fmadd against a single broadcast lane) + float
    // compare/saturating count. Roblox's boot frontier literally stopped on
    // `fmla v29.4s, v19.4s, v26.4s`; the by-scalar (v.s[0]) form is separate.
    assert_diff(
        "fv_fmla_scalar",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.31f; float seed = vseed;
    float a[16];
    for(int i=0;i<16;i++) a[i] = seed + (float)i;
    float c = 1.5f;
    for(int i=0;i<16;i++) a[i] = a[i]*c + (float)(i%4);  // fmla v.4s by scalar
    float s = 0.0f;
    for(int i=0;i<16;i++) s += a[i];
    return (long long)(s * 100.0f);
}
"#,
    );
    assert_diff(
        "fv_cmp_count",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 0.7f; float seed = vseed;
    float a[16]; int c = 0;
    for(int i=0;i<16;i++) a[i] = seed * (float)i;
    for(int i=0;i<16;i++) if (a[i] > 5.0f) c++;   // fcmgt / csel / branch
    return c;
}
"#,
    );
}

#[test]
fn diff_integer_signed_division_negative_edge() {
    // gcc's INT_MIN/2 fast-path is `add w,w,w,lsr#31; asr w,w,#1` — the 32-bit
    // arithmetic shift. A JIT that reads bit63 (=0) as the sign of the
    // zero-extended 0x80000000 produced 0x40000000 (logical) not 0xc0000000.
    // The remainder body (`cmp; and w,#1; cneg w,,lt`) separately exercises the
    // CSEL-family op decode (csneg must NEGATE, and csinv must NOT) and the 32-
    // bit W zero-extension of the not/neg results.
    assert_diff(
        "idiv_edge",
        "-O2",
        r#"
unsigned long long entry(void){
    volatile int a = -2147483647-1;   // INT_MIN
    unsigned long long q = (unsigned long long)(a / 2);
    unsigned long long r = (unsigned long long)(a % 2);
    return q + r*1000000;
}
"#,
    );
    assert_diff(
        "idiv_edge3",
        "-O3",
        r#"
unsigned long long entry(void){
    volatile int a = -2147483647-1;
    volatile int b = 3000000;
    long long acc = 0;
    for (int i=0;i<3;i++) acc += (long long)(a/2) + (long long)(a%2) + b;
    return (unsigned long long)acc;
}
"#,
    );
}

#[test]
fn diff_movn_w32_zero_extension_and_not() {
    // A `return x & 0xffffffff` that folds to `movn w0,#2` (NOT of an imm into
    // a W register) must ZERO-extend the result to 64 bits: 0xfffffffd, not
    // 0xfffffffffffffffd. mov_guest_imm's imm32 short-cut (mov r32 sign-extends)
    // left the upper half set before this fix.
    assert_diff(
        "movn_mask",
        "-O2",
        r#"
unsigned long long entry(void){
    unsigned long long x = 0xffffffffffffffffULL;
    x = ~x;                       // 0
    unsigned long long lo = x & 0xffffffff;
    unsigned long long hi = (~0x2ULL) & 0xffffffff;  // 0xfffffffd
    return lo*1000000 + hi;
}
"#,
    );
    assert_diff(
        "not_neg_w",
        "-O3",
        r#"
unsigned long long entry(void){
    unsigned w = 5;
    w = ~w;                        // csinv W: 0xfffffffa
    int n = -5;
    int m = 5;
    n = -m;                        // csneg W
    unsigned long long acc = (unsigned long long)w * 1000000
        + (unsigned long long)(unsigned)n;   // ~5*1e6 + (-5 as u32)
    return acc;
}
"#,
    );
}

#[test]
fn diff_rotate_extract_and_byte_accum() {
    // Two more silent-miscompile classes:
    //  (a) `(x >> (64-n)) | (x << n)` compiles to the GENERAL EXTR
    //      (`extr x0,x0,x1,#imm`, rm != rn) which used to fall through to a
    //      UBFM/SBFM misdecode (bit-rotate produced garbage).
    //  (b) gcc -O3 vectorizes `unsigned char` accumulate loops into
    //      `uzp1 v.8h`/`add v.16b` with SELF-aliasing sources (rd==rn), which
    //      the permute translate corrupted by writing rd while still reading
    //      the aliased source's high half.
    assert_diff(
        "rot_extract",
        "-O2",
        r#"
unsigned long long entry(void){
    volatile unsigned long long x = 0x123456789abcdef0ULL;
    volatile int n = 13;
    return (x >> (64 - n)) | (x << n);
}
"#,
    );
    assert_diff(
        "rot_extract_chain",
        "-O3",
        r#"
unsigned long long entry(void){
    volatile unsigned long long x = 0x13579bdf2468aceaULL;
    volatile int n = 29;
    unsigned long long acc = 0;
    acc += (x >> (64 - n)) | (x << n);
    acc += (x >> n) | (x << (64 - n));
    return acc;
}
"#,
    );
    assert_diff(
        "uchar_accum",
        "-O3",
        r#"
unsigned long long entry(void){
    unsigned char t = 0;
    for (int i=0;i<300;i++) t = (unsigned char)(t + i);
    return t;
}
"#,
    );
    assert_diff(
        "uchar_accum_hybrid",
        "-O3",
        r#"
unsigned long long entry(void){
    unsigned char t = 0, s = 0;
    for (int i=0;i<320;i++){ t = (unsigned char)(t + i*7); s = (unsigned char)(s + i); }
    return (unsigned long long)(t*1000 + s);
}
"#,
    );
}

#[test]
fn diff_ld4_st4_matrix_transpose() {
    // ld4/st4 (opcode 0b0000 / 0b0100 structure DEINTERLEAVE) silently
    // miscompiled: the op field folded ld4 into the single-register
    // consecutive-load path — matmul C[i][j] returned 20 vs oracle 5248
    // (gcc loads matrices transposed via `ld4 {v24-v27}, [sp]`). Fixed by
    // giving ld4/st4/ld3/st3 true deinterleave: Vd[j][i] = mem[base+i*N*es+j*es].
    assert_diff(
        "ld4_matmul",
        "-O3",
        r#"
long long entry(void){
    float A[4][4],B[4][4],C[4][4];
    for(int i=0;i<4;i++)for(int j=0;j<4;j++){A[i][j]=(float)(i*4+j+1);B[i][j]=(float)(j*4+i+2);C[i][j]=0;}
    for(int i=0;i<4;i++)for(int k=0;k<4;k++)for(int j=0;j<4;j++) C[i][j]+=A[i][k]*B[k][j];
    float s=0; for(int i=0;i<4;i++)for(int j=0;j<4;j++) s+=C[i][j];
    return (long long)s;
}
"#,
    );
    assert_diff(
        "st4_transpose",
        "-O3",
        r#"
long long entry(void){
    float m[4][4], t[4][4];
    for(int i=0;i<4;i++)for(int j=0;j<4;j++) m[i][j]=(float)(i*2+j);
    for(int i=0;i<4;i++)for(int j=0;j<4;j++) t[j][i]=m[i][j];
    float s=0; for(int i=0;i<4;i++){for(int j=0;j<4;j++) s += t[i][j]*((i==j)?11.0f:(j==i+1?7.0f:1.0f));}
    return (long long)(s*2);
}
"#,
    );
}

#[test]
fn diff_fmls_vector_subtract_accumulate() {
    // Vector fmls (Vd = Vd - Vn*Vm) inverted its operands: the Fmla translate
    // computed Vn*Vm - Vd (only correct because add is commutative), so every
    // accumulate-subtract fmls came out with the wrong sign but same magnitude —
    // a 4x4 complex matmul (heavy fmls) returned 10688 vs oracle 13504, and an
    // acc - broadcast*acc loop returned 20 (positive) vs oracle -100.
    // gcc -O3 emits `fmls v30.4s, v1.4s, v2.4s` (rd==rm accumulator).
    assert_diff(
        "fmls_accum_sub",
        "-O3",
        r#"
long long entry(void){
    double acc[2]={0,0}; float fm[2]={1.5f,2.5f};
    for(int it=0;it<4;it++){ for(int i=0;i<2;i++) acc[i]=acc[i]-fm[it%2]*fm[(it+1)%2]; }
    long long s=(long long)acc[0]+(long long)acc[1];
    return s;
}
"#,
    );
    assert_diff(
        "fmls_cmat4",
        "-O3",
        r#"
long long entry(void){
    float ar[4][4],ai[4][4],br[4][4],bi[4][4],cr[4][4];
    for(int i=0;i<4;i++)for(int j=0;j<4;j++){ar[i][j]=(float)(i*3+j); ai[i][j]=(float)(i+j); br[i][j]=(float)(i*2+j+1); bi[i][j]=(float)(i*4+j-1); cr[i][j]=0;}
    for(int i=0;i<4;i++)for(int k=0;k<4;k++)for(int j=0;j<4;j++) cr[i][j]+=ar[i][k]*br[k][j]-ai[i][k]*bi[k][j];
    float s=0; for(int i=0;i<4;i++)for(int j=0;j<4;j++) s+=cr[i][j];
    return (long long)(s*2);
}
"#,
    );
}

#[test]
fn diff_dup_from_gpr_matrix_init() {
    // `dup Vd.T, Wn` (GPR broadcast) was swallowed by the SIMD square/diff
    // saturating-add gate (both byte2==0x0c), so gcc's -O2 matrix fill loops
    // (`dup v30.4s, w1; add v30,v30,v31; scvtf; str q30,[x],#16`) decoded the
    // dup as sqadd(v1,v4): the array held sat-add garbage and init returned a
    // huge value (init_O2: 18446744039484557312 vs 96). The discriminator is
    // bit21 (sat-add sets it, dup-from-GPR clears it).
    assert_diff(
        "dup_gpr_2d_fill",
        "-O2",
        r#"
long long entry(void){
    float a[4][4];
    for(int i=0;i<4;i++) for(int j=0;j<4;j++) a[i][j]=(float)(i*3+j);
    float s=0; for(int i=0;i<4;i++) for(int j=0;j<4;j++) s+=a[i][j];
    return (long long)s;
}
"#,
    );
    assert_diff(
        "dup_gpr_linear_fill",
        "-O2",
        r#"
long long entry(void){
    float a[16];
    for(int i=0;i<16;i++) a[i]=(float)(i*7-3);
    float s=0; for(int i=0;i<16;i++) s+=a[i];
    return (long long)(s*3);
}
"#,
    );
    assert_diff(
        "mix_hash_ubfiz",
        "-O2",
        r#"
unsigned long long entry(void){
    unsigned long long h=0xdeadbeefcafebabeULL;
    unsigned char msg[16]; for(int i=0;i<16;i++) msg[i]=(unsigned char)(i*31+h);
    for(int i=0;i<16;i++){ h ^= (unsigned long long)msg[i] << ((i%8)*8); h = h*31ULL + 17; }
    return h;
}
"#,
    );
    assert_diff(
        "fcvtzs_fixed_scale",
        "-O3",
        r#"
long long entry(void){
    double a[4]={3.5, -1.25, 9.75, 0.5};
    double s=0; for(int i=0;i<4;i++) s+=a[i]*a[i];
    return (long long)(s*4);
}
"#,
    );
    assert_diff(
        "fcvtzs_fixed_scale_neg",
        "-O2",
        r#"
long long entry(void){
    float v = -2.75f;
    long long r = (long long)(v * 8.0f);    // fcvtzs s,#3
    long long r2 = (long long)(v * 32.0f);  // fcvtzs s,#5
    return r*1000 + r2;
}
"#,
    );
    assert_diff(
        "fma_ld1_postidx",
        "-O2",
        r#"
long long entry(void){
    const double a[4]={1.5,2.5,3.5,4.5};
    const double b[4]={2.0,3.0,4.0,5.0};
    double acc=0; for(int i=0;i<4;i++) acc += a[i]*b[i];
    return (long long)(acc*10);
}
"#,
    );
}

#[test]
fn diff_double_vector_arith() {
    // The .2d double-lane vector family (fmla v.2d / fmul v.2d / fcvtzs v.2d),
    // the double sibling of the .4s group above.
    assert_diff(
        "dv_arith",
        "-O3",
        r#"
long long entry(void){
    volatile double vseed = 0.91; double seed = vseed;
    double a[16];
    for(int i=0;i<16;i++) a[i] = seed * (double)(i*i);
    for(int i=0;i<16;i++) a[i] = a[i]*1.5 + (double)(i & 1);  // may fmla v.2d
    double s = 0.0;
    for(int i=0;i<16;i++) s += a[i];
    return (long long)(s * 10.0);
}
"#,
    );
}

#[test]
fn diff_float_vector_reduced() {
    // Narrower probes on the still-failing fv_arith shape, to localize the
    // (small-error) miscompile to either the by-element fmul, the fmla, or the
    // scalar reduction. n=4 keeps it to ONE vector group + one fmla.
    assert_diff(
        "fv4",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.13f; float seed = vseed;
    float a[4];
    for(int i=0;i<4;i++) a[i] = seed * (float)i;
    for(int i=0;i<4;i++) a[i] = a[i]*2.0f + 1.0f;
    float s = 0.0f;
    for(int i=0;i<4;i++) s += a[i];
    return (long long)(s * 10.0f);
}
"#,
    );
    assert_diff(
        "fv4_fmul_only",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.13f; float seed = vseed;
    float a[4];
    for(int i=0;i<4;i++) a[i] = seed * (float)i;
    float s = 0.0f;
    for(int i=0;i<4;i++) s += a[i];
    return (long long)(s * 10.0f);
}
"#,
    );
}

#[test]
fn diff_scalar_fp_round_single() {
    // Scalar 1-source FP rounding on SINGLE (frintm/frintp/frintz s) and the
    // fsqrt-s sibling. Two silent miscompiles lived in the FpUnary single
    // path:
    //  (a) the loaded float bits (RAX) were never moved into xmm0 before
    //      `cvtss2sd`, so EVERY single frint/fsqrt read stale xmm0 — a floor
    //      loop returned 450 vs 360 (and would return garbage generically).
    //  (b) `frintz s` (toward zero = truncate) used roundsd mode 0b00 which is
    //      round-to-NEAREST, not truncate — the negative-heavy trunc probe
    //      returned 20 vs native 40. The double path already used 0b11.
    // A floor probe (positive) caught (a); a trunc probe with negatives is the
    // sharp canary for (b).
    assert_diff(
        "fr_floor_single",
        "-O3",
        r#"
long long entry(void){
    volatile float a[8] = {1.4f,2.6f,3.5f,4.1f,5.9f,6.2f,7.8f,8.3f};
    float s=0; for(int i=0;i<8;i++) s += __builtin_floorf(a[i]);
    return (long long)(s*10.0f);
}
"#,
    );
    assert_diff(
        "fr_ceil_single",
        "-O3",
        r#"
long long entry(void){
    volatile float a[8] = {1.4f,2.6f,3.5f,4.1f,5.9f,6.2f,7.8f,8.3f};
    float s=0; for(int i=0;i<8;i++) s += __builtin_ceilf(a[i]);
    return (long long)(s*10.0f);
}
"#,
    );
    assert_diff(
        "fr_trunc_single_negatives",
        "-O3",
        r#"
long long entry(void){
    volatile float a[8] = {-1.4f,2.6f,-3.5f,4.1f,-5.9f,6.2f,-7.8f,8.3f};
    float s=0; for(int i=0;i<8;i++) s += __builtin_truncf(a[i]);
    return (long long)(s*10.0f);
}
"#,
    );
}

#[test]
fn diff_vector_frint_rounding() {
    // VECTOR frint{v,m,p,z} Vd.T, Vn.T (32-bit lanes). These used to decode as
    // SimdMull (smull/umull widen-multiply) because the coarse 0x0f00_c000 gate
    // swallows byte2 0x98/0x88 into the smlal residue — a floor/ceil loop was a
    // silent widening-multiply (garbage), not an honest stop (a floor loop
    // returned 1.9e16 vs 590). Fixed by giving frint its own decode BEFORE
    // SimdMull + tightening SimdMull to exact byte2 0x80/0xc0.
    assert_diff(
        "vflr_floor",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 2.2f; float seed = vseed;
    float a[8]; for(int i=0;i<8;i++) a[i] = __builtin_floorf(seed*(float)i);
    float s=0; for(int i=0;i<8;i++) s += a[i];
    return (long long)(s*10);
}
"#,
    );
    assert_diff(
        "vflr_ceil",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = 1.3f; float seed = vseed;
    float a[8]; for(int i=0;i<8;i++) a[i] = __builtin_ceilf(seed*(float)i + 0.2f);
    float s=0; for(int i=0;i<8;i++) s += a[i];
    return (long long)(s*10);
}
"#,
    );
}

#[test]
fn diff_vector_fp_compare_zero() {
    // Vector FP compare-to-zero (fcmlt/fcmgt/fcmge/fcmge/fcmle Vd, Vn, #0.0)
    // driving a bitwise select (bsl). Used to decode as SimdMull too (byte2
    // 0xea -> smlal residue), silently selecting the wrong branch. fnint: the
    // select must pick the negative (fcmlt) / positive (fcmgt) lanes.
    assert_diff(
        "vcmp0_lt_keep_neg",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = -1.3f; float seed = vseed;
    float a[8]; for(int i=0;i<8;i++) a[i] = (seed*(float)i < 0 ? seed*(float)i : -(seed*(float)i));
    float s=0; for(int i=0;i<8;i++) s += a[i];
    return (long long)(s*10);
}
"#,
    );
    assert_diff(
        "vcmp0_gt_keep_pos",
        "-O3",
        r#"
long long entry(void){
    volatile float vseed = -2.7f; float seed = vseed;
    float a[8]; for(int i=0;i<8;i++) a[i] = (seed*(float)i > 0 ? seed*(float)i : 0.0f);
    float s=0; for(int i=0;i<8;i++) s += a[i];
    return (long long)(s*10);
}
"#,
    );
}
#[test]
fn diff_minmax_reduction_float() {
    // SIMD single-precision min/max reductions (fminnm/fmaxnm v.4s or the
    // scalar fmin/fmax path gcc -O3 emits for a running min/max over an array
    // pair). A 3D engine computes AABBs / camera bounds / skeleton extents
    // with exactly this shape; a wrong min/max lane or a mis-decoded fcmlt/
    // fsel silently flips geometry bounds.
    assert_diff(
        "mm_pair_minmax",
        "-O3",
        r#"
long long entry(void){
    volatile float s0=0.4f; float sc=s0;
    float a[8],b[8];
    for(int i=0;i<8;i++){ a[i]=(float)(i+1)*sc; b[i]=(float)(8-i)*sc; }
    float mn=0,mx=0;
    for(int i=0;i<8;i++){
        float m = a[i]<b[i]?a[i]:b[i];
        float M = a[i]>b[i]?a[i]:b[i];
        mn = mn<m?mn:m;
        mx = mx>M?mx:M;
    }
    return (long long)((mn+mx)*100);
}
"#,
    );
    assert_diff(
        "mm_single_running_min",
        "-O3",
        r#"
long long entry(void){
    volatile float s0=3.0f; float sc=s0;
    volatile float a0=0.25f; float a=a0;
    float mn=1e9f,mx=-1e9f;
    for(int i=0;i<8;i++){
        float v = a + (float)i*sc*0.01f;
        mn = mn<v?mn:v;
        mx = mx>v?mx:v;
    }
    return (long long)((mx-mn)*100000);
}
"#,
    );
}

#[test]
fn diff_reciprocal_and_div_float() {
    // Reciprocal / division funnel (audio normalization, colour-space scale):
    // sum of 1/(i+3), filtered through a range test. Exercises scalar fdiv s,
    // fadd s, fcvtzs and a FCMP-guarded branch in one function.
    assert_diff(
        "g_recip_sum",
        "-O3",
        r#"
long long entry(void){
    float a[8]; for(int i=0;i<8;i++) a[i]=1.0f/(float)(i+3);
    float s=0; for(int i=0;i<8;i++) s+=a[i];
    return (long long)(s*1000.0f);
}
"#,
    );
    assert_diff(
        "g_div_range_filter",
        "-O3",
        r#"
long long entry(void){
    float a[8]; for(int i=0;i<8;i++) a[i]=48.0f/(float)(i+4);
    float s=0; for(int i=0;i<8;i++){ if(a[i]>5.0f && a[i]<9.0f) s+=a[i]; }
    return (long long)(s*10.0f);
}
"#,
    );
}

#[test]
fn diff_widening_mul_accumulate_int() {
    // smaddl/umaddl / smull widening integer multiply-accumulate (position/
    // index math, hash tables). A wrong MADD acc/dir or a missing zero-extend
    // silently corrupts the accumulator.
    assert_diff(
        "w_smaddl_pair_product",
        "-O3",
        r#"
long long entry(void){
    int a[9]; for(int i=0;i<9;i++) a[i]=(i+1);
    long long acc=0;
    for(int i=0;i<9;i++) acc += (long long)a[i]*(long long)a[8-i];
    return acc;
}
"#,
    );
    assert_diff(
        "w_umaddl_unsigned",
        "-O3",
        r#"
long long entry(void){
    unsigned int a[7]; for(int i=0;i<7;i++) a[i]=(unsigned int)(i+1)*0x10000u;
    unsigned long long acc=0;
    for(int i=0;i<7;i++) acc += (unsigned long long)a[i]*(unsigned long long)i;
    return (long long)acc;
}
"#,
    );
}

#[test]
fn diff_double_fp_loop_condition() {
    // Double FP with an array built from division and a FP loop-condition
    // guard (lighting / physics accumulation). Exercises double FMADD,
    // FCMP->B.cond, and a mixed int+double reduction.
    assert_diff(
        "l_double_div_accum_guard",
        "-O3",
        r#"
long long entry(void){
    volatile double t0=2.0; double t=t0;
    double a[6];
    for(int i=0;i<6;i++) a[i]= (double)(i+1)*t/(double)(i+2);
    double s=0; int n=0;
    for(int i=0;i<6;i++){ if(a[i]<3.0){ s+=a[i]; n++; } }
    return (long long)((s*1000.0)+(double)n);
}
"#,
    );
}

#[test]
fn diff_scalar_reduction_minmax_float() {
    // Running single-precision min/max reduction over an array (bounding-box /
    // channel-energy pattern). gcc -O3 typically emits scalar fcmp+fsel pairs or
    // vector fminnm/fmaxnm; either way the JIT must track the true extents.
    assert_diff(
        "rm_running_extents",
        "-O3",
        r#"
long long entry(void){
    volatile float s0=3.1f; float s=s0;
    float a[8]; for(int i=0;i<8;i++) a[i]= (float)i*s - (float)(i%3)*1.7f;
    float mn=1e9f,mx=-1e9f;
    for(int i=0;i<8;i++){ mn = mn<a[i]?mn:a[i]; mx = mx>a[i]?mx:a[i]; }
    return (long long)((mx-mn)*1000);
}
"#,
    );
}

#[test]
fn diff_int_simd_minmax_reduction() {
    // Signed integer SIMD min/max (smin/smax v.4s/v.2d) + running acc — a
    // particle / collision / alpha-culling pattern. Not previously a canary.
    assert_diff(
        "im_running_minmax",
        "-O3",
        r#"
long long entry(void){
    volatile int s0=7; int a[8];
    for(int i=0;i<8;i++) a[i]= (i*i-5*(i%2))*s0;
    int mn=1<<30, mx=-(1<<30), acc=0;
    for(int i=0;i<8;i++){ if(a[i]<mn) mn=a[i]; if(a[i]>mx) mx=a[i]; acc+=a[i]; }
    return (long long)(mn)*1000 + (long long)mx*100 + (long long)acc;
}
"#,
    );
}

#[test]
fn diff_math128_and_carry() {
    // __int128 multiply/square (umulh/smulh high-half + madd chain) and
    // multiword add-with-carry (adc/adcs) — bignum / large-integer math that
    // appears in crypto / session signing / price arithmetic. A wrong high-half
    // product or a dropped carry silently corrupts the upper 64 bits.
    assert_diff(
        "math128",
        "-O2",
        r#"
typedef __int128 i128;
static i128 sq(i128 x){ return x*x; }
static i128 mul(i128 a,i128 b){ return a*b; }
static i128 madd(i128 a,i128 b,i128 c){ return a*b + c; }
unsigned long long entry(void){
    volatile unsigned long long lo=0x123456789abcdef0ULL, one=1ULL;
    i128 a = ((i128)lo << 64) | 0x0fedcba987654321ULL;
    i128 b = sq(a) >> 64;                 // high 64 of a*a
    i128 c = mul(a, 0xdeadbeefULL) >> 20;
    i128 d = madd(a, 0x100000001ULL, 7);
    unsigned long long acc = (unsigned long long)b;
    acc ^= (unsigned long long)(c >> 32);
    acc ^= (unsigned long long)(d >> 63);
    unsigned long long h=0, t=0;
    // multiword add: h:t + carry chain
    { unsigned long long x0=0xffffffffffffffffULL, x1=1ULL;
      unsigned long long s0 = x0 + x1; int carry = s0 < x0;
      unsigned long long s1 = x0 + carry;  // adc
      h = s1; t = s0; }
    acc ^= h*3 + t;
    return acc;
}
"#,
    );
    assert_diff(
        "math128_const",
        "-O3",
        r#"
typedef __int128 i128;
i128 entry128(void){ return ((i128)0x1122334455667788ULL * 0x99aabbccddeeffULL); }
unsigned long long entry(void){
    i128 v = entry128();
    return (unsigned long long)v ^ (unsigned long long)(v >> 64);
}
"#,
    );
    assert_diff(
        "math128_addsub",
        "-O2",
        // Forces REAL runtime carries through adds+adc and subs+sbc (the values
        // are volatile so gcc cannot constant-fold the carry chains), locking
        // the ADC/SBC borrow-convention-carry fix on its exact s/ns paths.
        r#"
typedef __int128 i128;
volatile unsigned long long A=0xfffffffffffffff0ULL, B=0x123456789abcdef0ULL;
volatile unsigned long long Cc=0x0000000000000005ULL, D=0x7fffffffffffffffULL;
unsigned long long entry(void){
    i128 x = ((i128)A << 64) | B;
    i128 y = ((i128)Cc << 64) | D;
    i128 s = x + y;              // adds + adc (carry into high word)
    i128 d = x - y;              // subs + sbc (borrow chain)
    unsigned long long lo = (unsigned long long)(s ^ d);
    unsigned long long hi = (unsigned long long)((s ^ d) >> 64);
    return lo ^ (hi << 1);
}
"#,
    );
}

#[test]
fn diff_switch_fnptr_hash() {
    // Robin-Hood switch lowering (jump table via adrp+ldr+br) + indirect
    // function-pointer dispatch (blr through a table) + an FNV-style hash chain
    // (XOR/mul/rotate) — control-flow + hashing the game core uses constantly.
    assert_diff(
        "switch_jump",
        "-O2",
        r#"
static int hot(int x){
    switch(x){
        case 0: return 11;
        case 1: return 101;
        case 3: return 1001;
        case 5: return 10001;
        case 9: return 100001;
        case 13:return 1000001;
        case 42:return 10000001;
        default: return 7;
    }
}
static int hot3(int x){
    switch(x){
        case 100: return 1;
        case 200: return 2;
        case 300: return 4;
        case 400: return 8;
        case 500: return 16;
        case 600: return 32;
        case 700: return 64;
        default: return 0;
    }
}
unsigned long long entry(void){
    unsigned long long acc = 0;
    int a[10] = {0,1,3,5,9,13,42,7,2,4};
    for(int i=0;i<10;i++) acc += hot(a[i]);
    acc ^= hot3(600);
    acc ^= hot3(999);
    return acc;
}
"#,
    );
    assert_diff(
        "fnptr_callback",
        "-O3",
        r#"
typedef unsigned long long (*fnt)(unsigned long long);
static unsigned long long add2(unsigned long long x){ return x+2; }
static unsigned long long mul3(unsigned long long x){ return x*3; }
static unsigned long long xorf(unsigned long long x){ return x ^ 0xdeadbeefULL; }
static fnt tbl[4] = {add2, mul3, xorf, add2};
unsigned long long entry(void){
    unsigned long long acc = 1;
    for(int i=0;i<8;i++) acc = tbl[acc & 3](acc);
    return acc;
}
"#,
    );
    assert_diff(
        "fnv_hash",
        "-O2",
        r#"
unsigned long long entry(void){
    const unsigned char msg[32] = "open-sober robux proto hash";
    unsigned long long h = 0xcbf29ce484222325ULL;
    for(int i=0;i<32 && msg[i];i++){ h ^= msg[i]; h = h*0x100000001b3ULL; }
    h ^= h >> 33; h *= 0xff51afd7ed558ccdULL; h ^= h >> 33;
    return h;
}
"#,
    );
}

#[test]
fn diff_halfword_minmax_negatives() {
    // Signed SHORT/CHAR arrays with negatives fed into min/max/sum reductions —
    // exercises the 16/8-bit sign-extended load pipeline (movsx REX.W fix) and
    // the across-lanes reduce in a real compiled context.
    assert_diff(
        "hw_short_reduce",
        "-O3",
        r#"
long long entry(void){
    volatile short seed=7; short a[8];
    long long s=0;
    for(int i=0;i<8;i++){ a[i]=(short)((i*i-5*(i%2))*seed); s+=a[i]; }
    short mn=32767, mx=-32768;
    for(int i=0;i<8;i++){ if(a[i]<mn) mn=a[i]; if(a[i]>mx) mx=a[i]; }
    return (long long)mn*10000 + (long long)mx*100 + s;
}
"#,
    );
    assert_diff(
        "hw_schar_reduce",
        "-O2",
        r#"
long long entry(void){
    volatile signed char seed=5; signed char a[8];
    long long s=0;
    for(int i=0;i<8;i++){ a[i]=(signed char)((i*i-4*i)*seed); s+=a[i]; }
    signed char mn=127, mx=-128;
    for(int i=0;i<8;i++){ if(a[i]<mn) mn=a[i]; if(a[i]>mx) mx=a[i]; }
    return (long long)mn*10000 + (long long)mx*100 + s;
}
"#,
    );
}


#[test]
fn diff_fcvtl_widen_and_fcvtn_narrow() {
    // Guard for SIMD float<->double widening/narrowing: gcc -O2 emits
    // fcvtl/fcvtl2 (float->double, low & upper source half) and fcvtn/fcvtn2
    // (double->float, low & upper dest half) when a program round-trips
    // float[] <-> double[] elementwise. Distinct per-lane values + upper-half
    // use catch any fcvtl/fcvtn lane-offset or width slip (the JIT's fcvtl/
    // fcvtn gates land Unsupported without the feature; a decode collision =
    // silent wrong values). Also trips the latent pre/post-index scalar
    // single-precision store bug (str s30,[x4],#4 -> stored to the float's
    // bit pattern), since widened/narrowed lanes are written back with
    // post-indexed scalar stores.
    assert_diff(
        "fcvtl_fcvtn",
        "-O2",
        r#"
long long entry(void){
    long long seed = 3847481;
    float f[16]; double d[16];
    for(int i=0;i<16;i++) f[i] = (float)((seed*(i+3))%97) * 0.5f;
    for(int i=0;i<16;i++) d[i] = (double)f[i];       // fcvtl / fcvtl2
    for(int i=0;i<16;i++) f[i] = (float)d[i];        // fcvtn / fcvtn2
    long long acc = 0;
    for(int i=0;i<16;i++) acc += (long long)(f[i]*4.0f);
    return acc;
}
"#,
    );
}


#[test]
fn diff_int64_to_int32_narrowing_bif() {
    // gcc -O2 int64->int32 truncation (with large negative values that need
    // sign handling) compiles through smull/saddl/saddw + cmeq + BIT/BIF +
    // uzp. Guards the bit-vs-bif decode fix: BIF was silently decoded as BSL
    // (the opposite mask select) because the SimdSel gate ignored bit23, and
    // the narrowing pipeline's bif produced wrong lanes. jit==oracle.
    assert_diff(
        "narrow_bif",
        "-O2",
        r#"
long long entry(void){
    long long i64[16]; int i32[16];
    for(int k=0;k<16;k++) i64[k] = (long long)(k*k*7 + (k%3==0?-3000000000LL:0));
    for(int k=0;k<16;k++) i32[k] = (int)i64[k];      // int64->int32 truncate
    long long acc = 0;
    for(int k=0;k<16;k++) acc += (long long)i32[k];
    return acc;
}
"#,
    );
    // The full round-trip (int64 -> int32 -> short) also uses bsl/bit and the
    // post-indexed single store path.
    assert_diff(
        "narrow_bif_full",
        "-O2",
        r#"
long long entry(void){
    long long i64[16]; int i32[16]; short i16[16];
    for(int k=0;k<16;k++) i64[k] = (long long)(k*k*7 + (k%3==0?-3000000000LL:0));
    for(int k=0;k<16;k++) i32[k] = (int)i64[k];
    for(int k=0;k<16;k++) i16[k] = (short)i32[k];
    long long acc = 0;
    for(int k=0;k<16;k++) acc += (long long)i16[k]*1000 + i32[k];
    return acc;
}
"#,
    );
}


#[test]
fn diff_byte_negcount_ssubw_cmlt() {
    // gcc -O3 vectorizes `if(sc[i]<0) count++` into cmlt -> sxtl -> ssubw (the
    // count is accumulated by subtracting the sign-extended cmlt mask: -(-1)=+1
    // per negative). Guards (a) SIMD integer compare-to-zero (cmlt Vd,Vn,#0)
    // and (b) the sub-wide (ssubw/usubw) family, which the SimdAddw gate
    // silently decoded as ADD. Also exercises uaddw/uxtl byte-array widening.
    assert_diff(
        "byte_negcount",
        "-O3",
        r#"
long long entry(void){
    unsigned char c[32]; signed char sc[32];
    volatile int seed=13;
    for(int i=0;i<32;i++){ c[i]=(unsigned char)((i*seed+3)&0xff); sc[i]=(signed char)((i*seed*3-50)&0xff); }
    long long acc=0; long long nm=0;
    for(int i=0;i<32;i++){ acc += (long long)c[i]; if(sc[i]<0) nm++; }
    unsigned short u16[16];
    for(int i=0;i<16;i++) u16[i]=(unsigned short)((i*seed*7+11)&0xffff);
    unsigned long long s2=0;
    for(int i=0;i<16;i++) s2 += (unsigned long long)u16[i]*3;
    return acc + nm + (long long)s2;
}
"#,
    );
}

#[test]
fn diff_ld2_halfword_strided_accumulate() {
    // gcc -O3 vectorizes a u16 STRIDED accumulate (`for(i+=2) s += b[i]`) into
    // `ld2 {v.8h, v.8h}` (structure DEINTERLEAVE at 2-byte element granularity)
    // + zip/uaddw. Root cause of an open silent miscompile: the ld2 translate
    // arm deinterleaved AT BYTE GRANULARITY regardless of element size, so the
    // 2-byte strided even-index load read mem[2i], mem[2i+1] instead of the
    // correct mem[4i], mem[4i+2] - the even-index sum registered the wrong
    // memory elements (isolated repro: 311814 vs native 281606; and it
    // contaminated a co-resident byte-acc loop to ~2x). Element size must
    // scale the stride. Guards the ld2/st2 halfword path end-to-end.
    assert_diff(
        "ld2h_strided_even",
        "-O3",
        r#"
long long entry(void){
    unsigned short b[64];
    volatile unsigned long long seedv = 0x9e3779b97f4a7c15ull;
    unsigned long long x = seedv*1103515245u + 12345u;
    for(int i=0;i<64;i++){ x = x*1103515245u + 12345u; b[i]=(unsigned short)((x>>16)^0x33); }
    long long s1=0, s2=0;
    for(int i=0;i<64;i+=1) s1 += (long long)b[i];
    for(int i=0;i<64;i+=2) s2 += (long long)b[i];
    return s1 + s2*1000;
}
"#,
    );
}

#[test]
fn diff_wform_bitfield_masks_high_bits_before_shift() {
    // Silent miscompile: a W-form (32-bit) logical bitfield op — here the
    // `lsr w?, w1, #24` byte-extract gcc emits `for(...) b[i]=(x>>24)&0xff` —
    // must discard the UPPER 32 bits of the 64-bit source register BEFORE the
    // shift. The source (x1) holds a 64-bit LCG product from `madd x1,x1,x4,x3`,
    // so a full-register `shr` pulled guest high bits 32-55 down into the
    // extracted byte (real repro: 8652 -> 0x37562e2cc). Small array (N=8) forces
    // gcc to the scalar madd+lsr-w chain (larger arrays vectorize to uaddw and
    // do not trip this path). Root cause: Inst::BitField loaded Rn as u64 and
    // shifted without `zero_ext_r32` first. Regression guard.
    assert_diff(
        "wform_lsrw_after_madd",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 123456789ull;
    unsigned long long x = seedv;
    unsigned char b[8];
    for(int i=0;i<8;i++){ x = x*1103515245ull + 12345ull; b[i]=(unsigned char)((x>>24)&0xff); }
    long long s1=0, s2=0;
    for(int i=0;i<8;i+=1) s1 += (long long)b[i];
    for(int i=0;i<8;i+=2) s2 += (long long)b[i];
    return s1*7 + s2;
}
"#,
    );
}

#[test]
fn diff_shrn_shift_right_narrow_lanes() {
    // Silent miscompile: shrn/shrn2 (shift-right-NARROW, bit15=0x8000 set) was
    // decoded as a plain equal-size ushr/sshr, so the source lanes were read at
    // HALF stride (32-bit instead of the double-width 64-bit) and the shift
    // amount was wrong. gcc -O3 compiles `x ^ (x>>16)` (u64 LCG then truncate
    // to u32 per lane) into shrn/shrn2 + uzp1 + eor + uxtl; before the fix the
    // JIT returned 0x83f9b82e6 instead of the oracle 0x91f998726. Guards the
    // whole transform. Root cause: missing SimdShrn decode+translate.
    assert_diff(
        "shrn_xor_shift_lanes",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 987654321ull;
    unsigned long long x = seedv;
    unsigned int arr[16];
    for(int i=0;i<16;i++){ x = x*1664525ull + 1013904223ull; arr[i] = (unsigned int)(x ^ (x>>16)); }
    long long s=0;
    for(int i=0;i<16;i++) s += (long long)((unsigned int)(arr[i] * 2654435761u));
    return s;
}
"#,
    );
}

#[test]
fn diff_simd_shl_immediate_large_shift() {
    // Silent miscompile: `shl Vd.4S, Vn.4S, #imm` decoded the shift amount as
    // just immb (bits 18:16) and esize from immh.trailing_zeros (dropping bit22)
    // — so `shl v29.4s,#25` ran as a #1 shift on 1-byte lanes. gcc -O3 compiles
    // `(unsigned int)(x<<25)` (u64 LCG then truncate per 32-bit lane) into
    // shl.4s + uaddw; before the fix JIT returned 36832764464 vs oracle
    // 35165044736. Guards the shift-left-immediate path: shift must be
    // UInt(immh4:immb) - esize_bits.
    assert_diff(
        "shl32_immed_large",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 123456789ull;
    unsigned long long x=seedv; unsigned int a[16];
    for(int i=0;i<16;i++){x=x*1103515245ull+12345ull; a[i]=(unsigned int)(x<<25);}
    long long s=0; for(int i=0;i<16;i++) s+=a[i]; return s;
}
"#,
    );
}

#[test]
fn diff_ubfx_masks_upper_bits_and_is_not_ror() {
    // Two silent-miscompile guards for the Inst::BitField general-extract path:
    // (a) a UBFM extract whose width makes the mask >= 2^31 (here ubfx x,#16,#32,
    //     mask 0xffffffff) must NOT be emitted via `and_ri64(m as u32)`, which
    //     SIGN-EXTENDS the imm32 to 0xffffffffffffffff (a no-op) leaving the high
    //     32 bits of the shifted value corrupting the result.
    // (b) such a UBFM (immr<=imms, imms+immr+1==bits) must NOT be mis-decoded as
    //     a rotate: genuine `ror` is an EXTR alias, so a UBFM that satisfies
    //     immr+imms+1==bits is a plain extract, not a rotation. Before both
    //     fixes `(x>>16)` computed via ubfx returned garbage.
    assert_diff(
        "ubfx_hi_extract_mask",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv=424242ull;
    unsigned long long x=seedv; unsigned int v[4];
    for(int i=0;i<4;i++){ x=x*1103515245ull+12345ull; v[i]=(unsigned int)(x>>16); }
    long long t=0; for(int i=0;i<4;i++) t=t*31+(long long)v[i]; return t;
}
"#,
    );
    // Double-guard with genuine rotate + high extracts in the same function,
    // forcing gcc to emit both ror (EXTR) and ubfx so neither regresses.
    assert_diff(
        "ubfx_and_genuine_ror",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv=777888999ull;
    unsigned long long x=seedv*1103515245ull+12345ull;
    unsigned long long a=(x>>17)|(x<<47);        // ror x,#17 (EXTR)
    unsigned long long b=(x >> 21);              // ubfx x,#21,#43
    unsigned long long c=(x >> 45);              // ubfx x,#45,#19
    return (long long)((a^(b<<2))+(c*31));
}
"#,
    );
}

#[test]
fn diff_scalar_ucvtf_s_lane_from_volatile_u32() {
    // regression: scalar ucvtf S-form (in-place int->float of a loaded raw
    // 32-bit value) was decoded with sng=true but the translate arm ignored it
    // and did a 64-bit D-form convert, losing the value. The `volatile u32`
    // + `(float)s` is what gcc compiles to `ldr sX,[sp]` + `ucvtf sX,sX`.
    assert_diff(
        "scalar_ucvtf_s_from_volatile_u32",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned int s = 7;
    float a0=100.0f,a1=200.0f,a2=300.0f,a3=50.0f,a4=80.0f,a5=120.0f,a6=250.0f,a7=10.0f;
    float acc=0.0f;
    acc += a0/3.0f + (float)s;
    acc += a1/7.0f + (float)s;
    acc += a2/11.0f + (float)s;
    acc += a3/2.0f + (float)s;
    acc += a4/5.0f + (float)s;
    acc += a5/8.0f + (float)s;
    acc += a6/13.0f + (float)s;
    acc += a7/4.0f + (float)s;
    return (long long)(acc*1000.0f);
}
"#,
    );
}

// RESOLVED: the MOVI Vd.2D, #imm immediate was mis-decoded (byte-select #0xffff
// became 0xff), giving a wrong AND-extract mask that silently collapsed every
// vector reduction (fmla/div pipeline: acc += a[i]*b[i]+c[i]). Regression guard;
// also reproduces via fuzz_jit gen_fma_chain.
#[test]
fn diff_fmla_div_pipeline_lcg() {
    assert_diff(
        "fmla_div_pipeline_lcg",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 314159ull;
    unsigned long long x = seedv;
    float a[16], b[16], c[16];
    for(int i=0;i<16;i++){ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(float)(((x>>40)&0xffff)/1024.0); b[i]=(float)(((x>>24)&0xffff)/2048.0); c[i]=(float)(((x>>8)&0xffff)/4096.0); }
    float acc=0.0;
    for(int i=0;i<16;i++){ acc+=a[i]*b[i]+c[i]; }
    return (long long)(acc*1e3);
}
"#,
    );
}

// Integer vector NEG/ABS (two-reg-misc opcode 0xb). Root-cause: `neg v29.2s,
// v31.2s` (0x2ea0bbfd) collided with the vector float->int FcvVec gate (mask
// 0xffe0_fc00 zeroes bit16), so NEG decoded as fcvtzu and silently zeroed /
// corrupted each lane. This canary reduces across both -x and |x| (gcc -O3
// emits vector `neg`/`abs`); the fused two-loop gen_signed_div in fuzz_jit.py
// is the same family.
#[test]
fn diff_vector_neg_abs_unary() {
    assert_diff(
        "vector_neg_abs_unary",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 42424217ull;
    unsigned long long x = seedv;
    int a[16];
    for(int i=0;i<16;i++){ x=x*1103515245ull+12345ull; a[i]=(int)((x>>31)-(x>>1)); }
    long long s=0;
    for(int i=0;i<16;i++){
        int n = -a[i];
        int m = (a[i]<0) ? -a[i] : a[i];
        s += (long long)(n - m);
    }
    return s;
}
"#,
    );
}

// SIMD integer compare-greater (cmgt) + bsl bit-select used by a vectorized
// min-clamp `a[i] < k ? a[i] : k`. TWO co-resident JIT bugs lived here:
// (1) DECODE — the SimdAddD/B/H gates lacked Simd4s's bit15 guard, so cmgt
// (bit15 CLEAR) was swallowed as a vector ADD, summing lanes instead of
// comparing (wrong clamp values, silent); (2) TRANSLATE — SimdCmgt used
// cmovg which sets 0 ON greater (inverted), so the mask came out all-ones
// where it should be 0. Both fixed. gcc -O3 vectorizes the clamp into
// cmgt+bsl+add+addp over 64-bit lanes.
#[test]
fn diff_simd_cmgt_clamp_min() {
    assert_diff(
        "simd_cmgt_clamp",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 777313ull;
    unsigned long long x = seedv;
    long long b[16];
    for(int i=0;i<16;i++){ x=x*2862933555777941757ull+3037000493ull; b[i]=(long long)(((x>>40)&0x7ffff)-65536); }
    long long s1=0, s2=0;
    for(int i=0;i<16;i++){ long long v=b[i]; if(v>100000ll) v=100000ll; s1+=v; }   // min( ,100000)
    for(int i=0;i<16;i++){ long long v=b[i]; if(v<100000ll) v=100000ll; s2+=v; }   // max( ,100000)
    return s1*7 + s2;
}
"#,
    );
}

// Full-program FP edge-case canary (fuzz seed 9000_58, cycle-33 open bug). gcc
// -O3 jointly schedules SIMD fcvtzs v.2d (in-place), integer cmgt clamps, an
// addp/pairwise reduce, AND scalar fcsel min/max over a NaN — and one `fcsel
// Dd,Dn,Dm,mi` (0x1e654f9a) shares its top-16 (0x1e65) with fcvtau, so the
// fcvt-to-int gate was swallowing it as an FcvtToInt (writing integer X{rd}
// instead of vector D{rd}): the min-accumulator register kept a stale
// a[i]*1e6 double and the final total was wrong by exactly that element.
// jit returned 2474795 vs oracle 2039651 before the fix; both = 2039651 now.
#[test]
fn diff_fp_edge_fcsel_swallowed_as_fcvt() {
    assert_diff(
        "fp_edge_fcsel_fcvt",
        "-O3",
        r#"
long long entry(void){
    volatile unsigned long long seedv = 777313ull;
    unsigned long long x = seedv;
    double a[8];
    for(int i=0;i<8;i++){ x=x*2862933555777941757ull+3037000493ull; a[i]=(double)(((long long)((x>>45)&0x7ffff)-65536))*1e-6; }
    double acc = 0.0; double lo = 1.0/0.0; double hi = -1.0/0.0;
    double sp = (double)(-1.0/0.0);
    for(int i=0;i<8;i++){ acc = acc + a[i]; }
    acc = acc + sp;
    for(int i=0;i<8;i++){ if(a[i]<lo) lo=a[i]; if(a[i]>hi) hi=a[i]; }
    double nn = (double)(0.0/0.0);
    double mn = a[0] < nn ? a[0] : nn;
    double mx = a[0] > nn ? a[0] : nn;
    long long ivals=0;
    for(int i=0;i<8;i++){
        long long v=(long long)(a[i]*1e6); if(v>4000000000ll) v=4000000000ll; if(v<-4000000000ll) v=-4000000000ll;
        ivals += v;
    }
    double fp = (double)ivals;
    long long back = (long long)fp;
    long long zero_cmp = (double)(0.0) > (double)-0.0 ? 7 : 3;
    long long r = ((long long)acc & 0x1ffff) + (long long)lo + (long long)hi + (long long)mn + (long long)mx + back + zero_cmp + (ivals&0xff);
    return r & 0xfffffffff;
}
"#,
    );
}
