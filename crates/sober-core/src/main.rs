// SPDX-License-Identifier: MIT
//
// open-sober — Main entry point for the Open Sober Roblox Linux runtime.
//
// Flow:
// 1. Parse CLI args
// 2. Initialize config
// 3. If auth needed, spawn sober-services (or open browser directly)
// 4. Initialize QEMU user-mode + android2gnulinux environment
// 5. Launch Roblox Android APK via binary translation
// 6. Handle graphics/sandbox

mod apk;
mod config;
mod qemu;
mod android_env;
mod dirs_setup;

use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "open-sober", about = "Open-source Roblox Linux runtime")]
struct Cli {
    /// Path to a Roblox APK file (downloads if not provided)
    #[arg(short, long)]
    apk: Option<String>,

    /// Roblox place ID to join directly
    #[arg(short, long)]
    place_id: Option<u64>,

    /// Skip auth and use existing .ROBLOSECURITY cookie
    #[arg(short, long)]
    token: Option<String>,

    /// Path to config JSON
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Command to run: "auth" (login only), "play" (play game), "launch" (full flow)
    #[arg(default_value = "launch")]
    command: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "open_sober=debug,sober_core=debug"
    } else {
        "open_sober=info,sober_core=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .init();

    info!("Open Sober v{} starting", env!("CARGO_PKG_VERSION"));

    // Load config
    let cfg = config::SoConfig::load(cli.config.as_deref())?;
    info!("Config loaded: {:?}", cfg);

    match cli.command.as_str() {
        "auth" => run_auth(&cli, &cfg)?,
        "play" => run_play(&cli, &cfg)?,
        "launch" => run_launch(&cli, &cfg)?,
        _ => {
            anyhow::bail!("Unknown command: {}. Use 'auth', 'play', or 'launch'.", cli.command);
        }
    }

    Ok(())
}

/// Run authentication flow — opens an embedded webview window for Roblox login.
fn run_auth(_cli: &Cli, _cfg: &config::SoConfig) -> anyhow::Result<()> {
    info!("Starting embedded webview authentication...");

    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Open Sober — Sign in to Roblox")
        .with_inner_size(tao::dpi::LogicalSize::new(800.0, 700.0))
        .build(&event_loop)?;

    let token_saved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ts = token_saved.clone();

    let _webview = WebViewBuilder::new()
        .with_url("https://www.roblox.com/login")
        .with_navigation_handler(move |url: String| -> bool {
            if url.starts_with("roblox:") || url.starts_with("roblox-player:") {
                info!("Intercepted Roblox URI: {}", url);
                false
            } else {
                true
            }
        })
        .with_ipc_handler(move |req| {
            let msg = req.body();
            if !ts.load(std::sync::atomic::Ordering::SeqCst) {
                let token = msg.trim_matches('"').to_string();
                if !token.is_empty() {
                    info!("Auth token received via IPC");
                    if let Err(e) = save_token(&token) {
                        tracing::error!("Failed to save token: {}", e);
                    } else {
                        ts.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        })
        .with_initialization_script(
            r#"
            setInterval(function() {
                try {
                    var cookies = document.cookie.split(';').map(function(c) { return c.trim(); });
                    for (var i = 0; i < cookies.length; i++) {
                        if (cookies[i].startsWith('.ROBLOSECURITY=')) {
                            var token = cookies[i].substring('.ROBLOSECURITY='.length);
                            window.ipc.postMessage(JSON.stringify(token));
                            break;
                        }
                    }
                } catch(e) {}
            }, 2000);
            "#,
        )
        .build(&window)
        .map_err(|e| anyhow::anyhow!("Failed to create webview: {}", e))?;

    info!("Webview displayed, waiting for login...");

    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if token_saved.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Auth complete, closing webview");
            println!("\n✅ Authentication successful!");
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Save the auth token to disk for future use.
fn save_token(token: &str) -> anyhow::Result<()> {
    let token_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("open-sober")
        .join(".ROBLOSECURITY");

    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&token_path, token)?;
    info!("Auth token saved to {:?}", token_path);
    Ok(())
}

/// URL-encode a string for use in a redirect URI.
#[allow(dead_code)]
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

/// Play a Roblox experience (assumes already authenticated).
fn run_play(cli: &Cli, cfg: &config::SoConfig) -> anyhow::Result<()> {
    let token_str: Option<String> = cli.token.clone()
        .or_else(|| std::env::var("ROBLOSECURITY").ok())
        .or_else(|| {
            let path = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("open-sober")
                .join(".ROBLOSECURITY");
            std::fs::read_to_string(&path).ok()
        });
    let token = token_str.as_deref()
        .ok_or_else(|| anyhow::anyhow!(
            "No auth token found. Run 'open-sober auth' or set ROBLOSECURITY env var."
        ))?;

    let token = token.trim().to_string();
    info!("Auth token loaded ({} chars)", token.len());

    // Download APK if needed
    let apk_path = apk::ensure_apk(cli.apk.as_deref())?;
    info!("APK ready: {}", apk_path.display());

    // Set up Android environment
    let env = android_env::AndroidEnv::setup()?;
    info!("Android environment ready at: {}", env.root.display());

    // Launch via QEMU user-mode
    info!("Launching Roblox via QEMU user-mode...");
    qemu::launch_roblox(&apk_path, &env, &token, cfg, cli.place_id)?;

    Ok(())
}

/// Full launch: authenticate if needed, then play.
fn run_launch(cli: &Cli, cfg: &config::SoConfig) -> anyhow::Result<()> {
    // Check if we have a token
    let has_token = cli.token.is_some()
        || std::env::var("ROBLOSECURITY").is_ok()
        || dirs::data_dir()
            .map(|d| d.join("open-sober").join(".ROBLOSECURITY").exists())
            .unwrap_or(false);

    if !has_token {
        info!("No auth token found — launching auth flow");
        run_auth(cli, cfg)?;
        // After auth, the token should be saved. Continue to play.
    }

    run_play(cli, cfg)
}