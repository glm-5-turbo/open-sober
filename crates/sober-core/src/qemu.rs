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

    // LD_PRELOAD our stub library for missing Android/Bionic symbols
    cmd.env("LD_PRELOAD", "/system/lib64/libcxx_syms.so");

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

/// Build and install the ARM64 JNI shim and stub libraries into the Android env.
fn setup_jni_shim(env: &AndroidEnv) -> Result<()> {
    let syslib64 = env.root.join("system").join("lib64");

    // Check if already installed
    if env.root.join("jni_shim").exists() && syslib64.join("libcxx_syms.so").exists() {
        info!("JNI shim already installed");
        return Ok(());
    }

    info!("Building ARM64 JNI shim and stubs...");

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

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

    // 2. Build the stub symbols library
    let stubs_src = crate_dir.join("symbols_aarch64.c");
    let stubs_out = syslib64.join("libcxx_syms.so");

    if stubs_src.exists() {
        let status = std::process::Command::new("aarch64-linux-gnu-gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-o")
            .arg(&stubs_out)
            .arg(&stubs_src)
            .status()?;

        if !status.success() {
            warn!("Stub library compilation failed (continuing anyway)");
        }
    }

    info!("JNI shim ready at: {}", shim_out.display());
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