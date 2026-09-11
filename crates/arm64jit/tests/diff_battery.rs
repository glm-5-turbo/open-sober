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
    let (_n, _u) = arm64jit::plt::bind_image_plt(&el);
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