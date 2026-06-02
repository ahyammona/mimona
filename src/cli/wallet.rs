use crate::payment::wallet::{create_wallet, get_balance, load_wallet};
use anyhow::Result;
use colored::*;

pub async fn create() -> Result<()> {
    println!();
    let wallet = create_wallet().await?;

    println!("  {} Wallet created!", "✓".green().bold());
    println!("  {}", "─".repeat(52).dimmed());
    println!("  {} {}", "Address:".dimmed(), wallet.address.cyan().bold());
    println!("  {} {}", "Stored at:".dimmed(), crate::config::wallet_path().display());
    println!();
    println!("  {} Keep your wallet file safe — it controls your funds!", "⚠".yellow());
    println!("  {} Never share your secret key.\n", "⚠".yellow());

    Ok(())
}

pub async fn balance() -> Result<()> {
    let wallet = load_wallet().await?;
    let balance = get_balance(&wallet.address).await?;

    println!();
    println!("  {}", "Wallet Balance".bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} {}", "Address:".dimmed(), wallet.address.cyan());
    println!("  {} {} SOL", "Balance:".dimmed(), format!("{:.6}", balance).cyan().bold());
    println!("  {} {}", "Created:".dimmed(), wallet.created_at.split('T').next().unwrap_or(""));
    println!();

    if balance < 0.01 {
        println!("  {} Low balance. Send SOL to your address to use paid models.\n", "⚠".yellow());
    }

    Ok(())
}

pub async fn address() -> Result<()> {
    let wallet = load_wallet().await?;
    println!("\n  {}\n", wallet.address.cyan().bold());
    Ok(())
}
