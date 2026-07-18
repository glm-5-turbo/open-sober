// SPDX-License-Identifier: MIT
//
// Android runtime initialization — creates the Android directory structure
// and symlinks needed by the Roblox Android binary.
//
// The Android binary expects a specific filesystem layout including:
// - /data/data/com.roblox.client/ — app data directory
// - /storage/ and /sdcard/ — storage symlinks
// - /system/lib, /vendor/lib — library paths
// - /system/bin, /system/xbin — binary paths
// - /data/local/tmp — temporary directory
// - /data/dalvik-cache — cache directory

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{debug, info};

/// The Roblox Android package name.
const ROBLOX_PACKAGE: &str = "com.roblox.client";

/// Set up the Android-specific directory layout under the chroot root.
///
/// This creates the directories and symlinks that the Roblox Android
/// binary expects to find in its runtime environment.
pub fn setup_android_layout(root_path: &Path) -> Result<()> {
    debug!("Setting up Android layout at {}", root_path.display());

    // /data hierarchy
    setup_data_directory(root_path)?;

    // /system hierarchy
    setup_system_directory(root_path)?;

    // /vendor hierarchy
    setup_vendor_directory(root_path)?;

    // /storage and /sdcard
    setup_storage_symlinks(root_path)?;

    // /mnt
    setup_mnt_directory(root_path)?;

    // /acct
    setup_acct_directory(root_path)?;

    // /config
    setup_config_directory(root_path)?;

    // /d (for /proc/self/fd compatibility)
    setup_d_directory(root_path)?;

    // /cache
    setup_cache_directory(root_path)?;

    // /metadata
    setup_metadata_directory(root_path)?;

    info!("Android layout set up at {}", root_path.display());
    Ok(())
}

/// Create /data/ directory structure.
///
/// This includes:
/// - /data/data/ — app data root
/// - /data/data/com.roblox.client/ — Roblox app data
/// - /data/data/com.roblox.client/files/ — app files
/// - /data/data/com.roblox.client/cache/ — app cache
/// - /data/data/com.roblox.client/lib/ — app native libraries
/// - /data/local/tmp/ — temporary directory
/// - /data/dalvik-cache/ — (optional) Dalvik/ART cache
fn setup_data_directory(root_path: &Path) -> Result<()> {
    let data = root_path.join("data");
    ensure_dir_android(&data, 0o755)?;

    let data_data = data.join("data");
    ensure_dir_android(&data_data, 0o755)?;

    // Roblox app data directory
    let roblox_dir = data_data.join(ROBLOX_PACKAGE);
    ensure_dir_android(&roblox_dir, 0o755)?;
    ensure_dir_android(&roblox_dir.join("files"), 0o755)?;
    ensure_dir_android(&roblox_dir.join("cache"), 0o755)?;
    ensure_dir_android(&roblox_dir.join("code_cache"), 0o755)?;
    ensure_dir_android(&roblox_dir.join("lib"), 0o755)?;
    ensure_dir_android(&roblox_dir.join("shared_prefs"), 0o755)?;
    ensure_dir_android(&roblox_dir.join("databases"), 0o755)?;

    // /data/local/tmp — world-writable temp directory
    let local = data.join("local");
    ensure_dir_android(&local, 0o755)?;
    let tmp = local.join("tmp");
    ensure_dir_android(&tmp, 0o1777)?;

    // /data/dalvik-cache — ART cache
    let dalvik = data.join("dalvik-cache");
    ensure_dir_android(&dalvik, 0o755)?;

    debug!("Created /data directory structure");
    Ok(())
}

/// Create /system/ directory structure.
///
/// This mirrors the Android system partition:
/// - /system/lib/ — 32-bit native libraries
/// - /system/lib64/ — 64-bit native libraries
/// - /system/bin/ — executables
/// - /system/xbin/ — extra executables
/// - /system/etc/ — configuration files
/// - /system/fonts/ — system fonts
/// - /system/framework/ — framework jars
fn setup_system_directory(root_path: &Path) -> Result<()> {
    let system = root_path.join("system");
    ensure_dir_android(&system, 0o755)?;

    // Library directories
    let lib32 = system.join("lib");
    ensure_dir_android(&lib32, 0o755)?;

    let lib64 = system.join("lib64");
    ensure_dir_android(&lib64, 0o755)?;

    // Binaries
    ensure_dir_android(&system.join("bin"), 0o755)?;
    ensure_dir_android(&system.join("xbin"), 0o755)?;

    // Configuration
    let etc = system.join("etc");
    ensure_dir_android(&etc, 0o755)?;

    // Create a minimal /system/build.prop so Android runtime doesn't crash
    let build_prop = etc.join("build.prop");
    if !build_prop.exists() {
        let prop_content = "ro.build.version.sdk=31\n\
             ro.build.version.release=12\n\
             ro.product.cpu.abi=x86_64\n\
             ro.product.cpu.abilist=x86_64,x86\n\
             ro.product.cpu.abilist64=x86_64\n\
             ro.product.cpu.abilist32=x86\n\
             dalvik.vm.isa.x86_64=x86_64\n\
             ro.build.date=Mon Jan 1 00:00:00 UTC 2024\n\
             ro.build.id=SOBER\n\
             ro.build.display.id=sober-1.0\n".to_string();
        fs::write(&build_prop, prop_content.as_bytes())
            .context("Failed to write /system/etc/build.prop")?;
        fs::set_permissions(&build_prop, fs::Permissions::from_mode(0o644))
            .ok();
    }

    // Fonts
    ensure_dir_android(&system.join("fonts"), 0o755)?;

    // Framework
    ensure_dir_android(&system.join("framework"), 0o755)?;

    // /system/usr — keylayout etc
    ensure_dir_android(&system.join("usr"), 0o755)?;
    ensure_dir_android(&system.join("usr/keylayout"), 0o755)?;
    ensure_dir_android(&system.join("usr/keychars"), 0o755)?;

    // /system/app — system apps
    ensure_dir_android(&system.join("app"), 0o755)?;

    debug!("Created /system directory structure");
    Ok(())
}

/// Create /vendor/ directory structure.
///
/// Mirrors the Android vendor partition for hardware-specific libraries.
fn setup_vendor_directory(root_path: &Path) -> Result<()> {
    let vendor = root_path.join("vendor");
    ensure_dir_android(&vendor, 0o755)?;
    ensure_dir_android(&vendor.join("lib"), 0o755)?;
    ensure_dir_android(&vendor.join("lib64"), 0o755)?;
    ensure_dir_android(&vendor.join("etc"), 0o755)?;
    ensure_dir_android(&vendor.join("firmware"), 0o755)?;

    debug!("Created /vendor directory structure");
    Ok(())
}

/// Create /storage/ and /sdcard/ symlinks.
///
/// Android apps expect /storage/emulated/0 and /sdcard to point
/// to expandable storage. We make them point to a directory
/// inside the chroot.
fn setup_storage_symlinks(root_path: &Path) -> Result<()> {
    let storage = root_path.join("storage");
    ensure_dir_android(&storage, 0o755)?;

    // /storage/emulated
    let emulated = storage.join("emulated");
    ensure_dir_android(&emulated, 0o755)?;

    // /storage/emulated/0 — primary external storage
    let sdcard0 = emulated.join("0");
    ensure_dir_android(&sdcard0, 0o755)?;
    ensure_dir_android(&sdcard0.join("Android"), 0o755)?;
    ensure_dir_android(&sdcard0.join("Android/data"), 0o755)?;
    ensure_dir_android(&sdcard0.join("Android/obb"), 0o755)?;
    ensure_dir_android(&sdcard0.join("Download"), 0o755)?;
    ensure_dir_android(&sdcard0.join("DCIM"), 0o755)?;
    ensure_dir_android(&sdcard0.join("Documents"), 0o755)?;

    // /sdcard -> /storage/emulated/0
    let sdcard_path = root_path.join("sdcard");
    if !sdcard_path.exists() {
        if let Err(e) = std::os::unix::fs::symlink("/storage/emulated/0", &sdcard_path) {
            // If it already exists (race), that's fine
            if e.raw_os_error() != Some(17) {
                return Err(e).context("Failed to symlink /sdcard")?;
            }
        }
    }

    // /mnt/sdcard -> /storage/emulated/0
    let mnt_sdcard = root_path.join("mnt/sdcard");
    ensure_dir_android(&root_path.join("mnt"), 0o755)?;
    if !mnt_sdcard.exists() {
        if let Err(e) = std::os::unix::fs::symlink("/storage/emulated/0", &mnt_sdcard) {
            // If it already exists (race), that's fine
            if e.raw_os_error() != Some(17) {
                return Err(e).context("Failed to symlink /mnt/sdcard")?;
            }
        }
    }

    debug!("Created /storage and /sdcard symlinks");
    Ok(())
}

/// Create /mnt/ directory structure.
fn setup_mnt_directory(root_path: &Path) -> Result<()> {
    let mnt = root_path.join("mnt");
    ensure_dir_android(&mnt, 0o755)?;
    ensure_dir_android(&mnt.join("media_rw"), 0o755)?;
    ensure_dir_android(&mnt.join("expand"), 0o755)?;
    ensure_dir_android(&mnt.join("obb"), 0o755)?;
    ensure_dir_android(&mnt.join("user"), 0o755)?;

    debug!("Created /mnt directory structure");
    Ok(())
}

/// Create /acct/ directory structure.
///
/// Android expects /acct to be a mount point for cgroup accounting.
/// We create a minimal directory.
fn setup_acct_directory(root_path: &Path) -> Result<()> {
    let acct = root_path.join("acct");
    ensure_dir_android(&acct, 0o755)?;

    debug!("Created /acct directory");
    Ok(())
}

/// Create /config/ directory.
fn setup_config_directory(root_path: &Path) -> Result<()> {
    let config = root_path.join("config");
    ensure_dir_android(&config, 0o755)?;

    debug!("Created /config directory");
    Ok(())
}

/// Create /d/ directory.
///
/// Some Android versions create /d/ as a symlink to /proc/self/fd.
fn setup_d_directory(root_path: &Path) -> Result<()> {
    let d = root_path.join("d");
    ensure_dir_android(&d, 0o755)?;

    debug!("Created /d directory");
    Ok(())
}

/// Create /cache/ directory.
///
/// Android system cache.
fn setup_cache_directory(root_path: &Path) -> Result<()> {
    let cache = root_path.join("cache");
    ensure_dir_android(&cache, 0o755)?;
    ensure_dir_android(&cache.join("recovery"), 0o755)?;

    debug!("Created /cache directory");
    Ok(())
}

/// Create /metadata/ directory.
fn setup_metadata_directory(root_path: &Path) -> Result<()> {
    let metadata = root_path.join("metadata");
    ensure_dir_android(&metadata, 0o755)?;

    debug!("Created /metadata directory");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Ensure a directory exists with Android-appropriate permissions.
fn ensure_dir_android(path: &Path, mode: u32) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            // Update permissions
            fs::set_permissions(path, fs::Permissions::from_mode(mode))
                .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
            return Ok(());
        }
        // Path exists but is not a directory — that's an error
        anyhow::bail!("{} exists but is not a directory", path.display());
    }

    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory {}", path.display()))?;

    // Set ownership (if running as root, set to configured uid/gid)
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup_test_root() -> PathBuf {
        let dir = std::env::temp_dir()
            .join(&format!("sober_android_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_setup_android_layout_creates_dirs() {
        let root = setup_test_root();
        setup_android_layout(&root).unwrap();

        // Check essential directories exist
        assert!(root.join("data/data/com.roblox.client").is_dir());
        assert!(root.join("data/data/com.roblox.client/files").is_dir());
        assert!(root.join("data/local/tmp").is_dir());
        assert!(root.join("system/lib").is_dir());
        assert!(root.join("system/lib64").is_dir());
        assert!(root.join("system/bin").is_dir());
        assert!(root.join("system/etc").is_dir());
        assert!(root.join("vendor/lib").is_dir());
        assert!(root.join("vendor/lib64").is_dir());
        assert!(root.join("storage/emulated/0").is_dir());
        assert!(root.join("cache").is_dir());

        // Check build.prop
        assert!(root.join("system/etc/build.prop").is_file());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_setup_android_layout_idempotent() {
        let root = setup_test_root();
        setup_android_layout(&root).unwrap();
        // Second call should succeed without error
        setup_android_layout(&root).unwrap();
        let _ = fs::remove_dir_all(&root);
    }
}