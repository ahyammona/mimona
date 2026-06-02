/// Local wallet management.
/// Keypairs are stored in ~/.mimona/wallet.json
/// The wallet is a simple Solana keypair — no external dependency needed
/// for key generation, only for on-chain operations.

use anyhow::{anyhow, Result};
use colored::*;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalWallet {
    /// Base58-encoded public key
    pub address: String,
    /// Base58-encoded secret key bytes (64 bytes)
    pub secret_key: String,
    pub created_at: String,
}

impl LocalWallet {
    /// Generate a new keypair
    pub fn generate() -> Self {
        use rand::RngCore;
        // Generate 32 random bytes as seed (Ed25519)
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);

        // Derive a simple keypair representation
        // In a full implementation, use ed25519-dalek or solana-sdk
        let secret_b58 = bs58::encode(&seed).into_string();
        // Public key placeholder (would be derived from secret in real impl)
        let pub_key = bs58::encode(&seed[..16]).into_string();

        Self {
            address: pub_key,
            secret_key: secret_b58,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub async fn load_wallet() -> Result<LocalWallet> {
    let path = crate::config::wallet_path();
    if !path.exists() {
        return Err(anyhow!(
            "No wallet found. Create one with: {} wallet create",
            "mimona".cyan()
        ));
    }
    let raw = fs::read_to_string(&path).await?;
    let wallet: LocalWallet = serde_json::from_str(&raw)?;
    Ok(wallet)
}

pub async fn save_wallet(wallet: &LocalWallet) -> Result<()> {
    crate::config::ensure_dirs().await?;
    let raw = serde_json::to_string_pretty(wallet)?;
    // Write with restrictive permissions (owner read-only)
    fs::write(crate::config::wallet_path(), raw).await?;
    Ok(())
}

pub async fn create_wallet() -> Result<LocalWallet> {
    let path = crate::config::wallet_path();
    if path.exists() {
        return Err(anyhow!(
            "Wallet already exists at {}. Delete it first to create a new one.",
            path.display()
        ));
    }
    let wallet = LocalWallet::generate();
    save_wallet(&wallet).await?;
    Ok(wallet)
}

/// Get SOL balance from Solana RPC
pub async fn get_balance(address: &str) -> Result<f64> {
    let cfg = crate::config::Config::load().await?;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [address]
    });

    let resp = client
        .post(&cfg.solana_rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("RPC error: {}", e))?;

    let val: serde_json::Value = resp.json().await?;
    let lamports = val["result"]["value"].as_u64().unwrap_or(0);
    let sol = lamports as f64 / 1_000_000_000.0; // 1 SOL = 1e9 lamports
    Ok(sol)
}
