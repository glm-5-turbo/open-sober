// SPDX-License-Identifier: MIT
//
// sober-services — Standalone entry point for the services process

use std::path::PathBuf;

use anyhow::Result;
use sober_services::config::AppConfig;
use sober_services::ipc;
use sober_services::ServiceConfig;
use sober_services::Service;
use tracing::{error, info};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Open Sober services starting");

    let config = ServiceConfig {
        auth_url: std::env::var("SOBER_AUTH_URL")
            .unwrap_or_else(|_| "https://www.roblox.com/login".to_string()),
        cookie_path: PathBuf::from(
            std::env::var("SOBER_COOKIE_PATH")
                .unwrap_or_else(|_| {
                    dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                        .join("open-sober")
                        .join("cookies.db")
                        .to_string_lossy()
                        .to_string()
                }),
        ),
        ipc_socket_path: PathBuf::from(
            std::env::var("SOBER_IPC_SOCKET")
                .unwrap_or_else(|_| {
                    let base = dirs::runtime_dir()
                        .unwrap_or_else(|| PathBuf::from("/tmp"));
                    base.join("open-sober").join("services.sock")
                        .to_string_lossy()
                        .to_string()
                }),
        ),
    };

    let service = Service::new(config.clone())?;

    // Start IPC listener thread
    let ipc_path = config.ipc_socket_path.clone();
    std::thread::Builder::new()
        .name("sober-services-ipc".into())
        .spawn(move || {
            if let Err(e) = ipc::listen_for_auth(&ipc_path, |msg| {
                info!("Received IPC message: {:?}", msg);
                Ok(())
            }) {
                error!("IPC listener error: {:#}", e);
            }
        })
        .expect("Failed to spawn IPC listener thread");

    // Run the login flow (starts HTTP server, opens browser, waits for token)
    service.run_login_flow()?;

    // Print config for debugging
    let app_config = AppConfig::load()?;
    info!("Loaded configuration: quality={}, fps_cap={:?}",
        app_config.quality, app_config.fps_cap);

    info!("Open Sober services shutting down");
    Ok(())
}