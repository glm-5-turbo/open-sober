// SPDX-License-Identifier: MIT
//
// sober-services — Authentication and launcher for Open Sober
//
// This crate provides the services layer for the Open Sober Roblox runtime.
// It handles:
// 1. Browser-based OAuth login (opens system browser, runs local HTTP server)
// 2. IPC for auth token exchange with the main sober process
// 3. Persistent user configuration
// 4. URI scheme handling for roblox: links

pub mod config;
pub mod ipc;
pub mod webview;

use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::info;

use config::AppConfig;

/// Configuration for the services process.
#[derive(Clone, Debug)]
pub struct ServiceConfig {
    /// Base URL for the Roblox authentication page.
    pub auth_url: String,
    /// Path to the cookie jar file.
    pub cookie_path: PathBuf,
    /// Path to the Unix domain socket for IPC.
    pub ipc_socket_path: PathBuf,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://www.roblox.com/login".to_string(),
            cookie_path: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("open-sober")
                .join("cookies.db"),
            ipc_socket_path: dirs::runtime_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("open-sober")
                .join("services.sock"),
        }
    }
}

/// The services application.
pub struct Service {
    config: ServiceConfig,
    app_config: AppConfig,
}

impl Service {
    /// Create a new `Service` instance.
    pub fn new(config: ServiceConfig) -> Result<Self> {
        let app_config = AppConfig::load().context("Failed to load application config")?;
        Ok(Self { config, app_config })
    }

    /// Run the login flow:
    /// 1. Start local HTTP server for OAuth callback
    /// 2. Open system browser for Roblox login
    /// 3. Wait for the auth token
    /// 4. Send it via IPC to the parent process
    pub fn run_login_flow(&self) -> Result<()> {
        info!("Starting OAuth login flow");

        let mut webview = webview::LoginWebview::new(&self.config)?;
        let port = webview.start_server()?;

        // Build the redirect URI
        let redirect_uri = format!("http://127.0.0.1:{}/callback", port);
        let auth_url = if self.config.auth_url.contains('?') {
            format!("{}&redirect_uri={}", self.config.auth_url, urlencoding(&redirect_uri))
        } else {
            format!("{}?redirect_uri={}", self.config.auth_url, urlencoding(&redirect_uri))
        };

        info!("Opening browser for authentication...");
        webview.open_browser(&auth_url)?;

        info!("Waiting for authentication...");
        let timeout_secs = 300; // 5 minutes
        match webview.wait_for_token(timeout_secs) {
            Some(token) => {
                info!("Authentication successful, sending token via IPC");
                crate::ipc::send_auth_token(
                    self.config.ipc_socket_path.to_str().unwrap_or("/tmp/open-sober-auth.sock"),
                    &token,
                )?;
                info!("Auth token sent to parent process");
                Ok(())
            }
            None => {
                anyhow::bail!("Authentication timed out after {} seconds", timeout_secs);
            }
        }
    }

    /// Return the current user configuration.
    pub fn app_config(&self) -> &AppConfig {
        &self.app_config
    }
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}