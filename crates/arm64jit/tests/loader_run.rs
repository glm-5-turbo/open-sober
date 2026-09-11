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
    st.tpidr = tls.as_ptr() as u64;

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