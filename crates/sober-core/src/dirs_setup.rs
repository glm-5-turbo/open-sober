// SPDX-License-Identifier: MIT
//
// Directory and path setup helpers.

use std::path::PathBuf;

/// Get the cache directory for Open Sober.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("open-sober")
}

/// Get the directory for extracted native libraries.
pub fn libs_dir() -> PathBuf {
    cache_dir().join("libs")
}

/// Get the Android environment root directory.
pub fn android_env_dir() -> PathBuf {
    cache_dir().join("android-env")
}

/// Get the APK cache directory.
pub fn apk_cache_dir() -> PathBuf {
    cache_dir().join("apks")
}

/// Ensure all directories exist.
pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(libs_dir())?;
    std::fs::create_dir_all(android_env_dir())?;
    std::fs::create_dir_all(apk_cache_dir())?;
    Ok(())
}