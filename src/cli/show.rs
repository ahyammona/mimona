use crate::models::{registry::find_model, storage::load_local_models, ModelTier};
use anyhow::Result;
use colored::*;

pub async fn handle(model_name: String) -> Result<()> {
    let entry = find_model(&model_name).await?;
    let local = load_local_models().await?;

    println!();
    println!("  {} {}",  "Model:".dimmed(), entry.name.cyan().bold());
    println!("  {} {}", "Description:".dimmed(), entry.description);
    println!(
        "  {} {}",
        "Tier:".dimmed(),
        match entry.tier {
            ModelTier::Free => "FREE".green().bold(),
            ModelTier::Paid => "PAID".yellow().bold(),
        }
    );
    println!("  {} {}", "Default tag:".dimmed(), entry.default_tag);
    println!();
    println!("  {}", "Variants:".bold());
    println!("  {}", "─".repeat(55).dimmed());
    println!(
        "  {:<10} {:<10} {:<10} {:<12} {}",
        "TAG".dimmed(),
        "SIZE".dimmed(),
        "RAM".dimmed(),
        "PRICE".dimmed(),
        "STATUS".dimmed()
    );
    println!("  {}", "─".repeat(55).dimmed());

    for tag in &entry.tags {
        if let Some(v) = entry.variants.get(tag) {
            let is_local = local.iter().any(|m| m.name == entry.name && m.tag == *tag);
            let status = if is_local {
                "downloaded".green()
            } else {
                "not downloaded".dimmed()
            };
            let price = if v.price_sol == 0.0 {
                "free".green()
            } else {
                format!("{:.4} SOL", v.price_sol).yellow()
            };
            println!(
                "  {:<10} {:<10} {:<10} {:<20} {}",
                tag.cyan(),
                format!("{:.1} GB", v.size_gb),
                format!("{} GB", v.ram_required_gb),
                price,
                status,
            );
        }
    }

    println!("  {}", "─".repeat(55).dimmed());
    println!();
    println!("  Pull:  {} pull {}:<tag>", "mimona".cyan(), entry.name);
    println!("  Run:   {} run {}:<tag>\n", "mimona".cyan(), entry.name);

    Ok(())
}
