use anyhow::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Root dir: ~/.mimona/
pub fn mimona_dir() -> PathBuf {
    home_dir().expect("Could not find home directory").join(".mimona")
}

pub fn models_dir() -> PathBuf {
    mimona_dir().join("models")
}

pub fn config_path() -> PathBuf {
    mimona_dir().join("config.toml")
}

pub fn wallet_path() -> PathBuf {
    mimona_dir().join("wallet.json")
}

pub fn whatsapp_users_path() -> PathBuf {
    mimona_dir().join("whatsapp_users.json")
}

pub fn whatsapp_bridge_log_path() -> PathBuf {
    mimona_dir().join("whatsapp-bridge.log")
}

pub async fn ensure_dirs() -> Result<()> {
    fs::create_dir_all(models_dir()).await?;
    Ok(())
}

// ─── Main Config ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub registry_url: String,
    pub serve_host: String,
    pub serve_port: u16,
    pub solana_rpc: String,
    pub default_model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            registry_url: "https://registry.mimona.io".to_string(),
            serve_host: "127.0.0.1".to_string(),
            serve_port: 11435,
            solana_rpc: "https://api.mainnet-beta.solana.com".to_string(),
            default_model: None,
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            let cfg = Config::default();
            cfg.save().await?;
            return Ok(cfg);
        }
        let raw = fs::read_to_string(&path).await?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }

    pub async fn save(&self) -> Result<()> {
        ensure_dirs().await?;
        let raw = toml::to_string_pretty(self)?;
        fs::write(config_path(), raw).await?;
        Ok(())
    }
}