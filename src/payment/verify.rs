use crate::payment::wallet::{get_balance, load_wallet};
use anyhow::{anyhow, Result};
use colored::*;
use dialoguer::Confirm;

/// Check the user has sufficient balance and confirm payment.
/// In a full implementation this sends a real Solana transaction.
pub async fn check_and_charge(price_sol: f64) -> Result<String> {
    if price_sol == 0.0 {
        return Ok("free".to_string());
    }

    let wallet = load_wallet().await.map_err(|_| {
        anyhow!(
            "No wallet found. Create one first:\n    {} wallet create",
            "mimona".cyan()
        )
    })?;

    let balance = get_balance(&wallet.address).await?;

    println!(
        "\n  {} Model tier: {} ({} SOL/query)",
        "ℹ".cyan(),
        "PAID".yellow().bold(),
        price_sol
    );
    println!(
        "  {} Wallet: {}",
        "ℹ".cyan(),
        wallet.address.dimmed()
    );
    println!(
        "  {} Balance: {} SOL",
        "ℹ".cyan(),
        format!("{:.4}", balance).cyan()
    );

    if balance < price_sol {
        return Err(anyhow!(
            "Insufficient balance. You have {:.4} SOL, need {:.4} SOL.\n    Deposit to: {}",
            balance,
            price_sol,
            wallet.address
        ));
    }

    let confirm = Confirm::new()
        .with_prompt(format!(
            "  Approve payment of {} SOL?",
            price_sol
        ))
        .default(true)
        .interact()?;

    if !confirm {
        return Err(anyhow!("Payment declined."));
    }

    // TODO: send_transaction(&wallet, MIMONA_TREASURY, price_sol).await?
    // For MVP, log the intent and proceed
    let tx_id = format!("mock_tx_{}", uuid::Uuid::new_v4());
    println!("  {} Payment approved (tx: {})", "✓".green(), tx_id.dimmed());

    Ok(tx_id)
}
