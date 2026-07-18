// SPDX-License-Identifier: MIT
//
// APK downloader and extractor for Roblox Android APK.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{info, warn};

/// Ensure a Roblox APK is available at the given path, or download it.
pub fn ensure_apk(apk_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = apk_path {
        let p = PathBuf::from(path);
        if p.exists() {
            info!("Using provided APK: {}", p.display());
            return Ok(p);
        }
        anyhow::bail!("APK not found at: {}", p.display());
    }

    // Check cache
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("open-sober")
        .join("apks");

    let cached_apk = cache_dir.join("roblox-android.apk");
    if cached_apk.exists() {
        info!("Using cached APK: {}", cached_apk.display());
        return Ok(cached_apk);
    }

    // No APK provided and none cached.
    anyhow::bail!(
        "No Roblox APK found. Please download it from an APK mirror or \
         extract it from an Android device, then pass it with --apk <path>.\n\
         Note: The Roblox Android APK is available from APKMirror or Google Play."
    );
}

/// Extract native libraries from an APK.
///
/// Handles both standard APK format (lib/arm64-v8a/*.so) and the newer
/// Roblox bundle format (assets/app.zip -> config.arm64_v8a.apk).
pub fn extract_libs(apk_path: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    let file = std::fs::File::open(apk_path)
        .with_context(|| format!("Failed to open APK: {}", apk_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .context("Failed to read APK as ZIP archive")?;

    let lib_dir = output_dir.join("lib");
    std::fs::create_dir_all(&lib_dir)?;

    let mut extracted = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        // Standard APK format: lib/arm64-v8a/*.so
        if name.starts_with("lib/arm64-v8a/") && name.ends_with(".so") {
            let filename = name.strip_prefix("lib/arm64-v8a/").unwrap();
            let out_path = lib_dir.join(filename);

            if out_path.exists() {
                extracted.push(out_path);
                continue;
            }

            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out)?;
            extracted.push(out_path);
        }

        // New bundle format: assets/app.zip (contains config.arm64_v8a.apk)
        if name == "assets/app.zip" {
            let tmpdir = std::env::temp_dir().join(format!("open-sober-apk-{}", std::process::id()));
            std::fs::create_dir_all(&tmpdir)?;

            let app_zip_path = tmpdir.join("app.zip");
            let mut app_zip_file = std::fs::File::create(&app_zip_path)?;
            std::io::copy(&mut entry, &mut app_zip_file)?;

            // Open the nested app.zip
            if let Ok(mut nested) = zip::ZipArchive::new(std::fs::File::open(&app_zip_path)?) {
                for j in 0..nested.len() {
                    let mut nested_entry = nested.by_index(j)?;
                    let nested_name = nested_entry.name().to_string();

                    // Extract the ARM64 config APK
                    if nested_name == "config.arm64_v8a.apk" {
                        let config_apk_path = tmpdir.join("config.arm64_v8a.apk");
                        let mut config_file = std::fs::File::create(&config_apk_path)?;
                        std::io::copy(&mut nested_entry, &mut config_file)?;

                        // Extract .so files from the config APK
                        if let Ok(mut config) = zip::ZipArchive::new(std::fs::File::open(&config_apk_path)?) {
                            for k in 0..config.len() {
                                let mut so_entry = config.by_index(k)?;
                                let so_name = so_entry.name().to_string();
                                if so_name.starts_with("lib/arm64-v8a/") && so_name.ends_with(".so") {
                                    let filename = so_name.strip_prefix("lib/arm64-v8a/").unwrap();
                                    let out_path = lib_dir.join(filename);
                                    if !out_path.exists() {
                                        let mut out = std::fs::File::create(&out_path)?;
                                        std::io::copy(&mut so_entry, &mut out)?;
                                    }
                                    extracted.push(out_path);
                                }
                            }
                        }
                        break;
                    }
                }
            }

            // Cleanup temp dir
            let _ = std::fs::remove_dir_all(&tmpdir);
        }
    }

    // Also try extracting from just the cached libs directory
    if extracted.is_empty() {
        let cache_libs = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("open-sober")
            .join("libs");
        if cache_libs.exists() {
            for entry in std::fs::read_dir(&cache_libs)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("so") {
                    let dest = lib_dir.join(path.file_name().unwrap());
                    std::fs::copy(&path, &dest)?;
                    extracted.push(dest);
                }
            }
        }
    }

    info!("Extracted {} native libraries to {}", extracted.len(), lib_dir.display());
    Ok(extracted)
}

/// List all .so files in the APK.
pub fn list_native_libs(apk_path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(apk_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut libs = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.contains("/lib") && name.ends_with(".so") {
            libs.push(name);
        }
    }

    Ok(libs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config_defaults() {
        let cfg = crate::config::SoConfig::default();
        assert_eq!(cfg.quality, 5);
        assert!(cfg.graphics.width > 0);
    }
}