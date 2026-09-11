// SPDX-License-Identifier: MIT
//
// Integration test: load a real cross-compiled aarch64 ELF with libloader's
// `load_elf_image`, bind its PLT (via `plt::bind_image_plt`), bootstrap a
// guest stack + TLS, and run through the PC-driven `jit::jit_run` dispatcher —
// the exact pipeline `elfjit`/`sober-core --jit` use to run the real
// libroblox.so on a capable host. This is a permanent regression gate for the
// loader→JIT path (the "drive the load path end-to-end against a synthetic/
// test ELF to confirm no regression from the divert + JNI changes" item from
// session bdd8b03), since the real-binary boot (HARD GATE) cannot be exercised
// on a GPU-less, APK-less host.
//
// It cross-compiles a handful of small -nostdlib aarch64 programs and asserts
// `jit_run` returns the documented value for each. Requires
// `aarch64-linux-gnu-gcc` on PATH; when absent the tests skip (so the suite
// stays green on hosts without the cross toolchain).

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use arm64jit::jit::{CpuState, jit_run};
use libloader::elf::load_elf_image;

/// Serialize the actual load+bind+run, because `load_elf_image` maps the JIT
/// image with `MAP_FIXED` at the fixed JIT base (0x400000 for non-PIE), and
/// four `#[test]` threads running in one process would concurrently map the
/// SAME fixed address and clobber each other's guest image (race → a `bl`
/// target missing from the compiled block's stub table panics in `jit_run`).
/// Real `open-sober` usage loads one guest ELF for the lifetime of the
/// process, so this is purely a test-harness serialization concern.
fn run_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Lock `run_lock`, tolerating a poisoned guard: if one test panics while
/// holding the lock (e.g. its cross-gcc compile fails), that mutex becomes
/// poisoned and `.unwrap()` on subsequent acquires would cascade-fail every
/// other test with `PoisonError`, hiding the real per-test result. Recovering
/// the poisoned guard keeps each test independently reporting its own outcome.
fn lock_run() -> std::sync::MutexGuard<'static, ()> {
    run_lock().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn cross_gcc() -> Option<String> {
    let out = Command::new("aarch64-linux-gnu-gcc")
        .arg("--version")
        .output()
        .ok()?;
    if out.status.success() {
        Some("aarch64-linux-gnu-gcc".to_string())
    } else {
        None
    }
}

fn workdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("open-sober-jit-regress-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Compile `src` (a C program with `int entry(void)`) to a static non-PIE
/// aarch64 ELF and return its path.
fn compile(workdir: &std::path::Path, name: &str, src: &str) -> PathBuf {
    let c = workdir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let elf = workdir.join(format!("{name}.elf"));
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-static", "-nostdlib", "-Wl,-e,entry"])
        .arg(&c)
        .arg("-o")
        .arg(&elf)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc: {e}"));
    assert!(
        out.status.success(),
        "cross-gcc failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    elf
}

/// Compile `src` to a static aarch64 ELF with **-O3** (auto-vectorization on),
/// the shape the differential pair/reduction generators rely on to emit the
/// SIMD widening ops (`uaddl`/`uxtl`/`ext`, ...) that -O0 keeps scalar.
fn compile_o3(workdir: &std::path::Path, name: &str, src: &str) -> PathBuf {
    let c = workdir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let elf = workdir.join(format!("{name}.elf"));
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-O3", "-static", "-nostdlib", "-Wl,-e,entry"])
        .arg(&c)
        .arg("-o")
        .arg(&elf)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc: {e}"));
    assert!(
        out.status.success(),
        "cross-gcc failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    elf
}

/// Compile `src` to a **PIE** (`-fPIE -pie`, dynamic/ET_DYN) aarch64 ELF —
/// the shape of a real shared library — and return its path. Such a PIE
/// carries `R_AARCH64_RELATIVE` data relocations for pointer globals that
/// `load_elf_image` must materialize before the code dereferences them.
fn compile_pie(workdir: &std::path::Path, name: &str, src: &str) -> PathBuf {
    let c = workdir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let elf = workdir.join(format!("{name}.elf"));
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-fPIE", "-pie", "-nostdlib", "-Wl,-e,entry"])
        .arg(&c)
        .arg("-o")
        .arg(&elf)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc: {e}"));
    assert!(
        out.status.success(),
        "cross-gcc failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    elf
}

/// Load `elf`, bind its PLT, bootstrap guest stack/TLS/auxv (mirroring
/// `crates/arm64jit/examples/elfjit.rs`), and run the entry through
/// `jit_run`. Returns the guest x0 at `ret`.
fn run_elf(path: &std::path::Path) -> Result<u64, String> {
    let el = unsafe { load_elf_image(path) }.map_err(|e| format!("load_elf_image: {e:#}"))?;

    let (_nbound, _unresolved) = arm64jit::plt::bind_image_plt(&el, None);

    // Full mapped span (all PT_LOADs + inter-segment gaps) as valid-pc extent,
    // relative to the exec segment base — same as elfjit.
    let seg = el
        .segments
        .iter()
        .find(|s| s.prot.execute)
        .ok_or_else(|| "no executable segment".to_string())?;
    let base = seg.guest_vaddr; // guest == host with load_elf_image
    let full_end = el
        .segments
        .iter()
        .fold(0u64, |m, s| m.max(s.guest_vaddr + s.memsz));
    let len = (full_end - base) as usize;
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, len) };

    let mut st = CpuState::new();
    let entry = el.info.entry; // relocated to guest space by load_elf_image

    // Guest stack (kernel-style initial stack with a minimal auxv) + TLS.
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let stack = Box::leak(vec![0u8; STACK_SIZE].into_boxed_slice());
    let mut auxv = arm64jit::boot::standard_auxv(
        &el,
        arm64jit::boot::HWCAP_FP | arm64jit::boot::HWCAP_ASIMD,
        0,
    );
    let sp = arm64jit::boot::layout_initial_stack(
        stack.as_ptr() as *mut u8,
        STACK_SIZE,
        Some(&[0u8; 0]),
        &[],
        &mut auxv,
    );
    st.set(31, sp);
    let tls = Box::leak(vec![0u8; 64 * 1024].into_boxed_slice());
    // Seed the guest TLS region from the image's PT_TLS (local-exec/initial-exec
    // thread-locals) so `mrs tpidr_el0` + `:tprel:` addressing reads real data.
    // ELFs without a PT_TLS segment get tpidr = region (unchanged behaviour).
    st.tpidr = libloader::elf::setup_guest_tls(&el.info, path, tls.as_ptr() as *mut u8, 64 * 1024)
        .map_err(|e| format!("setup_guest_tls: {e:#}"))?;

    jit_run(image, base, entry, &mut st as *mut CpuState)
}

/// Run a single program end-to-end and assert its return value.
fn assert_runs(tag: &str, src: &str, expected: u64) {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping {tag}: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir(tag);
    let elf = compile(&wd, tag, src);
    match run_elf(&elf) {
        Ok(v) => {
            assert_eq!(
                v, expected,
                "{tag}: jit_run returned {v}, expected {expected}"
            );
            eprintln!("\x1b[32mPASS\x1b[0m {tag}: entry() -> {v}");
        }
        Err(e) => panic!("{tag}: jit_run failed: {e}"),
    }
    let _ = std::fs::remove_dir_all(&wd);
}

#[test]
fn loader_run_add_returns_42() {
    assert_runs(
        "add",
        "int entry(void){ return 40 + 2; }\n",
        42,
    );
}

#[test]
fn loader_run_loop_accumulates_45() {
    assert_runs(
        "loop",
        "int entry(void){ int s=0; for(int i=0;i<10;i++) s+=i; return s; }\n",
        45,
    );
}

#[test]
fn loader_run_fp_mul_returns_10() {
    assert_runs(
        "fp",
        "int entry(void){ return (int)(2.5 * 4.0); }\n",
        10,
    );
}

#[test]
fn loader_run_recursion_fib7_is_13() {
    assert_runs(
        "fib",
        "static int f(int n){ return n<2 ? n : f(n-1)+f(n-2); }\n\
         int entry(void){ return f(7); }\n",
        13,
    );
}

#[test]
fn loader_run_pie_relative_global_returns_42() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_pie: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("pie-rel");
    // A REAL PIE (ET_DYN): `gptr = &shared_static` is an R_AARCH64_RELATIVE
    // data reloc in .data.rel.ro that `load_elf_image` must apply (write
    // base+addend into the gptr slot) before the JIT dereferences the pointer
    // global; without the apply, *gptr reads the unrelocated link address on
    // the NULL page and faults. Entry dereferences through the global -> 42.
    let elf = compile_pie(
        &wd,
        "pie",
        "static int shared_static = 41;\n\
         int *gptr = &shared_static;\n\
         int entry(void){ return *gptr + 1; }\n",
    );

    // Sanity: the fixture really is a PIE with the RELATIVE reloc.
    let rel = Command::new("aarch64-linux-gnu-readelf")
        .args(["-r", "--use-dynamic"])
        .arg(&elf)
        .output()
        .expect("readelf missing");
    let rel_txt = String::from_utf8_lossy(&rel.stdout);
    assert!(
        rel_txt.contains("R_AARCH64_RELATIVE") || rel_txt.contains("R_AARCH64_RELATIV"),
        "PIE fixture should have an R_AARCH64_RELATIVE reloc:\n{}",
        rel_txt
    );

    match run_elf(&elf) {
        Ok(v) => {
            assert_eq!(
                v, 42,
                "pie: entry() -> {v}, expected 42 (RELATIVE not applied?)"
            );
            eprintln!("\x1b[32mPASS\x1b[0m pie: entry() -> {v} via RELATIVE-relocated global");
        }
        Err(e) => panic!("pie: jit_run failed: {e}"),
    }
    let _ = std::fs::remove_dir_all(&wd);
}

/// Compile `src` (a C program with `int entry(void)`) to a **shared library**
/// (`-shared -fPIC`, ET_DYN) with entry exported — the shape of a real Android
/// `.so`. Unlike the PIE compile, `-shared` makes exported globals referenced
/// from module code go through the main GOT via `R_AARCH64_GLOB_DAT` (and
/// function-pointer/data initializers become `R_AARCH64_ABS64`); both are the
/// "symbol runtime-address into a location" family `bind_glob_dat` resolves.
fn compile_shared(workdir: &std::path::Path, name: &str, src: &str) -> PathBuf {
    let c = workdir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let elf = workdir.join(format!("{name}.so"));
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-shared", "-fPIC", "-nostdlib", "-Wl,-e,entry"])
        .arg(&c)
        .arg("-o")
        .arg(&elf)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc: {e}"));
    assert!(
        out.status.success(),
        "cross-gcc failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    elf
}

#[test]
fn loader_run_shared_glob_dat_and_abs64_returns_37() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_glob_dat: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("globdat");
    // Two exported globals referenced through the module's own GOT:
    //   global_data (1 GLOB_DAT slot + must bind base+0x20000),
    //   gfp = &internal_fn (1 GLOB_DAT slot -> base+0x20008) whose initializer
    //   is an R_AARCH64_ABS64 (write &internal_fn = base+0x340).
    // entry() reads global_data via GOT, calls gfp(3) through GOT (-> 3*5=15),
    // then adds global_data again: 11 + 15 + 11 = 37. Without GLOB_DAT/ABS64
    // binding the guest GOT loads 0 and calls/derefs NULL (SIGSEGV or 0).
    let elf = compile_shared(
        &wd,
        "sh",
        "int internal_fn(int x){ return x*5; }\n\
         int global_data = 11;\n\
         int (*gfp)(int) = internal_fn;\n\
         int entry(void){ int acc = global_data; acc += gfp(3); return acc + global_data; }\n",
    );

    // The real run_elf pipeline applies RELATIVE only; GLOB_DAT/ABS64 must be
    // bound by bind_image_plt before jit_run. Confirm jit_run returns 37.
    match run_elf(&elf) {
        Ok(v) => {
            assert_eq!(
                v, 37,
                "globdat: entry() -> {v}, expected 37 (GLOB_DAT/ABS64 not bound?)"
            );
            eprintln!("\x1b[32mPASS\x1b[0m globdat: entry() -> {v} via GLOB_DAT + ABS64-bound globals");
        }
        Err(e) => panic!("globdat: jit_run failed: {e}"),
    }
    let _ = std::fs::remove_dir_all(&wd);
}
#[test]
fn loader_run_self_import_binds_to_own_guest_body() {
    // REGRESSION: a `-shared` module calling one of its OWN exported functions
    // goes through `@plt`; the JUMP_SLOT symbol is defined in the module itself
    // (st_shndx != SHN_UNDEF). Before the binder's self-import fix, this bound
    // to the NULL/0 graphics catch-all (host thunk), so `entry()` dispatched to
    // a stub that returned garbage (observed 0) instead of the real guest body.
    // The guest reloc's st_value is the link address of the definition.
    if cross_gcc().is_none() {
        eprintln!("skipping loader_run_self_import: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir("selfimport");
    // `internal_fn` default visibility => exported => callers use `bl fn@plt`.
    let elf = compile_shared(
        &wd,
        "self",
        "int internal_fn(int x){ return x * 5; }\n\\\n         int entry(void){ return internal_fn(7); }\n",
    );
    // Sanity: the JUMP_SLOT names a symbol the module defines.
    let rel = Command::new("aarch64-linux-gnu-readelf")
        .arg("-rW")
        .arg(&elf)
        .output()
        .unwrap();
    let rel = String::from_utf8_lossy(&rel.stdout);
    assert!(rel.contains("internal_fn"), "fixture has an internal_fn JUMP_SLOT");
    assert!(rel.contains("R_AARCH64_JUMP_SLOT"), "and it's a JUMP_SLOT reloc");

    match run_elf(&elf) {
        Ok(v) => {
            assert_eq!(
                v, 35,
                "selfimport: entry() -> {v}, expected 35 (self-import bound to hose catch-all?)"
            );
            eprintln!("\x1b[32mPASS\x1b[0m selfimport: entry() -> {v} (own-export self-import bound to guest body)");
        }
        Err(e) => panic!("selfimport: jit_run failed: {e}"),
    }
    let _ = std::fs::remove_dir_all(&wd);
}

#[test]
fn loader_run_asymmetric_logic_imm_mask_returns_correct() {
    // End-to-end gate for the LogicImm DecodeBitMasks rotate-RIGHT fix.
    // `volatile` prevents gcc constant-folding, so it emits a real
    // `and x0, x0, #0xffffffff80000001` (0x92618400) on a runtime value — a
    // rotation-ASYMMETRIC logical-immediate mask that the old left-rotate
    // silently miscompiled. Runs the full loader->bind->jit_run pipeline.
    // Correct mask & 1234 = 0 (1234 is even, bits31..63 clear). The OLD
    // left-rotate decoded 0xfffffffe00000007, which keeps bit1 -> 2. So a
    // regression returns 2 instead of 0.
    assert_runs(
        "asymmask",
        "unsigned long long entry(void){ volatile unsigned long long x = 1234; return (x & 0xffffffff80000001ULL); }\n",
        0,
    );
}

#[test]
fn loader_run_pairwise_xor_inplace_uaddl_returns_2314() {
    // End-to-end gate for the in-place uaddl/saddl ALIAS bug (SimdAddl had no
    // permscratch snapshot when rd aliases rn/rm). gcc's pair-xor reduction
    // `s ^= a[i]+a[i+1]` at -O3 emits `uaddl v0.4s, v0.4h, v1.4h` (dest==src):
    // the widened 2*esrc write of lane i clobbers the still-needed narrow
    // source bytes of lane i+1, corrupting the sum. The exact program came from
    // the differential fuzzer (gen_pairwise_reduce) and the 2314 oracle matches
    // BOTH native gcc and qemu-aarch64. Before the fix this returned 59648.
    if cross_gcc().is_none() {
        eprintln!("skipping pairxor: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir("pairxor");
    let src = "\
long long entry(void){\n\
volatile unsigned long long seedv=654321ull; unsigned long long x=seedv; unsigned short a[24];\n\
for(int i=0;i<24;i++){ x=x*6364136223846793005ull+1442695040888963407ull; a[i]=(unsigned short)((x>>24)^(x&0xffff)); }\n\
long long s=0;\n\
for(int i=0;i<24;i+=2) s ^= (long long)a[i] + a[i+1];\n\
return s;\n\
}\n";
    let elf = compile_o3(&wd, "pairxor", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(v, 2314, "pairxor: jit_run returned {v}, expected 2314"),
        Err(e) => panic!("pairxor: jit_run failed: {e}"),
    }
    let _ = std::fs::remove_dir_all(&wd);
}

#[test]
fn loader_run_pairwise_add_reduction_returns_76() {
    // End-to-end gate for the scalar-D `addp Dd, Vn.2D` (SimdPairAddD) fix:
    // under -O3 gcc vectorizes the i*i loop and reduces with `addp d, v.2d`
    // (0x5ef1...) + a smulh magic Nr by-97. The sum is 1240 -> 76. Before the
    // bit20 discriminator, addp collided with the scalar fcvtzs gate and
    // silently ran as a float->int translate -> 0. Runs the full
    // loader->bind->jit_run pipeline.
    assert_runs(
        "pairsum",
        // -O3 forces the SIMD pairwise-add reduction path (addp).
        "int entry(void){ int a[16]; for(int i=0;i<16;i++)a[i]=i*i; long long s=0; for(int i=0;i<16;i++) s+=a[i]; return (int)(s%97); }\n",
        76,
    );
}

#[test]
fn loader_run_neon_byelem_fmla_lane_returns_504() {
    // NEON *by-element* fmla (vmlaq_n_f32 -> `fmla v.4s, v.4s, v.s[l]`) — the
    // scalar-broadcast multiply-accumulate class gcc's auto-vectorizer rarely
    // emits but a 3D/audio engine uses heavily. Lane insert/extract is done via
    // memory (vst1q/vld1q) so this compiles at -O0 (the harness has no -O flag
    // and const-index vgetq_lane/vsetq_lane need -O to fold). Binary-exact
    // lanes keep the accumulated value exact in float32; expected 504 is the
    // qemu-aarch64 architectural oracle (host x86 can't compile <arm_neon.h>,
    // so loader_run's hardcoded-value + cross-gcc model is the right seal).
    assert_runs(
        "byelem",
        "#include <arm_neon.h>\n\
         long long entry(void){\n\
         volatile unsigned long long seedv = 777333ull;\n\
         unsigned long long x = seedv;\n\
         float fa[8];\n\
         for(int i=0;i<8;i++){ x=x*1664525ull+1013904223ull; fa[i]=(float)(int)(((x>>40)&0x3f)-32); }\n\
         float32x4_t v = vdupq_n_f32(0.0f);\n\
         float scalar_acc = 0.0f;\n\
         for(int i=0;i<8;i++){ v = vmlaq_n_f32(v, vdupq_n_f32(fa[i]), 2.0f); scalar_acc += fa[i]*2.0f; }\n\
         float buf[4]; vst1q_f32(buf, v);\n\
         float l1 = buf[1] + 1.0f; buf[2] = l1;\n\
         float32x4_t w = vld1q_f32(buf);\n\
         v = vaddq_f32(v, w);\n\
         float s = 0; vst1q_f32(buf, v); for(int l=0;l<4;l++) s += buf[l];\n\
         return (long long)(s - (scalar_acc + 1.0));\n}\n",
        504,
    );
}

#[test]
fn loader_run_thread_local_storage_returns_123456804() {
    // End-to-end gate for guest TLS bootstrapping (R_AARCH64_TLS local-exec):
    // `__thread` globals compiled by gcc read/write through `mrs tpidr_el0` +
    // baked `:tprel:` offsets (`add x0, tp, #n`). `setup_guest_tls` must copy
    // the PT_TLS init image into a region at TP+16 (the AArch64 TCB) so the
    // accesses land on real data. g_slot=7, g_big=123456789, g_zero=0(.tbss),
    // and bump() adds 1 -> 7+123456789+0+8 = 123456804 (matches the native
    // x86-64 oracle; qemu-aarch64 itself SIGSEGVs on this nostdlib static
    // because it doesn't seed PT_TLS without a dynamic loader). Before the TLS
    // seeding the region was zeroed, so all `__thread` reads returned 0.
    assert_runs(
        "tls1",
        "__thread int g_slot = 7;\n\
         __thread long long g_big = 123456789;\n\
         __thread int g_zero;\n\
         int get_a(void){ return g_slot; }\n\
         int get_b(void){ return (int)g_big; }\n\
         int get_c(void){ return g_zero; }\n\
         int bump(void){ g_slot += 1; return g_slot; }\n\
         int entry(void){ return get_a() + get_b() + get_c() + bump(); }\n",
        123456804,
    );
}

#[test]
fn loader_run_bfi_64_returns_279514809947() {
    // 64-bit bitfield-insert (bfi/bfiz on GPRs, immr>imms overlap the old
    // ubfiz-vs-ror bug): a volatile (so the compiler can't fold it) mask-shift
    // field insert into an accumulator. Native x86-64 oracle == 279514809947;
    // a wrong bfi/bfiz shift or a ror-vs-insert mix computes garbage.
    assert_runs(
        "bfi64",
        "long long entry(void){\n\
         volatile unsigned long long v = 0xFF00FF0012345678ull;\n\
         unsigned long long acc = 0;\n\
         for(int i=0;i<6;i++){ unsigned long long field = (v >> (17+i)) & 0x3f; acc |= (field << (i*7)); acc ^= (v >> i) & 0xff; }\n\
         return (long long)acc;\n}\n",
        279514809947,
    );
}

#[test]
fn loader_run_tbz_branches_returns_4068() {
    // Bit-test branches (tbz/tbnz — gcc emits these from `v & (1ull<<i)` with a
    // compile-time bit): 64 single-bit tests, branch on bit + fallthrough with
    // an odd-i xor. Native oracle == 4068.
    assert_runs(
        "tbz",
        "long long entry(void){\n\
         volatile unsigned long long v = 0x8000000000000123ull;\n\
         long long s = 0;\n\
         for(int i=0;i<64;i++){ if(v & (1ull<<i)) s += (long long)(i*i); else if(i & 1) s ^= i; }\n\
         return s;\n}\n",
        4068,
    );
}

#[test]
fn loader_run_fixed_pt_fcvt_returns_99() {
    // Fixed-point float->int (fcvtzs/fcvtzu #fbits): (long long)(a*2^k) with a
    // volatile k so it isn't folded; also trunc toward zero on negatives
    // ((long long)(a/2.0)). Native oracle == 99.
    assert_runs(
        "fcvtfp",
        "long long entry(void){\n\
         volatile double a[6] = {0.75, -1.5, 2.25, -3.75, 8.5, -0.125};\n\
         volatile double k = 16.0;\n\
         long long s = 0;\n\
         for(int i=0;i<6;i++){ s += (long long)(a[i]*k); if(i & 1) s -= (long long)(a[i]/2.0); }\n\
         return s;\n}\n",
        99,
    );
}

/// Run a whole `DT_NEEDED` chain through the loader → cross-module binder →
/// `jit_run` pipeline. Mirrors `run_elf` but loads the dependency closure,
/// binds every module against the combined export scope, and feeds `jit_run` a
/// single image slice covering the whole contiguous chain.
fn run_chain(
    chain: &libloader::deps::LoadedChain,
    main_path: &std::path::Path,
) -> Result<u64, String> {
    let refs: Vec<&libloader::elf::LoadedElf> = chain.entries.iter().collect();
    let scope = arm64jit::plt::build_export_scope(&refs);
    for el in &chain.entries {
        arm64jit::plt::bind_image_plt(el, Some(&scope));
    }

    let main = chain.main();
    let base = chain.base();
    let end = chain.end();
    let len = (end - base) as usize;
    let image = unsafe { std::slice::from_raw_parts(base as *const u8, len) };

    let mut st = CpuState::new();
    let entry = main.info.entry; // relocated to guest space by load_elf_image_at

    // Guest stack + TLS (kernel-style initial stack), same as run_elf/elfjit.
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    let stack = Box::leak(vec![0u8; STACK_SIZE].into_boxed_slice());
    let mut auxv = arm64jit::boot::standard_auxv(
        main,
        arm64jit::boot::HWCAP_FP | arm64jit::boot::HWCAP_ASIMD,
        0,
    );
    let sp = arm64jit::boot::layout_initial_stack(
        stack.as_ptr() as *mut u8,
        STACK_SIZE,
        Some(&[0u8; 0]),
        &[],
        &mut auxv,
    );
    st.set(31, sp);
    let tls = Box::leak(vec![0u8; 64 * 1024].into_boxed_slice());
    st.tpidr = libloader::elf::setup_guest_tls(
        &main.info,
        main_path,
        tls.as_ptr() as *mut u8,
        64 * 1024,
    )
    .map_err(|e| format!("setup_guest_tls: {e:#}"))?;

    jit_run(image, base, entry, &mut st as *mut CpuState)
}

#[test]
fn loader_run_needed_dep_cross_module_call_returns_82() {
    // Multi-module (DT_NEEDED) end-to-end: the loader resolves the main .so's
    // `DT_NEEDED libdep.so`, maps the dependency contiguously after it in the
    // same guest region, and the guest's import of the dependency's exported
    // function (`bl dep_val@plt`, a JUMP_SLOT) AND exported data global
    // (dep_global, a GLOB_DAT), neither of which the host resolver can satisfy,
    // both resolve through the cross-module export scope to the dependency's
    // own guest address, which jit_run compiles from the shared image slice.
    // entry() = dep_val() + dep_global + 2 = 40 + 40 + 2 = 82.
    if cross_gcc().is_none() {
        eprintln!("skipping loader_run_needed_dep: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir("chain");

    // libdep.so exports dep_val() = 40 and a data global dep_global = 40
    // (a plain -shared library, no entry).
    let dep_c = wd.join("dep.c");
    std::fs::write(&dep_c, "int dep_global = 40;\nint dep_val(void){ return dep_global; }\n").unwrap();
    let dep_so = wd.join("libdep.so");
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-shared", "-fPIC", "-nostdlib"])
        .arg(&dep_c)
        .arg("-o")
        .arg(&dep_so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc (dep): {e}"));
    assert!(
        out.status.success(),
        "cross-gcc (dep) failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // libmain.so NEEDs libdep.so; entry calls dep_val() AND reads dep_global,
    // so it exercises BOTH the cross-module JUMP_SLOT (function) and GLOB_DAT
    // (data-global) scope-resolution paths: 40 + 40 + 2 = 82.
    let main_c = wd.join("main.c");
    std::fs::write(
        &main_c,
        "extern int dep_global; extern int dep_val(void); int entry(void){ return dep_val() + dep_global + 2; }\n",
    )
    .unwrap();
    let main_so = wd.join("libmain.so");
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-shared", "-fPIC", "-nostdlib", "-Wl,-e,entry"])
        .arg("-Wl,--no-as-needed")
        .arg("-L")
        .arg(&wd)
        .arg("-l")
        .arg("dep")
        .arg(&main_c)
        .arg("-o")
        .arg(&main_so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run cross-gcc (main): {e}"));
    assert!(
        out.status.success(),
        "cross-gcc (main) failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Sanity: libmain really DT_NEEDs libdep.
    let rel = Command::new("aarch64-linux-gnu-readelf")
        .args(["-d"])
        .arg(&main_so)
        .output()
        .expect("readelf missing");
    let dyn_txt = String::from_utf8_lossy(&rel.stdout);
    assert!(
        dyn_txt.contains("libdep.so"),
        "libmain should DT_NEED libdep.so:\n{dyn_txt}"
    );

    let search = vec![wd.clone()];
    let chain = libloader::deps::load_elf_with_deps(&main_so, &search)
        .unwrap_or_else(|e| panic!("load_elf_with_deps: {e:#}"));
    assert_eq!(
        chain.entries.len(),
        2,
        "expected main + 1 dependency, got {}",
        chain.entries.len()
    );
    // Dependency (entry 1) sits at a distinct, higher guest base than main.
    let main_base = chain.main().base_addr as u64;
    let dep_base = chain.entries[1].base_addr as u64;
    assert!(
        dep_base > main_base,
        "dependency should map after main (dep {dep_base:#x} <= main {main_base:#x})"
    );

    match run_chain(&chain, &main_so) {
        Ok(v) => assert_eq!(
            v, 82,
            "chain: entry() -> {v}, expected 82 (cross-module JUMP_SLOT/GLOB_DAT not resolved?)"
        ),
        Err(e) => panic!("chain: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m chain: entry() -> 82 via DT_NEEDED cross-module JUMP_SLOT + GLOB_DAT");
    let _ = std::fs::remove_dir_all(&wd);
}

/// Like `compile_shared` but for a dependency library: `-shared -fPIC
/// -nostdlib`, no entry, with a NEEDED link to `lib<need>.so` in `workdir`.
fn compile_dep(workdir: &std::path::Path, name: &str, src: &str, need: Option<&str>) -> PathBuf {
    let c = workdir.join(format!("{name}.c"));
    std::fs::write(&c, src).unwrap();
    let so = workdir.join(format!("lib{name}.so"));
    let mut cmd = Command::new("aarch64-linux-gnu-gcc");
    cmd.args(["-shared", "-fPIC", "-nostdlib", "-Wl,--no-as-needed", "-Wl,-e,entry"]);
    if let Some(n) = need {
        cmd.arg("-L").arg(workdir).arg("-l").arg(n);
    }
    cmd.arg(&c).arg("-o").arg(&so);
    let out = cmd.output().unwrap_or_else(|e| panic!("failed to run cross-gcc ({name}): {e}"));
    assert!(
        out.status.success(),
        "cross-gcc ({name}) failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    so
}

#[test]
fn loader_run_deep_dep_chain_transitive_returns_44() {
    // Transitive DT_NEEDED closure: libmain NEEDs libdep2, which NEEDs libdep3.
    // entry() → dep2_val() → dep3_val(): the loader must recursively resolve
    // BOTH edges, map all three modules contiguously, and bind libdep2's import
    // of dep3_val (its OWN NEEDED, one level down) through the scope to
    // libdep3's guest address, in addition to binding main → libdep2.
    //   dep3_val() = 30 ; dep2_val() = dep3_val() + 12 = 42 ; entry = 42 + 2.
    if cross_gcc().is_none() {
        eprintln!("skipping loader_run_deep_dep_chain: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir("chain2");

    compile_dep(&wd, "dep3", "int dep3_val(void){ return 30; }\n", None);
    compile_dep(
        &wd,
        "dep2",
        "extern int dep3_val(void); int dep2_val(void){ return dep3_val() + 12; }\n",
        Some("dep3"),
    );
    let main_so = compile_dep(
        &wd,
        "main",
        "extern int dep2_val(void); int entry(void){ return dep2_val() + 2; }\n",
        Some("dep2"),
    );

    let search = vec![wd.clone()];
    let chain = libloader::deps::load_elf_with_deps(&main_so, &search)
        .unwrap_or_else(|e| panic!("load_elf_with_deps: {e:#}"));
    assert_eq!(
        chain.entries.len(),
        3,
        "expected main + 2 transitive dependencies, got {}",
        chain.entries.len()
    );
    // Each module at a strictly increasing guest base (contiguous chain).
    let bases: Vec<u64> = chain.entries.iter().map(|e| e.base_addr as u64).collect();
    assert!(
        bases.windows(2).all(|w| w[1] > w[0]),
        "modules should map at increasing bases: {bases:#x?}"
    );

    match run_chain(&chain, &main_so) {
        Ok(v) => assert_eq!(
            v, 44,
            "chain2: entry() -> {v}, expected 44 (transitive dep binding failed?)"
        ),
        Err(e) => panic!("chain2: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m chain2: entry() -> 44 via 2-level transitive DT_NEEDED closure");
    let _ = std::fs::remove_dir_all(&wd);
}
