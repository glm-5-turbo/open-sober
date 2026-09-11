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