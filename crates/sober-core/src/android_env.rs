// SPDX-License-Identifier: MIT
//
// Android runtime environment setup. Creates the directory structure
// and filesystem that the Roblox Android binary expects.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use std::os::unix::fs::PermissionsExt;
use tracing::{info, warn};

/// Android environment root.
pub struct AndroidEnv {
    /// Root directory of the Android environment
    pub root: PathBuf,
    /// Path to the android2gnulinux runtime
    pub runtime: PathBuf,
}

impl AndroidEnv {
    /// Set up the Android runtime environment.
    ///
    /// Creates the directory structure and symlinks that the Roblox Android
    /// binary expects to find at runtime.
    pub fn setup() -> Result<Self> {
        let root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("open-sober")
            .join("android-env");

        let runtime = Self::find_or_build_runtime()?;

        info!("Setting up Android environment at: {}", root.display());

        // Create Android directory structure
        let dirs = [
            "data/data/com.roblox.client",
            "data/data/com.roblox.client/cache",
            "data/data/com.roblox.client/files",
            "data/data/com.roblox.client/databases",
            "data/data/com.roblox.client/shared_prefs",
            "data/data/com.roblox.client/no_backup",
            "data/app/com.roblox.client",
            "data/local/tmp",
            "sdcard/Android/data/com.roblox.client",
            "sdcard/Android/obb/com.roblox.client",
            "sdcard/Pictures",
            "sdcard/Download",
            "system/lib",
            "system/lib64",
            "vendor/lib",
            "vendor/lib64",
            "proc",
            "sys",
            "dev",
        ];

        for dir in &dirs {
            let p = root.join(dir);
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Failed to create {}", p.display()))?;
        }

        // Create symlinks for storage
        let sdcard_link = root.join("sdcard");
        let storage_link = root.join("storage");
        if !storage_link.exists() {
            std::os::unix::fs::symlink(&sdcard_link, &storage_link)?;
        }

        // Create /dev/null and friends
        let dev_null = root.join("dev").join("null");
        if !dev_null.exists() {
            std::fs::write(&dev_null, b"")?;
        }

        let dev_zero = root.join("dev").join("zero");
        if !dev_zero.exists() {
            std::fs::write(&dev_zero, b"")?;
        }

        // Copy android2gnulinux runtime libraries if available
        if runtime.join("lib").exists() {
            let runtime_lib = runtime.join("lib");
            let system_lib = root.join("system").join("lib64");
            Self::copy_dir(&runtime_lib, &system_lib)?;
            info!("Copied android2gnulinux runtime libraries to {}", system_lib.display());
        }

        info!("Android environment ready at: {}", root.display());
        Ok(Self { root, runtime })
    }

    /// Build or find android2gnulinux runtime.
    fn find_or_build_runtime() -> Result<PathBuf> {
        // Check the vendor submodule
        let vendor_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("vendor")
            .join("android2gnulinux");

        if vendor_path.exists() {
            // Check if it's already built
            let built = vendor_path.join("runtime");
            if built.join("lib").exists() {
                return Ok(built);
            }

            // Try to build it
            info!("Building android2gnulinux runtime...");
            let status = Command::new("make")
                .args(["-C", &vendor_path.to_string_lossy()])
                .status()
                .context("Failed to run make for android2gnulinux")?;

            if status.success() {
                if built.join("lib").exists() {
                    return Ok(built);
                }
            } else {
                warn!("android2gnulinux build failed — continuing without it");
            }
        }

        // Fallback: use system installed path or none
        let fallback = PathBuf::from("/usr/lib/android2gnulinux");
        if fallback.exists() {
            return Ok(fallback);
        }

        warn!("android2gnulinux runtime not found — Bionic→glibc compat will be limited");
        Ok(fallback)
    }

    /// Build the QEMU command for launching an ARM64 binary.
    pub fn build_qemu_cmd(
        &self,
        qemu_path: &Path,
        binary_path: &Path,
        env_vars: &[(&str, String)],
    ) -> Command {
        let mut cmd = Command::new(qemu_path);

        // QEMU user mode options
        cmd.arg("-L"); // dynamic linker path
        cmd.arg(&self.root);
        cmd.arg("-E"); // set env var
        cmd.arg("LD_LIBRARY_PATH=/system/lib64:/vendor/lib64:/data/app/com.roblox.client/lib/arm64");

        // Android environment vars
        cmd.arg("-E");
        cmd.arg("ANDROID_ROOT=/system");
        cmd.arg("-E");
        cmd.arg("ANDROID_DATA=/data");
        cmd.arg("-E");
        cmd.arg("ANDROID_STORAGE=/storage");
        cmd.arg("-E");
        cmd.arg("EXTERNAL_STORAGE=/sdcard");

        // Pass through environment variables
        for (key, val) in env_vars {
            cmd.arg("-E");
            cmd.arg(format!("{}={}", key, val));
        }

        // Drop QEMU's own env vars cleanly
        cmd.arg("-drop-ld-preload");

        // The binary to execute
        cmd.arg(binary_path);

        cmd
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry?;
            let relative = entry.path().strip_prefix(src)?;
            let target = dst.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else if entry.file_type().is_file() {
                std::fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }
}

/// Setup an Android environment suitable for use with QEMU user mode.
/// Creates the chroot filesystem with the expected Android paths.
pub fn setup_android_env(root: &Path) -> Result<()> {
    let env = AndroidEnv::setup()?;

    // Create a wrapper script that can be used directly
    let wrapper_path = root.join("run-in-android-env.sh");
    let wrapper = format!(
        r#"#!/bin/bash
# Open Sober - Android environment wrapper
# Usage: ./run-in-android-env.sh <qemu-aarch64> <binary> [args...]

QEMU="$1"
shift
BINARY="$1"
shift

ROOT="{}"
RUNTIME="{}"

exec "$QEMU" \
    -L "$ROOT" \
    -E LD_LIBRARY_PATH="/system/lib64:/vendor/lib64" \
    -E ANDROID_ROOT="/system" \
    -E ANDROID_DATA="/data" \
    -E ANDROID_STORAGE="/storage" \
    -E EXTERNAL_STORAGE="/sdcard" \
    "$BINARY" "$@"
"#,
        env.root.display(),
        env.runtime.display(),
    );

    std::fs::write(&wrapper_path, wrapper)?;
    std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755))?;

    info!("Wrapper script created at: {}", wrapper_path.display());
    Ok(())
}