// SPDX-License-Identifier: MIT
//
// sober-services — User configuration (sober config.json parser)

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Behaviour when the user closes the main launcher window.
///
/// Mirrors the `close_behaviour` field in Sober's config.json.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CloseBehaviour {
    /// Exit the application entirely.
    CloseOnLeave,
    /// Hide to the system tray (requires a tray implementation).
    MinimizeToTray,
    /// Keep the process running in the background without a tray icon.
    KeepRunning,
}

impl Default for CloseBehaviour {
    fn default() -> Self {
        Self::CloseOnLeave
    }
}

/// User-facing application configuration.
///
/// Stored at `~/.config/open-sober/config.json` and parsed with serde.
/// Fields correspond one-to-one with Sober's config.json schema.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    /// Whether to store auth tokens in libsecret (GNOME Keyring) instead of
    /// a plain-text cookie file.
    pub use_libsecret: bool,

    /// Whether to force OpenGL rendering. When false, the renderer may use
    /// Vulkan or other backends depending on availability.
    pub use_opengl: bool,

    /// Behaviour when the user closes the window.
    pub close_behaviour: CloseBehaviour,

    /// Graphics quality level (0–10, matching Roblox's internal quality slider).
    pub quality: u32,

    /// Optional frame-rate cap. None means uncapped.
    pub fps_cap: Option<u32>,

    /// Whether to enable Discord Rich Presence integration.
    pub discord_rpc: bool,

    /// Path to a custom RobloxPlayer binary. If None, the default bundled
    /// binary is used.
    pub player_path: Option<String>,

    /// Additional environment variables to pass to the RobloxPlayer process.
    pub extra_env: Vec<(String, String)>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            use_libsecret: false,
            use_opengl: false,
            close_behaviour: CloseBehaviour::default(),
            quality: 5,
            fps_cap: Some(60),
            discord_rpc: true,
            player_path: None,
            extra_env: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Path to the config file on disk.
    fn config_path() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .or_else(|| {
                dirs::home_dir().map(|h| h.join(".config"))
            })
            .context("Cannot determine config directory")?;
        Ok(base.join("open-sober").join("config.json"))
    }

    /// Load the configuration from disk.
    ///
    /// If the file does not exist or is malformed, a default configuration is
    /// returned. Errors other than "not found" are still surfaced.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("No config file found at {}, using defaults", path.display());
                return Ok(Self::default());
            }
            Err(e) => {
                anyhow::bail!("Failed to read config at {}: {}", path.display(), e);
            }
        };

        let cfg: Self =
            serde_json::from_str(&data).with_context(|| {
                format!("Failed to parse config at {}", path.display())
            })?;

        tracing::debug!("Loaded configuration from {}", path.display());
        Ok(cfg)
    }

    /// Save the configuration to disk.
    ///
    /// Creates the config directory and file if they do not exist.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {}", parent.display()))?;
        }

        let data = serde_json::to_string_pretty(self)
            .context("Failed to serialise config as JSON")?;

        fs::write(&path, &data)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        tracing::info!("Saved configuration to {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert!(!cfg.use_libsecret);
        assert!(!cfg.use_opengl);
        assert_eq!(cfg.close_behaviour, CloseBehaviour::CloseOnLeave);
        assert_eq!(cfg.quality, 5);
        assert_eq!(cfg.fps_cap, Some(60));
        assert!(cfg.discord_rpc);
    }

    #[test]
    fn test_serde_roundtrip() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.close_behaviour, parsed.close_behaviour);
        assert_eq!(cfg.fps_cap, parsed.fps_cap);
    }

    #[test]
    fn test_close_behaviour_deserialize() {
        let json = r#"{"close_behaviour": "MinimizeToTray"}"#;
        let partial: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(partial.close_behaviour, CloseBehaviour::MinimizeToTray);
    }
}