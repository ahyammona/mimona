use crate::models::{
    downloader::download_model,
    registry::find_model,
    storage::is_downloaded,
    ModelEntry, ModelTier,
};
use anyhow::Result;
use colored::*;

pub async fn handle(model_name: String, force: bool) -> Result<()> {
    let (name, tag_opt) = ModelEntry::parse_name_tag(&model_name);
    let entry = find_model(&name).await?;
    let tag = tag_opt.as_deref().unwrap_or(&entry.default_tag).to_string();

    let variant = entry
        .variants
        .get(&tag)
        .ok_or_else(|| anyhow::anyhow!("Tag '{}' not found. Available: {}", tag, entry.tags.join(", ")))?;

    println!();
    println!("  {} {}:{}", "Pulling".cyan().bold(), name, tag);
    println!("  {} {:.1} GB", "Size:".dimmed(), variant.size_gb);
    println!("  {} {} GB RAM required", "RAM:".dimmed(), variant.ram_required_gb);
    println!(
        "  {} {}",
        "Tier:".dimmed(),
        if entry.tier == ModelTier::Free {
            "FREE".green().bold()
        } else {
            format!("PAID ({} SOL/query)", variant.price_sol).yellow().bold()
        }
    );
    println!();

    if !force && is_downloaded(&name, &tag).await {
        println!("  {} Already downloaded. Use --force to re-download.", "✓".green());
        return Ok(());
    }

    download_model(&entry, &tag).await?;

    println!();
    println!(
        "  {} {}:{} is ready. Run it with:",
        "✓".green().bold(),
        name,
        tag
    );
    println!("    {} run {}:{}", "mimona".cyan(), name, tag);
    println!();

    Ok(())
}
