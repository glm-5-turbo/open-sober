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
fn loader_run_recursive_import_bearing_callee_returns_correct() {
    // REGRESSION: a recursive guest `bl f` where f's body ALSO calls a host
    // import (helper@plt). f is import-bearing, so its recursion must DIVERT via
    // a dispatcher stub (fresh f frame) — but the fixup-resolution previously
    // re-resolved `bl f` to `host_of_guest[f]` (f is the block's own start), so
    // the recursion inlined into the same block and the inline-call/diverted-
    // import interaction miscompiled (f(5)+helper chain returned 21, not 27).
    // This is the documented "nested guest bl-inline regression to once-routine"
    // bug, reproduced minimally.
    if cross_gcc().is_none() {
        eprintln!("skipping loader_run_recursive_import_bearing: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir("recimp");
    // f(n) = n<=0 ? 7 : f(n-1) + helper(n); helper(x)=x+1; entry=f(5).
    // Native oracle: f(0)=7, f(1)=9, f(2)=12, f(3)=16, f(4)=21, f(5)=27.
    let elf = compile_shared(
        &wd,
        "rec",
        "int helper(int x){ return x+1; }\n\\\n         int f(int n){ return n<=0 ? 7 : f(n-1) + helper(n); }\n\\\n         int entry(void){ return f(5); }\n",
    );
    match run_elf(&elf) {
        Ok(v) => {
            assert_eq!(
                v, 27,
                "recursive-import-bearing: entry() -> {v}, expected 27 (recursive bl f not diverted?)"
            );
            eprintln!("\x1b[32mPASS\x1b[0m recursive-import-bearing: entry() -> {v} (recursive f diverts cleanly)");
        }
        Err(e) => panic!("recursive-import-bearing: jit_run failed: {e}"),
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
    _main_path: &std::path::Path,
) -> Result<u64, String> {
    let refs: Vec<&libloader::elf::LoadedElf> = chain.entries.iter().collect();
    let scope = arm64jit::plt::build_export_scope(&refs);
    for el in &chain.entries {
        arm64jit::plt::bind_image_plt(el, Some(&scope));
    }
    // Cross-module TLS: seed every module's `__thread` block into one per-thread
    // region (not just the main image's) and bind the chain's
    // `R_AARCH64_TLS_TPREL64` / `R_AARCH64_TLSDESC` GOT slots to the resulting
    // TP-relative offsets. Without this a dependency's `__thread` reads garbage.
    let tls = Box::leak(vec![0u8; 64 * 1024].into_boxed_slice());
    let (tp, tls_offsets) = libloader::deps::setup_chain_tls(&chain, tls.as_ptr() as *mut u8, 64 * 1024)
        .map_err(|e| format!("setup_chain_tls: {e:#}"))?;
    arm64jit::plt::bind_chain_tls(&refs, &tls_offsets);
    // General-dynamic `__tls_get_addr` needs TP + the chain offsets at runtime.
    arm64jit::plt::set_chain_tls(tp, tls_offsets);

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
    st.tpidr = tp;

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

/// Build a main+N-dep chain where the DEPENDENCY carries `__thread` TLS, run it
/// through the full loader → cross-module binder → chain-TLS → `jit_run`
/// pipeline, and assert `entry()` == `expected`.
///
/// `dep_src`, `main_src` are the C sources; `tls_flags` the extra gcc flags for
/// the dep (the TLS model under test); `expected` the oracle return value. Both
/// `mrs tpidr_el0`:gottprel (initial-exec) and the TLSDESC descriptor calls are
/// exercised depending on `tls_flags` (GCC 13+ defaults to TLSDESC even for
/// `-ftls-model=global-dynamic`).
fn run_tls_chain_test(
    tag: &str,
    dep_src: &str,
    main_src: &str,
    tls_flags: &[&str],
    expect_reloc: &str,
    expected: u64,
) {
    if cross_gcc().is_none() {
        eprintln!("skipping {tag}: aarch64-linux-gnu-gcc not available");
        return;
    }
    let _guard = lock_run();
    let wd = workdir(tag);

    let dep_c = wd.join("dep.c");
    std::fs::write(&dep_c, dep_src).unwrap();
    let dep_so = wd.join("libdep.so");
    let mut cmd = Command::new("aarch64-linux-gnu-gcc");
    cmd.args(["-shared", "-fPIC", "-nostdlib"]);
    cmd.args(tls_flags);
    cmd.arg(&dep_c).arg("-o").arg(&dep_so);
    let out = cmd.output().unwrap_or_else(|e| panic!("failed to run cross-gcc (dep): {e}"));
    assert!(
        out.status.success(),
        "cross-gcc (dep) failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let main_c = wd.join("main.c");
    std::fs::write(&main_c, main_src).unwrap();
    let main_so = wd.join("libmain.so");
    let out = Command::new("aarch64-linux-gnu-gcc")
        .args(["-shared", "-fPIC", "-nostdlib", "-Wl,-e,entry", "-Wl,--no-as-needed"])
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

    // Sanity: the dep really carries the TLS relocation under test.
    let rel = Command::new("aarch64-linux-gnu-readelf")
        .args(["-rW"])
        .arg(&dep_so)
        .output()
        .unwrap();
    let rel = String::from_utf8_lossy(&rel.stdout);
    assert!(
        rel.contains(expect_reloc),
        "dep should carry a {expect_reloc} reloc:\n{rel}"
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

    match run_chain(&chain, &main_so) {
        Ok(v) => assert_eq!(
            v, expected,
            "{tag}: entry() -> {v}, expected {expected} (dep __thread TLS not bound? GOT unbound? block not seeded?)"
        ),
        Err(e) => panic!("{tag}: jit_run failed: {e}"),
    }
    eprintln!(
        "\x1b[32mPASS\x1b[0m {tag}: entry() -> {v} via cross-module dep TLS",
        v = expected
    );
    let _ = std::fs::remove_dir_all(&wd);
}

#[test]
fn loader_run_chain_dep_tls_tlsdesc_returns_1007() {
    // Dependency-local `__thread` accessed with GCC's DEFAULT TLS model, which
    // on this toolchain is TLSDESC (R_AARCH64_TLSDESC, 1031): the guest fetch is
    // `adrp/ldr/add/blr <resolver>; mrs tpidr_el0; add x0,tp,x0`. bind_chain_tls
    // plants the 16-byte descriptor {resolver, tprel} at the GOT slot and
    // setup_chain_tls seeds the dep's TLS block. entry = dep_getx() (7) +
    // dep_gety() (1000) = 1007. Before this fix the descriptor was uninitialized,
    // so the resolver returned garbage.
    run_tls_chain_test(
        "chaintls_tlsdesc",
        "__thread int dep_x = 7;\n\
         __thread long long dep_y = 1000;\n\
         int dep_getx(void){ return dep_x; }\n\
         long long dep_gety(void){ return dep_y; }\n",
        "extern int dep_getx(void); extern long long dep_gety(void);\n\
         int entry(void){ return dep_getx() + (int)dep_gety(); }\n",
        &[],
        "R_AARCH64_TLSDESC",
        1007,
    );
}

#[test]
fn loader_run_chain_dep_tls_initial_exec_returns_53() {
    // Dependency-local `__thread` with initial-exec (R_AARCH64_TLS_TPREL64,
    // 1030): `mrs tpidr_el0; adrp; ldr x0,[GOT]; add x0,tp,x0`. The GOT slot
    // must hold the TP-relative offset = dep block offset + symbol offset.
    // entry = dep_getx() (5) * dep_gety() (10) + 3 = 53.
    run_tls_chain_test(
        "chaintls_ie",
        "__thread int dep_x = 5;\n\
         __thread long long dep_y = 10;\n\
         int dep_getx(void){ return dep_x; }\n\
         long long dep_gety(void){ return dep_y; }\n",
        "extern int dep_getx(void); extern long long dep_gety(void);\n\
         int entry(void){ return dep_getx() * (int)dep_gety() + 3; }\n",
        &["-ftls-model=initial-exec"],
        "R_AARCH64_TLS_TPREL64",
        53,
    );
}

#[test]
fn loader_run_chain_dep_tls_global_dynamic_returns_403() {
    // Classic general-dynamic TLS via `__tls_get_addr(&tls_index)` — forced by
    // `-mtls-dialect=trad -ftls-model=global-dynamic` because GCC 13+ defaults
    // to TLSDESC. The dep's `dep_x`/`dep_y` access goes through
    // `__tls_get_addr@plt` with a 16-byte GOT `tls_index` holding the module id
    // (R_AARCH64_TLS_DTPMOD64, 1028) and the block offset
    // (R_AARCH64_TLS_DTPREL64, 1029). bind_chain_tls writes both; the
    // `__tls_get_addr` JUMP_SLOT binds to the host TLS resolver (via
    // ensure_tls_get_addr) which returns TP + offsets[module] + offset using
    // the chain state set by run_chain. entry = dep_getx() (3) + dep_gety()
    // (400) = 403. Before this slice the JUMP_SLOT fell to the NULL catch-all
    // and the guest called garbage.
    run_tls_chain_test(
        "chaintls_gd",
        "__thread int dep_x = 3;\n\
         __thread long long dep_y = 400;\n\
         int dep_getx(void){ return dep_x; }\n\
         long long dep_gety(void){ return dep_y; }\n",
        "extern int dep_getx(void); extern long long dep_gety(void);\n\
         int entry(void){ return dep_getx() + (int)dep_gety(); }\n",
        &["-mtls-dialect=trad", "-ftls-model=global-dynamic"],
        "R_AARCH64_TLS_DTPMOD64",
        403,
    );
}
    // Transitive DT_NEEDED closure: a NEW test header below.
#[test]
fn loader_run_deep_dep_chain_transitive_returns_44() {
    // libmain NEEDs libdep2, which NEEDs libdep3.
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

/// The guest thread model: `clone` (syscall 220) spawns a real host thread
/// that re-enters `jit_run` at the post-svc PC and continues the guest,
/// sharing the image memory (guest==host). The child here publishes a value to
/// a shared global (via its own stack, its own tid), then thread-exits; the
/// parent busy-waits on the flag with a bounded spin and returns what it read.
/// This proves the child actually RAN on a distinct host thread reading the
/// SAME guest address space, and that its thread-local exit (93) unwound its
/// `jit_run` without killing the parent.
#[test]
fn loader_run_clone_spawns_guest_thread_shared_memory() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_clone: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("clone-thread");

    let src = r#"
volatile long g_shared = 0;

int entry(void){
    // The child's stack lives in the image's .bss (real mapped memory shared
    // with the child thread, since guest==host).
    static char stack[65536] __attribute__((aligned(16)));

    // Raw AArch64 clone(2): x0=flags, x1=newsp, x2=ptid, x3=tls, x4=ctid.
    // CLONE_VM(0x100)|FS(0x200)|FILES(0x400)|SIGHAND(0x800) = 0xF00.
    register long x8 asm("x8") = 220;
    register long x0 asm("x0") = 0xF00;
    // The guest resumes at the post-svc PC WITHOUT the caller's `sub sp,#0x20`
    // prologue (the child is a fresh thread), so its frame locals live at
    // [sp+#8..#28] — ABOVE sp. Point child_stack below the buffer top so those
    // positive-offset locals land inside the mapped .bss stack region.
    register long x1 asm("x1") = (long)(stack + 65536 - 128);
    register long x2 asm("x2") = 0;
    register long x3 asm("x3") = 0;
    register long x4 asm("x4") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4) : "memory");
    long tid = x0; // parent -> child tid, child -> 0

    if (tid == 0) {
        // ---- child thread ----
        int acc = 0;
        for (int i = 0; i < 64; i++) acc += i; // 2016, real compute on child stack
        g_shared = 6 * 7 + (acc == 2016 ? 0 : 1000); // 42 if the loop ran correctly
        // thread-local exit (93) — must NOT kill the parent's process
        register long x8c asm("x8") = 93;
        register long x0c asm("x0") = 0;
        asm volatile("svc #0" :: "r"(x8c), "r"(x0c) : "memory");
        return -2; // unreachable (thread exit unwinds jit_run first)
    }

    // ---- parent thread: bounded spin for the child's publish ----
    long spins = 0;
    while (g_shared == 0 && spins < 1000000000L) { spins++; }
    return g_shared == 0 ? -1 : (int)g_shared;
}
"#;

    let elf = compile(&wd, "clone", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "clone-thread: entry() -> {v}, expected 42 (child did not publish/sum 2016?)"
        ),
        Err(e) => panic!("clone-thread: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m clone-thread: guest thread wrote 42 to shared memory");
    let _ = std::fs::remove_dir_all(&wd);
}

/// `pthread_join` primitive: clone with CLONE_CHILD_CLEARTID, then have the
/// PARENT block on FUTEX_WAIT on the child's clear-tid word. The child computes,
/// writes its result to a shared global, then thread-exits (93); the exit path
/// zeroes the clear-tid word + FUTEX_WAKEs it, so the parent's FUTEX_WAIT
/// returns and the join succeeds. This proves the blocking-join mechanism
/// (render/audio/network workers that a host main loop joins) works end-to-end.
#[test]
fn loader_run_clone_child_cleartid_join_via_futex() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_join: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("clone-join");

    let src = r#"
volatile long g_result = 0;
volatile int g_ctlid = 0; // child clear-tid word (written by CLONE_CHILD_SETTID)

int entry(void){
    static char stack[131072] __attribute__((aligned(16)));
    int cctlid = 0;

    // clone(flags, newsp, ptid, tls, ctid)
    // CLONE_VM(0x100)|FS(0x200)|FILES(0x400)|SIGHAND(0x800)|THREAD(0x10000)
    //   |CHILD_SETTID(0x1000000)|CHILD_CLEARTID(0x200000)
    register long x8 asm("x8") = 220;
    register long x0 asm("x0") = 0xF00 | 0x10000 | 0x200000 | 0x1000000;
    register long x1 asm("x1") = (long)(stack + 131072 - 128);
    register long x2 asm("x2") = 0;                    // ptid
    register long x3 asm("x3") = 0;                    // tls
    register long x4 asm("x4") = (long)&cctlid;        // ctid
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4) : "memory");
    long tid = x0;

    if (tid == 0) {
        // ---- child ----
        long sum = 0;
        for (int i = 0; i < 256; i++) sum += (i & 1) ? i : -i; // = +128 (odd sum 16384, even sum 16256)
        g_result = 10 + (sum == 128 ? 0 : 1000);
        // thread-local exit (93) clears cctlid + FUTEX_WAKEs it
        register long x8c asm("x8") = 93;
        register long x0c asm("x0") = 0;
        asm volatile("svc #0" :: "r"(x8c), "r"(x0c) : "memory");
        return -2;
    }

    // ---- parent: pthread_join blocks on FUTEX_WAIT(ctid) ----
    // FUTEX_WAIT(0) on &cctlid, expecting value == tid (nonzero).
    // The child's clear-tid exit writes 0 + FUTEX_WAKE, so this returns.
    int expect = (int)tid;
    register long x8f asm("x8") = 98; // futex
    register long x0f asm("x0") = (long)&cctlid;
    register long x1f asm("x1") = 0;  // FUTEX_WAIT
    register long x2f asm("x2") = expect;
    register long x3f asm("x3") = 0;  // no timeout
    long fr = 12345;
    asm volatile("svc #0" : "+r"(x0f) : "r"(x8f), "r"(x1f), "r"(x2f), "r"(x3f) : "memory");
    fr = x0f; // 0 = woken

    // After the FUTEX_WAIT returns, the clear-tid word is 0 and the result is set.
    long c_cleared = (cctlid == 0) ? 1 : 0;
    if (fr != 0) return 1000;                 // futex wait interrupted?? -> fail
    if (g_result != 10) return 2000;          // child didn't set result
    if (c_cleared != 1) return 3000;          // clear-tid wasn't zeroed on exit
    return 42;
}
"#;

    let elf = compile(&wd, "join", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "clone-join: entry() -> {v}, expected 42 (futex join / cleartid failed?)"
        ),
        Err(e) => panic!("clone-join: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m clone-join: parent FUTEX_WAIT joined child via CLONE_CHILD_CLEARTID wake");
    let _ = std::fs::remove_dir_all(&wd);
}

/// clone3 (435): the struct-based form modern glibc/bionic use preferentially.
/// The parent passes a struct clone_args (flags/child_tid/parent_tid/stack/tls
/// at their u64 offsets); the child runs, publishes a result, and thread-exits;
/// the parent futex-joins on the child's clear-tid word (CLONE_CHILD_CLEARTID).
#[test]
fn loader_run_clone3_struct_args_spawns_guest_thread() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_clone3: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("clone3");

    let src = r#"
volatile long g_result = 0;
volatile int g_ctlid = 0;

struct clone_args {
    unsigned long flags;       // 0
    unsigned long pidfd;       // 8
    unsigned long child_tid;   // 16
    unsigned long parent_tid;  // 24
    unsigned long exit_signal; // 32
    unsigned long stack;       // 40
    unsigned long stack_size;  // 48
    unsigned long tls;         // 56
};

int entry(void){
    static char stack[131072] __attribute__((aligned(16)));
    int child_tid_slot = 0;

    struct clone_args ca;
    ca.flags       = 0xF00 | 0x10000 | 0x200000 | 0x1000000; // VM|FS|FILES|SIGHAND|THREAD|CLEARTID|CHILDSETTID
    ca.pidfd       = 0;
    ca.child_tid   = (unsigned long)&child_tid_slot;
    ca.parent_tid  = 0;
    ca.exit_signal = 0;
    ca.stack       = (unsigned long)(stack + 131072 - 128);
    ca.stack_size  = 0;
    ca.tls         = 0;

    // syscall 435: clone3(cl_args*, size)
    register long x8 asm("x8") = 435;
    register long x0 asm("x0") = (long)&ca;
    register long x1 asm("x1") = sizeof(struct clone_args);
    long tid;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1) : "memory");
    tid = x0;

    if (tid == 0) {
        // ---- child ----
        int p = 1;
        for (int i = 1; i <= 10; i++) p *= i; // 10! = 3628800
        g_result = 99 + (p == 3628800 ? 0 : 1000);
        register long x8c asm("x8") = 93;
        register long x0c asm("x0") = 0;
        asm volatile("svc #0" :: "r"(x8c), "r"(x0c) : "memory");
        return -2;
    }

    // ---- parent: futex-join on the child's clear-tid word ----
    int expect = (int)tid;
    register long x8f asm("x8") = 98;
    register long x0f asm("x0") = (long)&child_tid_slot;
    register long x1f asm("x1") = 0; // FUTEX_WAIT
    register long x2f asm("x2") = expect;
    register long x3f asm("x3") = 0;
    asm volatile("svc #0" : "+r"(x0f) : "r"(x8f), "r"(x1f), "r"(x2f), "r"(x3f) : "memory");

    if (x0f != 0) return 1000;                 // futex wait interrupted
    if (g_result != 99) return 2000;           // child factorial result wrong/missing
    if (child_tid_slot != 0) return 3000;      // clear-tid not zeroed on exit
    return 42;
}
"#;

    let elf = compile(&wd, "clone3", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "clone3: entry() -> {v}, expected 42 (struct-arg clone spawn/join failed?)"
        ),
        Err(e) => panic!("clone3: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m clone3: struct-arg clone spawned+joined a guest thread (10! via futex join)");
    let _ = std::fs::remove_dir_all(&wd);
}

/// Guest signal delivery — self-delivered handler dispatch. The guest installs a
/// SIGUSR1(10) handler via rt_sigaction(134), then tgkill(131)s its own thread.
/// The handler must run (set a global), and the interrupted tgkill syscall must
/// resume normally on the guest's `ret` (x30 -> SIGRET -> sigreturn) with x0=0.
#[test]
fn loader_run_self_signal_handler_runs_and_resumes() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_self_signal: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("sig-self");

    let src = r#"
volatile long g_hit = 0;
volatile long g_sig = 0;

void on_usr1(int sig){ g_hit = 1; g_sig = sig; }

struct ksa {
    unsigned long handler;   // 0
    unsigned long flags;     // 8
    unsigned long restorer;  // 16
    unsigned char mask[8];   // 24
};

static long my_rt_sigaction(long sig, long act, long oact) {
    register long x8 asm("x8") = 134;
    register long x0 asm("x0") = sig;
    register long x1 asm("x1") = act;
    register long x2 asm("x2") = oact;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_tgkill(long tid, long sig) {
    register long x8 asm("x8") = 131;
    register long x0 asm("x0") = 0;   // tgid (unused by the runtime)
    register long x1 asm("x1") = tid;
    register long x2 asm("x2") = sig;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_gettid(void) {
    register long x8 asm("x8") = 178;
    register long x0 asm("x0") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8) : "memory");
    return x0;
}

int entry(void){
    struct ksa sa = {0};
    struct ksa old = {0};
    sa.handler = (unsigned long)on_usr1;
    long r = my_rt_sigaction(10, (long)&sa, (long)&old);
    if (r != 0) return 1000;              // rt_sigaction failed
    r = my_tgkill(my_gettid(), 10);
    if (r != 0) return 2000;              // self tgkill failed
    if (g_hit != 1) return 3000;          // handler never ran
    if (g_sig != 10) return 4000;         // handler got the wrong signo
    return 42;
}
"#;
    let elf = compile(&wd, "sig-self", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "sig-self: entry() -> {v}, expected 42 (guest signal dispatch failed?)"
        ),
        Err(e) => panic!("sig-self: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m sig-self: self-delivered SIGUSR1 handler ran + resumed");
    let _ = std::fs::remove_dir_all(&wd);
}

/// The SIG_IGN default-disposition path: a guest that installs SIG_IGN for a
/// default-terminating signal (SIGPIPE, 13) and then sends itself that signal
/// must survive (the default would _exit(141)); the runtime honors SIG_IGN.
#[test]
fn loader_run_sig_ign_prevents_termination() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_sig_ign: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("sig-ign");

    let src = r#"
struct ksa {
    unsigned long handler;   // 0
    unsigned long flags;     // 8
    unsigned long restorer;  // 16
    unsigned char mask[8];   // 24
};
static long my_rt_sigaction(long sig, long act, long oact) {
    register long x8 asm("x8") = 134;
    register long x0 asm("x0") = sig;
    register long x1 asm("x1") = act;
    register long x2 asm("x2") = oact;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_tgkill(long tid, long sig) {
    register long x8 asm("x8") = 131;
    register long x0 asm("x0") = 0;
    register long x1 asm("x1") = tid;
    register long x2 asm("x2") = sig;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_gettid(void) {
    register long x8 asm("x8") = 178;
    register long x0 asm("x0") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8) : "memory");
    return x0;
}
int entry(void){
    struct ksa sa = {0};
    sa.handler = 1;                       // SIG_IGN
    long r = my_rt_sigaction(13, (long)&sa, 0); // SIGPIPE -> ignore
    if (r != 0) return 1000;
    r = my_tgkill(my_gettid(), 13);       // would _exit(141) if not ignored
    if (r != 0) return 2000;
    return 42;                            // survived the default-death signal
}
"#;
    let elf = compile(&wd, "sig-ign", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "sig-ign: entry() -> {v}, expected 42 (SIG_IGN default disposition not honored?)"
        ),
        Err(e) => panic!("sig-ign: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m sig-ign: SIG_IGN for SIGPIPE let the guest survive a self-delivered signal");
    let _ = std::fs::remove_dir_all(&wd);
}
/// Cross-thread signal delivery: one guest thread (the PARENT) posts SIGUSR1 to
/// a spawned child via `tgkill`, and the CHILD's dispatcher loop picks it up
/// cooperatively (pending_signal) and runs its installed handler. The handler's
/// effect (g_hit) is observed by the child, which publishes g_result, thread-
/// exits (CLONE_CHILD_CLEARTID wake), and the parent futex-joins. This is the
/// "signal to a specific child thread" delivery a real boot needs for worker
/// threads (render/audio/network) to be interrupted.
#[test]
fn loader_run_cross_thread_signal_delivers_to_child() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_cross_thread_signal: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("sig-xthread");

    let src = r#"
volatile long g_ready = 0;
volatile long g_hit = 0;
volatile long g_result = 0;
volatile int g_ctlid = 0;

void on_usr1(int sig){ g_hit = sig; }

struct ksa { unsigned long h; unsigned long f; unsigned long r; unsigned char m[8]; };

static long my_rt_sigaction(long sig,long act,long oact){
    register long x8 asm("x8")=134; register long x0 asm("x0")=sig;
    register long x1 asm("x1")=act; register long x2 asm("x2")=oact;
    asm volatile("svc #0":"+r"(x0):"r"(x8),"r"(x1),"r"(x2):"memory"); return x0;
}
static long my_tgkill(long tgt,long sig){
    register long x8 asm("x8")=131; register long x0 asm("x0")=0;
    register long x1 asm("x1")=tgt; register long x2 asm("x2")=sig;
    asm volatile("svc #0":"+r"(x0):"r"(x8),"r"(x1),"r"(x2):"memory"); return x0;
}
static long my_yield(void){
    register long x8 asm("x8")=124; register long x0 asm("x0")=0;
    asm volatile("svc #0":"+r"(x0):"r"(x8):"memory"); return x0;
}

int entry(void){
    static char stack[131072] __attribute__((aligned(16)));
    int cctlid = 0;
    // Install the handler (process-wide SIG_ACTIONS; the child sees it too).
    struct ksa sa = {0};
    sa.h = (unsigned long)on_usr1;
    long r = my_rt_sigaction(10, (long)&sa, 0);
    if (r != 0) return 8000;

    // clone(flags, newsp, ptid, tls, ctid)
    register long x8 asm("x8") = 220;
    register long x0 asm("x0") = 0xF00 | 0x10000 | 0x200000 | 0x1000000;
    register long x1 asm("x1") = (long)(stack + 131072 - 128);
    register long x2 asm("x2") = 0;
    register long x3 asm("x3") = 0;
    register long x4 asm("x4") = (long)&cctlid;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4) : "memory");
    long tid = x0;

    if (tid == 0) {
        // ---- child: announce readiness, then spin (yielding each pass so the
        // dispatcher loop re-checks pending_signal) until the cross-thread
        // SIGUSR1 arrives; the handler runs on this child's own thread.
        g_ready = 1;
        while (g_hit == 0) { my_yield(); }
        g_result = (g_hit == 10) ? 5 : 5000;
        register long x8c asm("x8") = 93;
        register long x0c asm("x0") = 0;
        asm volatile("svc #0" :: "r"(x8c), "r"(x0c) : "memory");
        return -2;
    }

    // ---- parent: wait for child readiness, then post the cross-thread signal.
    while (g_ready == 0) {}
    long rr = my_tgkill(tid, 10);

    // futex-join the child (CLONE_CHILD_CLEARTID wake on its exit).
    int expect = (int)tid;
    register long x8f asm("x8") = 98;
    register long x0f asm("x0") = (long)&cctlid;
    register long x1f asm("x1") = 0;  // FUTEX_WAIT
    register long x2f asm("x2") = expect;
    register long x3f asm("x3") = 0;
    asm volatile("svc #0" : "+r"(x0f) : "r"(x8f), "r"(x1f), "r"(x2f), "r"(x3f) : "memory");
    if (x0f != 0) return 7000;             // join failed
    if (rr != 0) return 6000;              // cross-thread tgkill failed
    if (g_result != 5) return g_result;    // child handler effect missing/wrong
    return 42;
}
"#;
    let elf = compile(&wd, "sig-xthread", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "sig-xthread: entry() -> {v}, expected 42 (cross-thread tgkill delivery failed?)"
        ),
        Err(e) => panic!("sig-xthread: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m sig-xthread: parent tgkill delivered SIGUSR1 to the child's own thread");
    let _ = std::fs::remove_dir_all(&wd);
}

/// Real rt_sigprocmask blocking: a guest first BLOCKS SIGUSR1 (so a self
/// tgkill marks it pending but does NOT run the handler), verifies the handler
/// has not run while blocked, then UNBLOCKS SIGUSR1 and verifies the pending
/// signal is delivered immediately (the handler runs once unblocked). This is
/// the Linux "blocked signals stay pending until unmasked" contract the runtime
/// must honor for worker threads that briefly mask signals during critical
/// sections.
#[test]
fn loader_run_sigprocmask_block_then_unblock_delivers_pending() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_sigprocmask: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("sig-procmask");

    let src = r#"
struct ksa {
    unsigned long handler;   // 0
    unsigned long flags;     // 8
    unsigned long restorer;  // 16
    unsigned char mask[8];   // 24
};
volatile long g_hit = 0;
volatile long g_sig = 0;
void on_usr1(int sig){ g_hit = 1; g_sig = sig; }
static long my_rt_sigaction(long sig, long act, long oact) {
    register long x8 asm("x8") = 134;
    register long x0 asm("x0") = sig;
    register long x1 asm("x1") = act;
    register long x2 asm("x2") = oact;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_tgkill(long tid, long sig) {
    register long x8 asm("x8") = 131;
    register long x0 asm("x0") = 0;
    register long x1 asm("x1") = tid;
    register long x2 asm("x2") = sig;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_gettid(void) {
    register long x8 asm("x8") = 178;
    register long x0 asm("x0") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8) : "memory");
    return x0;
}
static long my_rt_sigprocmask(long how, long set, long oset, long sz) {
    register long x8 asm("x8") = 135;
    register long x0 asm("x0") = how;
    register long x1 asm("x1") = set;
    register long x2 asm("x2") = oset;
    register long x3 asm("x3") = sz;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3) : "memory");
    return x0;
}
int entry(void){
    struct ksa sa = {0};
    sa.handler = (unsigned long)on_usr1;
    long r = my_rt_sigaction(10, (long)&sa, 0);      // SIGUSR1 -> handler
    if (r != 0) return 1000;

    // BLOCK SIGUSR1 (SIG_BLOCK=0, set = bit 9 -> sig 10).
    unsigned long block = 1 << (10 - 1);
    r = my_rt_sigprocmask(0, (long)&block, 0, 8);
    if (r != 0) return 2000;

    // Self-deliver SIGUSR1 while blocked: must be marked pending, NOT run yet.
    r = my_tgkill(my_gettid(), 10);
    if (r != 0) return 3000;
    if (g_hit != 0) return 4000;                      // handler MUST NOT have run while blocked

    // UNBLOCK SIGUSR1 (SIG_UNBLOCK=1): the pending signal is now delivered.
    r = my_rt_sigprocmask(1, (long)&block, 0, 8);
    if (r != 0) return 5000;
    if (g_hit != 1) return 6000;                      // pending signal not delivered on unblock
    if (g_sig != 10) return 7000;                     // delivered the wrong signal number
    return 42;
}
"#;
    let elf = compile(&wd, "sig-procmask", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "sig-procmask: entry() -> {v}, expected 42 (block/pending/unblock delivery failed?)"
        ),
        Err(e) => panic!("sig-procmask: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m sig-procmask: blocked SIGUSR1 went pending and was delivered on unblock");
    let _ = std::fs::remove_dir_all(&wd);
}

/// Real POSIX interval-timer -> guest-signal dispatch: a guest installs a
/// SIGALRM handler, creates a timer (timer_create), arms it for a few ms
/// (timer_settime), then spins (yielding each pass) until the handler runs.
/// A host worker thread sleeps the interval and POSTS SIGALRM into the guest's
/// blocked-aware pending model; the dispatcher loop drains it and runs the
/// handler. Before this slice timers forwarded to host POSIX timers whose
/// expiry signal was delivered to the HOST process, never the guest handler.
#[test]
fn loader_run_timer_signal_delivers_sigalm_to_handler() {
    let gcc = match cross_gcc() {
        Some(g) => g,
        None => {
            eprintln!("skipping loader_run_timer_signal: aarch64-linux-gnu-gcc not available");
            return;
        }
    };
    let _ = gcc;
    let _guard = lock_run();
    let wd = workdir("sig-timer");

    let src = r#"
struct ksa {
    unsigned long handler;   // 0
    unsigned long flags;     // 8
    unsigned long restorer;  // 16
    unsigned char mask[8];   // 24
};
volatile long g_hit = 0;
volatile long g_sig = 0;
volatile long g_ticks = 0;
volatile long g_tid = 0;
void on_alrm(int sig){ g_hit = 1; g_sig = sig; g_ticks++; }
static long my_rt_sigaction(long sig, long act, long oact) {
    register long x8 asm("x8") = 134;
    register long x0 asm("x0") = sig;
    register long x1 asm("x1") = act;
    register long x2 asm("x2") = oact;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_timer_create(long sevp, long tid) {
    register long x8 asm("x8") = 107;
    register long x0 asm("x0") = 1;   // CLOCK_MONOTONIC
    register long x1 asm("x1") = sevp;
    register long x2 asm("x2") = tid;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}
static long my_timer_settime(long tid, long newv) {
    register long x8 asm("x8") = 110;
    register long x0 asm("x0") = tid;
    register long x1 asm("x1") = 0;   // flags=0
    register long x2 asm("x2") = newv;
    register long x3 asm("x3") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3) : "memory");
    return x0;
}
static long my_nanosleep(long req) {
    register long x8 asm("x8") = 101;
    register long x0 asm("x0") = req;
    register long x1 asm("x1") = 0;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1) : "memory");
    return x0;
}
int entry(void){
    struct ksa sa = {0};
    sa.handler = (unsigned long)on_alrm;
    long r = my_rt_sigaction(14, (long)&sa, 0);   // SIGALRM -> handler
    if (r != 0) return 1000;

    r = my_timer_create(0, (long)&g_tid);          // default sigevent -> SIGALRM
    if (r != 0 || g_tid == 0) return 2000;

    // itimerspec { it_value = { .1s, 0 }, it_interval = { .1s, 0 } } (periodic).
    unsigned long long its[2];
    its[0] = 0;              // it_value.tv_sec
    its[1] = 100000000;      // it_value.tv_nsec = 100ms
    its[2] = 0;              // it_interval.tv_sec
    its[3] = 100000000;      // it_interval.tv_nsec = 100ms
    r = my_timer_settime(g_tid, (long)its);
    if (r != 0) return 3000;

    // Spin up to ~2s, yielding so the dispatcher drains pending signals; the
    // timer worker posts SIGALRM to us which the handler turns into ticks.
    struct { long sec; long nsec; } ts;
    ts.sec = 0; ts.nsec = 1000000;   // 1ms yield
    long guard = 0;
    while (g_ticks < 2 && guard < 20000) { guard++; my_nanosleep((long)&ts); }
    if (g_ticks < 2) return 4000;     // handler never ticked -> timer not dispatched
    if (g_sig != 14) return 5000;     // wrong signal
    return 42;
}
"#;
    let elf = compile(&wd, "sig-timer", src);
    match run_elf(&elf) {
        Ok(v) => assert_eq!(
            v, 42,
            "sig-timer: entry() -> {v}, expected 42 (timer signal never reached the guest handler?)"
        ),
        Err(e) => panic!("sig-timer: jit_run failed: {e}"),
    }
    eprintln!("\x1b[32mPASS\x1b[0m sig-timer: periodic timer delivered SIGALRM to the guest handler");
    let _ = std::fs::remove_dir_all(&wd);
}
