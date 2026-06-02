use crate::models::{storage::remove_model, ModelEntry};
use anyhow::Result;
use colored::*;
use dialoguer::Confirm;

pub async fn handle(model_name: String) -> Result<()> {
    let (name, tag_opt) = ModelEntry::parse_name_tag(&model_name);
    let tag = tag_opt.unwrap_or_else(|| "latest".to_string());

    let confirm = Confirm::new()
        .with_prompt(format!("  Remove {}:{}?", name, tag))
        .default(false)
        .interact()?;

    if !confirm {
        println!("  Cancelled.");
        return Ok(());
    }

    let removed = remove_model(&name, &tag).await?;

    if removed {
        println!("  {} Removed {}:{}", "✓".green(), name, tag);
    } else {
        println!("  {} Model {}:{} not found locally.", "!".yellow(), name, tag);
    }

    Ok(())
}
