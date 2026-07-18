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
use tracing::{info, warn, error};

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

/// Run authentication flow only.
fn run_auth(_cli: &Cli, _cfg: &config::SoConfig) -> anyhow::Result<()> {
    info!("Starting authentication flow...");
    // TODO: Launch sober-services binary or open browser with OAuth
    // For now, print instructions
    println!("Open Sober Authentication");
    println!("========================");
    println!("1. Open https://www.roblox.com/login in your browser");
    println!("2. After logging in, copy the .ROBLOSECURITY cookie value");
    println!("3. Run: open-sober --token \"YOUR_COOKIE\" play");
    Ok(())
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