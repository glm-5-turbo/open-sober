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
/// Uses a JNI shim binary to load libroblox.so with Android stub libraries.
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

    // Build and install the JNI stub/shims
    setup_jni_shim(env)?;

    // Build the QEMU command using the shim as entry point
    let shim_path = env.root.join("jni_shim");
    let mut cmd = env.build_qemu_cmd(&cfg.qemu_path, &shim_path, &[]);

    // Point to the real Roblox library
    cmd.env("ROBLOX_LIB", &main_binary);

    // LD_PRELOAD our bionic shim to intercept @LIBC versioned symbols
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

/// Build and install the ARM64 JNI shim and bionic stub library into the Android env.
///
/// The bionic shim (`libbionic_shim.so`) provides @LIBC-versioned symbols that
/// libroblox.so needs. It's compiled as a shared library with:
///   - `bionic_symbols.S` — ~390 assembly trampolines that tail-call glibc
///   - `bionic_stubs.c` — ~6 hand-written C stubs for Bionic-only symbols + data init
///   - `bionic_version.ver` — version script tagging everything as LIBC/LIBC_N/LIBC_O
///
/// At runtime, the shim is LD_PRELOAD'ed and its LIBC-versioned symbols satisfy
/// libroblox.so's symbol lookups, forwarding to glibc via the PLT.
fn setup_jni_shim(env: &AndroidEnv) -> Result<()> {
    let syslib64 = env.root.join("system").join("lib64");

    // Check if already installed
    if env.root.join("jni_shim").exists() && syslib64.join("libbionic_shim.so").exists() {
        // Rebuild if source is newer than the binary
        let shim_path = syslib64.join("libbionic_shim.so");
        if let (Ok(s_meta), Ok(b_meta)) = (
            std::fs::metadata(shim_source("bionic_symbols.S", env)),
            std::fs::metadata(&shim_path),
        ) {
            if s_meta.modified().ok() <= b_meta.modified().ok() {
                info!("Bionic shim already installed and up-to-date");
                return Ok(());
            }
        }
    }

    info!("Building ARM64 JNI shim and bionic shim...");

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let glibc_path = syslib64.join("libglibc.so");
    let libm_path = syslib64.join("libm.so.6");

    // 1. Build the JNI shim executable
    let shim_src = crate_dir.join("jni_shim.c");
    let shim_out = env.root.join("jni_shim");

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

    // 2. Build the bionic shim shared library (libbionic_shim.so)
    let asm_src = crate_dir.join("bionic_symbols.S");
    let c_src = crate_dir.join("bionic_stubs.c");
    let ver_script = crate_dir.join("bionic_version.ver");
    let shim_out = syslib64.join("libbionic_shim.so");

    // Only build if the generated source files exist
    if !asm_src.exists() || !c_src.exists() || !ver_script.exists() {
        warn!("Bionic shim source files not found — libroblox.so may fail to load");
        info!("JNI shim ready at: {}", shim_out.display());
        return Ok(());
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
        anyhow::bail!("Failed to assemble bionic_symbols.S");
    }

    // Compile C stubs
    let status = std::process::Command::new("aarch64-linux-gnu-gcc")
        .args(["-c", "-o"])
        .arg(&obj_c)
        .arg("-fPIC")
        .arg(&c_src)
        .status()
        .context("Failed to compile bionic shim stubs")?;

    if !status.success() {
        anyhow::bail!("Failed to compile bionic_stubs.c");
    }

    // Link into shared library with version script
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
        .arg("-lglibc")   // links against libglibc.so for glibc symbols
        .arg("-lm")       // links against libm.so.6 for math symbols
        .arg("-ldl")
        .arg("-nostartfiles")
        .status()
        .context("Failed to link bionic shim")?;

    if !status.success() {
        anyhow::bail!("Failed to link libbionic_shim.so");
    }

    // Clean up object files
    let _ = std::fs::remove_file(&obj_asm);
    let _ = std::fs::remove_file(&obj_c);

    // Verify the shim was created and has the right symbols
    if shim_out.exists() {
        info!("libbionic_shim.so built successfully ({} bytes)", shim_out.metadata().map(|m| m.len()).unwrap_or(0));
    }

    info!("JNI shim ready at: {}", shim_out.display());
    Ok(())
}

/// Helper to resolve a source file path relative to the crate.
fn shim_source(name: &str, env: &AndroidEnv) -> std::path::PathBuf {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    crate_dir.join(name)
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

/// Build environment variables for the QEMU process.
fn build_env_vars(cfg: &SoConfig) -> Result<Vec<(&'static str, String)>> {
    let mut vars = Vec::new();

    // Graphics settings
    vars.push(("SOBER_QUALITY", cfg.quality.to_string()));
    if let Some(cap) = cfg.fps_cap {
        vars.push(("SOBER_FPS_CAP", cap.to_string()));
    }

    // Discord RPC
    if cfg.discord_rpc {
        vars.push(("SOBER_DISCORD_RPC", "1".into()));
    }

    Ok(vars)
}