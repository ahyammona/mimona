use crate::config::{mimona_dir, Config};
use crate::models::{ModelEntry, Registry};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::fs;

const BUNDLED_REGISTRY: &str = include_str!("../../registry.json");

/// Fetch registry from network or fall back to bundled
pub async fn fetch_registry() -> Result<Registry> {
    let cfg = Config::load().await?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    match client
        .get(format!("{}/registry.json", cfg.registry_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let registry: Registry = resp.json().await?;
            // Cache locally
            let cache_path = mimona_dir().join("registry.json");
            let raw = serde_json::to_string_pretty(&registry)?;
            let _ = fs::write(cache_path, raw).await;
            Ok(registry)
        }
        _ => {
            // Try local cache
            let cache_path = mimona_dir().join("registry.json");
            if cache_path.exists() {
                let raw = fs::read_to_string(&cache_path).await?;
                let registry: Registry = serde_json::from_str(&raw)?;
                return Ok(registry);
            }
            // Fall back to bundled
            let registry: Registry = serde_json::from_str(BUNDLED_REGISTRY)?;
            Ok(registry)
        }
    }
}

/// Find a specific model entry by name (and optional tag)
pub async fn find_model(name: &str) -> Result<ModelEntry> {
    let (model_name, _tag) = ModelEntry::parse_name_tag(name);
    let registry = fetch_registry().await?;

    registry
        .models
        .into_iter()
        .find(|m| m.name == model_name || format!("{}:{}", m.name, m.default_tag) == name)
        .ok_or_else(|| anyhow!("Model '{}' not found in registry. Run `mimona list` to see available models.", name))
}
