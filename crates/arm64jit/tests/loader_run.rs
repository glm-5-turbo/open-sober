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

    let (_nbound, _unresolved) = arm64jit::plt::bind_image_plt(&el);

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
    let _guard = run_lock().lock().unwrap();
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
    let _guard = run_lock().lock().unwrap();
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
    let _guard = run_lock().lock().unwrap();
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
