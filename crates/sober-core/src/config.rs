// SPDX-License-Identifier: MIT
//
// Configuration management for Open Sober.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Open Sober configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SoConfig {
    /// Path to the Roblox APK
    pub apk_path: Option<PathBuf>,

    /// Path to QEMU user-mode binary (qemu-aarch64)
    pub qemu_path: PathBuf,

    /// Path to android2gnulinux runtime directory
    pub android_runtime_path: Option<PathBuf>,

    /// Path to store downloaded APKs
    pub cache_dir: PathBuf,

    /// Graphics settings
    pub graphics: GraphicsConfig,

    /// Quality level (1-10, Roblox Android quality)
    pub quality: u32,

    /// FPS cap (None = uncapped)
    pub fps_cap: Option<u32>,

    /// Use OpenGL instead of Vulkan
    pub use_opengl: bool,

    /// Discord RPC integration
    pub discord_rpc: bool,

    /// Extra environment variables for the QEMU process
    pub extra_env: Vec<String>,
}

/// Graphics settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsConfig {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Fullscreen mode
    pub fullscreen: bool,
    /// Mesa driver override (e.g., "zink" for GLES→Vulkan)
    pub mesa_driver: Option<String>,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fullscreen: false,
            mesa_driver: Some("zink".into()),
        }
    }
}

impl Default for SoConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("open-sober");

        Self {
            apk_path: None,
            qemu_path: which_qemu(),
            android_runtime_path: None,
            cache_dir,
            graphics: GraphicsConfig::default(),
            quality: 5,
            fps_cap: Some(60),
            use_opengl: false,
            discord_rpc: false,
            extra_env: vec![],
        }
    }
}

fn which_qemu() -> PathBuf {
    // Check common locations
    for path in &[
        "/usr/bin/qemu-aarch64",
        "/usr/local/bin/qemu-aarch64",
        "/run/current-system/sw/bin/qemu-aarch64",
    ] {
        if std::path::Path::new(path).exists() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from("qemu-aarch64") // fall back to PATH lookup
}

impl SoConfig {
    /// Load config from a JSON file path, or use defaults.
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        if let Some(path) = path {
            let data = std::fs::read_to_string(path)?;
            let cfg: SoConfig = serde_json::from_str(&data)?;
            Ok(cfg)
        } else {
            // Try default config path
            let default_path = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("open-sober")
                .join("config.json");

            if default_path.exists() {
                let data = std::fs::read_to_string(&default_path)?;
                let cfg: SoConfig = serde_json::from_str(&data)?;
                Ok(cfg)
            } else {
                Ok(SoConfig::default())
            }
        }
    }

    /// Save config to the default path.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("open-sober")
            .join("config.json");

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Get the mesa driver override string.
    pub fn mesa_loader_override(&self) -> Option<&str> {
        self.graphics.mesa_driver.as_deref()
    }
}