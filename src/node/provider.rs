use crate::models::storage::load_local_models;
use crate::payment::wallet::load_wallet;
use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub wallet: String,
    pub models: Vec<String>,
    pub endpoint: String,
    pub specs: NodeSpecs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeSpecs {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub has_gpu: bool,
}

pub async fn start_node(port: u16) -> Result<()> {
    let wallet = load_wallet().await?;
    let local_models = load_local_models().await?;

    if local_models.is_empty() {
        println!("\n  {} No models downloaded. Pull models first:\n", "!".yellow());
        println!("    mimona pull qwen2.5-coder:7b\n");
        return Ok(());
    }

    println!();
    println!("  {} Starting Mimona Node Provider", "✓".green().bold());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {} {}", "Wallet:".dimmed(), wallet.address.cyan());
    println!("  {} {}", "Port:".dimmed(), port);
    println!(
        "  {} {}",
        "Models:".dimmed(),
        local_models
            .iter()
            .map(|m| m.full_name())
            .collect::<Vec<_>>()
            .join(", ")
            .cyan()
    );

    // Detect system specs
    let specs = detect_specs().await;
    println!("  {} {} cores, {} GB RAM{}", "Hardware:".dimmed(), specs.cpu_cores, specs.ram_gb,
        if specs.has_gpu { ", GPU detected" } else { "" }
    );
    println!();

    // Register with Mimona network
    println!("  {} Registering with Mimona network...", "→".cyan());
    match register_node(&wallet.address, &local_models.iter().map(|m| m.full_name()).collect::<Vec<_>>(), port, &specs).await {
        Ok(_)  => println!("  {} Registered! Listening for requests...\n", "✓".green()),
        Err(e) => println!("  {} Could not reach registry ({}). Running in local-only mode.\n", "!".yellow(), e),
    }

    println!("  {}", "─".repeat(50).dimmed());
    println!("  Press {} to stop\n", "Ctrl+C".dimmed());

    // Start API server as node
    crate::server::api::start("0.0.0.0".to_string(), port).await?;
    Ok(())
}

async fn register_node(
    wallet: &str,
    models: &[String],
    port: u16,
    specs: &NodeSpecs,
) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "wallet": wallet,
        "models": models,
        "port": port,
        "specs": specs,
    });

    client
        .post("https://registry.mimona.io/nodes/register")
        .json(&body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    Ok(())
}

async fn detect_specs() -> NodeSpecs {
    // TODO: use sys-info crate for real detection
    NodeSpecs {
        ram_gb: 16,
        cpu_cores: num_cpus::get() as u32,
        has_gpu: false,
    }
}

pub async fn node_status() -> Result<()> {
    let wallet = load_wallet().await?;
    let models = load_local_models().await?;

    println!();
    println!("  {}", "Node Status".bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} {}", "Wallet:".dimmed(), wallet.address.cyan());
    println!("  {} {} model(s) available", "Models:".dimmed(), models.len());
    println!("  {} 0.00 SOL", "Earned today:".dimmed());
    println!("  {} 0.00 SOL", "Earned total:".dimmed());
    println!("  {} Offline", "Status:".dimmed());
    println!();
    println!("  Start earning with:  {} node start\n", "mimona".cyan());

    Ok(())
}
