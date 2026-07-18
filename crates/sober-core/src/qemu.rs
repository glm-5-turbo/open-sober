// SPDX-License-Identifier: MIT
//
// QEMU user-mode launcher for running ARM64 Android binaries on x86-64.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use crate::android_env::AndroidEnv;
use crate::config::SoConfig;

/// Launch the Roblox Android APK via QEMU user-mode translation.
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

    // Find the main Roblox binary (libroblox.so or similar)
    let main_binary = find_main_binary(&libs)?;
    info!("Main Roblox binary: {}", main_binary.display());

    // Build environment variables
    let env_vars = build_env_vars(cfg)?;

    // Build the QEMU command
    let mut cmd = env.build_qemu_cmd(&cfg.qemu_path, &main_binary, &env_vars);

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

    info!("Launching: {:?}", cmd);
    debug!("Full command: {:?}", cmd);

    // Spawn and wait
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
        // Non-zero exit isn't necessarily a failure for games
        Ok(())
    }
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

/// Check if QEMU user-mode is available.
pub fn check_qemu_available(qemu_path: &Path) -> bool {
    Command::new(qemu_path)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Spawn an ARM64 binary under QEMU user-mode with the Android environment.
pub fn spawn_qemu_process(
    qemu_path: &Path,
    android_root: &Path,
    binary: &Path,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<Child> {
    let mut cmd = Command::new(qemu_path);

    cmd.arg("-L");
    cmd.arg(android_root);
    cmd.arg("-E");
    cmd.arg("ANDROID_ROOT=/system");
    cmd.arg("-E");
    cmd.arg("ANDROID_DATA=/data");
    cmd.arg("-E");
    cmd.arg("ANDROID_STORAGE=/storage");

    for (key, val) in env_vars {
        cmd.arg("-E");
        cmd.arg(format!("{}={}", key, val));
    }

    cmd.arg(binary);
    cmd.args(args);

    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    cmd.spawn().context("Failed to spawn QEMU process")
}