use anyhow::{anyhow, Result};
use colored::*;
use std::path::PathBuf;

const BRIDGE_BASE: &str = "http://localhost:3344";

/// Locates the whatsapp-bridge/ directory relative to the running binary,
/// falling back to the current working directory for `cargo run` during
/// development. Mirrors the same lookup pattern used for frontend/ in
/// server/api.rs.
fn bridge_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let candidate = exe
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("whatsapp-bridge");
    if candidate.exists() {
        candidate
    } else {
        PathBuf::from("whatsapp-bridge")
    }
}

async fn ping_bridge() -> bool {
    reqwest::Client::new()
        .get(format!("{}/health", BRIDGE_BASE))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Starts the WhatsApp bridge as a detached background process if it
/// isn't already running, and waits briefly for it to come up. Unlike
/// `ensure_ollama_running`, a missing bridge isn't fatal — WhatsApp
/// support is optional, so failures here are reported but don't stop
/// `mimona serve` from continuing to run everything else.
pub async fn ensure_bridge_running() {
    if ping_bridge().await {
        println!("  {} WhatsApp bridge already running", "✓".green());
        return;
    }

    let dir = bridge_dir();
    if !dir.exists() {
        println!(
            "  {} WhatsApp bridge not found at {} — skipping (WhatsApp support disabled)",
            "!".yellow(),
            dir.display()
        );
        return;
    }

    if let Err(e) = ensure_dependencies_installed(&dir).await {
        println!(
            "  {} Could not prepare WhatsApp bridge: {}",
            "!".yellow(),
            e
        );
        return;
    }

    if let Err(e) = spawn_bridge(&dir).await {
        println!("  {} Could not start WhatsApp bridge: {}", "!".yellow(), e);
        return;
    }

    for i in 0..20u32 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if ping_bridge().await {
            println!("  {} WhatsApp bridge ready", "✓".green());
            return;
        }
        if i == 6 {
            println!("  {} Starting WhatsApp bridge (first run installs dependencies)...", "→".cyan());
        }
    }

    println!(
        "  {} WhatsApp bridge is taking a while to start — check {} for details",
        "!".yellow(),
        crate::config::whatsapp_bridge_log_path().display()
    );
}

/// Runs `npm install` the first time (no node_modules yet). Subsequent
/// boots skip this — it's the slow part and only needs to happen once
/// per dependency change.
async fn ensure_dependencies_installed(dir: &PathBuf) -> Result<()> {
    if dir.join("node_modules").exists() {
        return Ok(());
    }

    println!("  {} Installing WhatsApp bridge dependencies (first run only)...", "→".cyan());

    let status = tokio::process::Command::new("npm")
        .arg("install")
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| anyhow!("Failed to run npm install: {}", e))?;

    if !status.success() {
        return Err(anyhow!(
            "npm install failed in {} — run it manually to see the error",
            dir.display()
        ));
    }

    Ok(())
}

/// Spawns `npm start` detached, with output redirected to a log file
/// (rather than swallowed entirely) so a misbehaving bridge can still be
/// diagnosed without cluttering Mimona's own terminal output.
async fn spawn_bridge(dir: &PathBuf) -> Result<()> {
    crate::config::ensure_dirs().await.ok();
    let log_path = crate::config::whatsapp_bridge_log_path();

    let log_file_out = std::fs::File::create(&log_path)
        .map_err(|e| anyhow!("Could not create bridge log file: {}", e))?;
    let log_file_err = log_file_out
        .try_clone()
        .map_err(|e| anyhow!("Could not duplicate log file handle: {}", e))?;

    tokio::process::Command::new("npm")
        .arg("start")
        .current_dir(dir)
        .stdout(std::process::Stdio::from(log_file_out))
        .stderr(std::process::Stdio::from(log_file_err))
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn npm start: {}", e))?;

    Ok(())
}