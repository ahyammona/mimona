use crate::config::models_dir;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub name: String,
    pub tag: String,
    pub filename: String,
    pub size_bytes: u64,
    pub downloaded_at: String,
    pub checksum: Option<String>,
}

impl LocalModel {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    pub fn path(&self) -> PathBuf {
        models_dir().join(&self.filename)
    }
}

/// Path to the local model manifest
fn manifest_path() -> PathBuf {
    crate::config::mimona_dir().join("models.json")
}

/// Load all locally downloaded models
pub async fn load_local_models() -> Result<Vec<LocalModel>> {
    let path = manifest_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path).await?;
    let models: Vec<LocalModel> = serde_json::from_str(&raw)?;
    Ok(models)
}

/// Save updated model list
pub async fn save_local_models(models: &[LocalModel]) -> Result<()> {
    let raw = serde_json::to_string_pretty(models)?;
    fs::write(manifest_path(), raw).await?;
    Ok(())
}

/// Register a newly downloaded model
pub async fn register_model(model: LocalModel) -> Result<()> {
    let mut models = load_local_models().await?;
    // Remove existing entry if any
    models.retain(|m| m.full_name() != model.full_name());
    models.push(model);
    save_local_models(&models).await
}

/// Check if a model is already downloaded
pub async fn is_downloaded(name: &str, tag: &str) -> bool {
    let models = load_local_models().await.unwrap_or_default();
    models.iter().any(|m| m.name == name && m.tag == tag && m.path().exists())
}

/// Remove a model record and delete its file
pub async fn remove_model(name: &str, tag: &str) -> Result<bool> {
    let mut models = load_local_models().await?;
    if let Some(pos) = models.iter().position(|m| m.name == name && m.tag == tag) {
        let model = models.remove(pos);
        if model.path().exists() {
            fs::remove_file(model.path()).await?;
        }
        save_local_models(&models).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
