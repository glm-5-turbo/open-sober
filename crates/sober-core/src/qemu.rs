// SPDX-License-Identifier: MIT
//
// QEMU user-mode launcher for running ARM64 Android binaries on x86-64.

use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::android_env::AndroidEnv;
use crate::config::SoConfig;

/// Launch the Roblox Android APK via QEMU user-mode translation.
/// Uses a JNI shim binary to load libroblox.so with the bionic shim.
pub fn launch_roblox(
    apk_path: &Path,
    env: &AndroidEnv,
    _token: &str,
    cfg: &SoConfig,
    _place_id: Option<u64>,
) -> Result<()> {
    info!("Preparing to launch Roblox via QEMU user-mode");

    // Extract native libraries from the APK
    let lib_dir = crate::dirs_setup::libs_dir();
    let libs = crate::apk::extract_libs(apk_path, &lib_dir)
        .context("Failed to extract native libraries from APK")?;

    if libs.is_empty() {
        anyhow::bail!("No ARM64 native libraries found in the APK");
    }

    // Find the main Roblox binary
    let main_binary = find_main_binary(&libs)?;
    info!("Main Roblox binary: {}", main_binary.display());

    // Build and install the JNI shim + bionic shim
    setup_jni_shim(env)?;
    setup_bionic_shim(env)?;

    // Build the QEMU command using the shim as entry point
    let shim_path = env.root.join("jni_shim");
    let mut cmd = env.build_qemu_cmd(&cfg.qemu_path, &shim_path, &[]);

    // Point to the real Roblox library
    cmd.env("ROBLOX_LIB", &main_binary);

    // LD_PRELOAD our bionic shim to intercept @LIBC-versioned symbols
    cmd.env("LD_PRELOAD", "/system/lib64/libbionic_shim.so");

    // Put libroblox.so's directory on the library path
    if let Some(lib_dir) = main_binary.parent() {
        let ld_path = format!(
            "/system/lib64:/vendor/lib64:/lib:{}",
            lib_dir.display()
        );
        cmd.env("LD_LIBRARY_PATH", &ld_path);
    }

    // Set up graphics
    if let Some(driver) = cfg.mesa_loader_override() {
        cmd.env("MESA_LOADER_DRIVER_OVERRIDE", driver);
    }
    if cfg.use_opengl {
        cmd.env("MESA_GL_VERSION_OVERRIDE", "3.3");
    }

    // Set display
    cmd.env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into()));
    cmd.env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_default());

    // Configure process
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.stdin(Stdio::inherit());

    info!("Launching Roblox under QEMU with JNI shim...");
    debug!("Full command: {:?}", cmd);

    let mut child = cmd.spawn()
        .context("Failed to start QEMU process. Is qemu-aarch64 installed?")?;

    info!("Roblox started (PID: {})", child.id());

    let status = child.wait()
        .context("Failed to wait for QEMU process")?;

    if status.success() {
        info!("Roblox exited successfully");
        Ok(())
    } else {
        warn!("Roblox exited with status: {:?}", status.code());
        Ok(())
    }
}

/// Build and install the ARM64 JNI shim executable.
///
/// The JNI shim is a minimal entry point that:
/// 1. Initializes a fake JavaVM / JNIEnv with stub function tables
/// 2. Loads libroblox.so via dlopen()
/// 3. Calls JNI_OnLoad()
/// 4. Lets the game run
fn setup_jni_shim(env: &AndroidEnv) -> Result<()> {
    let shim_out = env.root.join("jni_shim");

    // Check if already installed and up to date
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let shim_src = crate_dir.join("jni_shim.c");
    if shim_out.exists() {
        if let (Ok(s_meta), Ok(b_meta)) = (
            std::fs::metadata(&shim_src),
            std::fs::metadata(&shim_out),
        ) {
            if s_meta.modified().ok() <= b_meta.modified().ok() {
                info!("JNI shim already installed and up-to-date");
                return Ok(());
            }
        }
    }

    info!("Building ARM64 JNI shim...");

    let status = std::process::Command::new("aarch64-linux-gnu-gcc")
        .arg("-o")
        .arg(&shim_out)
        .arg(&shim_src)
        .arg("-ldl")
        .status()
        .context("Failed to compile JNI shim (aarch64-linux-gnu-gcc required)")?;

    if !status.success() {
        anyhow::bail!("JNI shim compilation failed");
    }

    info!("JNI shim ready at: {}", shim_out.display());
    Ok(())
}

/// Build the bionic shim shared library (`libbionic_shim.so`).
///
/// The bionic shim provides @LIBC-versioned symbols that libroblox.so needs.
/// It is built from:
///   - `bionic_shim.S` — ~392 assembly trampolines that lazily resolve to glibc
///   - `bionic_init.c` — ~5 Bionic-only C stubs + lazy dispatch resolver
///   - `bionic_version.ver` — version script tagging everything as LIBC/LIBC_N/LIBC_O
///
/// At runtime, it's LD_PRELOAD'ed and its LIBC-versioned symbols satisfy
/// libroblox.so's symbol lookups, forwarding to glibc via the trampoline table.
fn setup_bionic_shim(env: &AndroidEnv) -> Result<()> {
    let syslib64 = env.root.join("system").join("lib64");
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let asm_src = crate_dir.join("bionic_shim.S");
    let c_src = crate_dir.join("bionic_init.c");
    let ver_script = crate_dir.join("bionic_version.ver");
    let shim_out = syslib64.join("libbionic_shim.so");

    // Check if source files exist
    if !asm_src.exists() || !c_src.exists() || !ver_script.exists() {
        warn!("Bionic shim source files not found — build may be incomplete");
        return Ok(());
    }

    // Check if already built and up to date
    if shim_out.exists() {
        // Rebuild if any source file is newer than the binary
        let sources = [&asm_src, &c_src, &ver_script];
        let need_rebuild = sources.iter().any(|src| {
            src.metadata()
                .and_then(|m| m.modified())
                .ok()
                .zip(shim_out.metadata().and_then(|m| m.modified()).ok())
                .map(|(s_mtime, b_mtime)| s_mtime > b_mtime)
                .unwrap_or(true)
        });

        if !need_rebuild {
            info!("libbionic_shim.so already installed and up-to-date");
            return Ok(());
        }
    }

    info!("Building libbionic_shim.so (Bionic→glibc symbol bridge)...");

    let obj_asm = syslib64.join("bionic_symbols.o");
    let obj_c = syslib64.join("bionic_stubs.o");

    // Assemble trampolines
    let status = std::process::Command::new("aarch64-linux-gnu-gcc")
        .args(["-c", "-o"])
        .arg(&obj_asm)
        .arg(&asm_src)
        .status()
        .context("Failed to assemble bionic shim trampolines")?;

    if !status.success() {
        anyhow::bail!("Failed to assemble bionic_shim.S");
    }

    // Compile C stubs
    let status = std::process::Command::new("aarch64-linux-gnu-gcc")
        .args(["-c", "-o"])
        .arg(&obj_c)
        .arg("-fPIC")
        .arg(&c_src)
        .status()
        .context("Failed to compile bionic_init.c")?;

    if !status.success() {
        anyhow::bail!("Failed to compile bionic_init.c");
    }

    // Link into shared library with version script.
    // We link against libglibc.so (a cross-compiled copy of glibc for ARM64)
    // so the trampolines' dlsym() calls can resolve glibc symbols at runtime.
    let status = std::process::Command::new("aarch64-linux-gnu-gcc")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-o")
        .arg(&shim_out)
        .arg(&obj_asm)
        .arg(&obj_c)
        .arg("-Wl,--version-script")
        .arg(&ver_script)
        .arg("-Wl,-rpath,/system/lib64")
        .arg("-L")
        .arg(&syslib64)
        .arg(&format!("-Wl,-rpath,{}", syslib64.display()))
        .arg("-lglibc")   // cross-compiled glibc for ARM64
        .arg("-lm")       // math library
        .arg("-ldl")      // dlopen/dlsym
        .arg("-nostartfiles")  // no _start needed — this is a shim library
        .status()
        .context("Failed to link libbionic_shim.so")?;

    if !status.success() {
        anyhow::bail!("Failed to link libbionic_shim.so");
    }

    // Clean up object files
    let _ = std::fs::remove_file(&obj_asm);
    let _ = std::fs::remove_file(&obj_c);

    // Verify the shim was created
    if shim_out.exists() {
        info!("libbionic_shim.so built successfully ({} bytes)",
            shim_out.metadata().map(|m| m.len()).unwrap_or(0));
    }

    Ok(())
}

/// Find the main Roblox shared library among extracted libs.
fn find_main_binary(libs: &[std::path::PathBuf]) -> Result<std::path::PathBuf> {
    // Order of preference for main Roblox binary
    let candidates = [
        "libroblox.so",
        "librbx.so",
        "libmain.so",
    ];

    for candidate in &candidates {
        for lib in libs {
            if lib.file_name().and_then(|n| n.to_str()) == Some(candidate) {
                return Ok(lib.clone());
            }
        }
    }

    // Fallback: return the first .so found
    libs.first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No native libraries found in APK"))
}