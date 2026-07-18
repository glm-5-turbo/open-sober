// SPDX-License-Identifier: MIT
//
// Android runtime environment setup. Creates the directory structure
// and filesystem that the Roblox Android binary expects.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use tracing::{info, warn};

/// Android environment root.
pub struct AndroidEnv {
    /// Root directory of the Android environment
    pub root: PathBuf,
    /// Path to the android2gnulinux runtime
    #[allow(dead_code)]
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
        if runtime.join("libdl.so").exists() {
            let system_lib = root.join("system").join("lib64");
            std::fs::create_dir_all(&system_lib)?;
            std::fs::copy(runtime.join("libdl.so"), system_lib.join("libdl.so"))?;
            info!("Copied android2gnulinux libdl.so to {}", system_lib.display());
        }
        // Also copy any prebuilt runtime libs
        if runtime.join("lib").exists() {
            let system_lib = root.join("system").join("lib64");
            Self::copy_dir(&runtime.join("lib"), &system_lib)?;
        }

        info!("Android environment ready at: {}", root.display());
        Ok(Self { root, runtime })
    }

    /// Build or find android2gnulinux runtime.
    fn find_or_build_runtime() -> Result<PathBuf> {
        let vendor_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("vendor")
            .join("android2gnulinux");

        if vendor_path.exists() {
            let runtime = vendor_path.join("runtime");
            let libdl = runtime.join("libdl.so");

            // Check if already built
            if libdl.exists() {
                info!("android2gnulinux runtime already built");
                return Ok(runtime);
            }

            // Build directly (Makefile is broken for modern GCC)
            info!("Building android2gnulinux runtime...");
            std::fs::create_dir_all(&runtime)?;

            let status = Command::new("cc")
                .args([
                    "-fPIC", "-shared",
                    "-Isrc",
                    "-D_GNU_SOURCE",
                    "-DANDROID_X86_LINKER",
                    "-DLINKER_DEBUG=0",
                    "-DRUNTIMEPATH=\"\"",
                    "src/wrapper/wrapper.c",
                    "src/linker/dlfcn.c",
                    "src/linker/linker.c",
                    "src/linker/linker_environ.c",
                    "src/linker/rt.c",
                    "src/linker/strlcpy.c",
                    "-ldl", "-lpthread",
                    "-o", "runtime/libdl.so",
                ])
                .current_dir(&vendor_path)
                .status()
                .context("Failed to compile android2gnulinux")?;

            if status.success() && libdl.exists() {
                info!("android2gnulinux built successfully");
                return Ok(runtime);
            }

            warn!("android2gnulinux build failed — continuing without it");
        }

        let fallback = PathBuf::from("/usr/lib/android2gnulinux");
        if fallback.exists() {
            return Ok(fallback);
        }

        warn!("android2gnulinux runtime not found — Bionic→glibc compat will be limited");
        Ok(PathBuf::from("/tmp/open-sober-no-android-runtime"))
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