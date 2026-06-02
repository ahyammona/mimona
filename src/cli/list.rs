use crate::models::storage::load_local_models;
use crate::models::registry::fetch_registry;
use crate::models::ModelTier;
use anyhow::Result;
use colored::*;

pub async fn handle() -> Result<()> {
    let local = load_local_models().await?;

    println!();

    if local.is_empty() {
        println!("  {} No models downloaded yet.\n", "!".yellow());
        println!("  Pull one with:  {} pull qwen2.5-coder:7b", "mimona".cyan());
        println!("  Browse all:     {} pull --list\n", "mimona".cyan());
        return Ok(());
    }

    println!("  {}", "Downloaded Models".bold());
    println!("  {}", "─".repeat(62).dimmed());
    println!(
        "  {:<30} {:<10} {:<10} {}",
        "NAME".dimmed(),
        "SIZE".dimmed(),
        "TIER".dimmed(),
        "DOWNLOADED".dimmed()
    );
    println!("  {}", "─".repeat(62).dimmed());

    for m in &local {
        let size_gb = m.size_bytes as f64 / 1_073_741_824.0;
        println!(
            "  {:<30} {:<10} {:<10} {}",
            format!("{}:{}", m.name, m.tag).cyan(),
            format!("{:.1} GB", size_gb),
            "FREE".green(),   // TODO: look up tier from registry
            m.downloaded_at.split('T').next().unwrap_or(""),
        );
    }

    println!("  {}", "─".repeat(62).dimmed());
    println!("  {} model(s)\n", local.len());

    Ok(())
}

pub async fn list_registry() -> Result<()> {
    println!("\n  {} Fetching registry...", "→".cyan());
    let registry = fetch_registry().await?;

    println!("\n  {}", "Available Models".bold());
    println!("  {}", "─".repeat(70).dimmed());
    println!(
        "  {:<28} {:<14} {:<8} {:<10} {}",
        "NAME".dimmed(),
        "TAGS".dimmed(),
        "RAM".dimmed(),
        "TIER".dimmed(),
        "DESCRIPTION".dimmed()
    );
    println!("  {}", "─".repeat(70).dimmed());

    for entry in &registry.models {
        let default_variant = entry.variants.get(&entry.default_tag);
        let ram = default_variant.map(|v| format!("{}GB", v.ram_required_gb)).unwrap_or_default();
        let tier_str = match entry.tier {
            ModelTier::Free => "FREE".green().bold(),
            ModelTier::Paid => {
                let price = default_variant.map(|v| v.price_sol).unwrap_or(0.0);
                format!("{:.3} SOL", price).yellow().bold()
            }
        };

        println!(
            "  {:<28} {:<14} {:<8} {:<18} {}",
            entry.name.cyan(),
            entry.tags.join(", "),
            ram,
            tier_str,
            entry.description.chars().take(28).collect::<String>(),
        );
    }

    println!("  {}", "─".repeat(70).dimmed());
    println!("  {} models available\n", registry.models.len());
    println!("  Pull a model:  {} pull <name>:<tag>", "mimona".cyan());
    println!("  Example:       {} pull qwen2.5-coder:7b\n", "mimona".cyan());

    Ok(())
}
