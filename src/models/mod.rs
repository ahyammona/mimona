pub mod downloader;
pub mod registry;
pub mod storage;

use serde::{Deserialize, Serialize};

/// Pricing tier for a model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Free,
    Paid,
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelTier::Free => write!(f, "FREE"),
            ModelTier::Paid => write!(f, "PAID"),
        }
    }
}

/// A single model variant (e.g. qwen2.5-coder:7b)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    /// HuggingFace repo (e.g. "bartowski/Qwen2.5-Coder-7B-Instruct-GGUF")
    pub hf_repo: String,
    /// Filename within the repo
    pub filename: String,
    /// Size in GB
    pub size_gb: f64,
    /// Minimum RAM required in GB
    pub ram_required_gb: u32,
    /// SHA-256 checksum
    pub checksum: Option<String>,
    /// Price per query in SOL (0.0 = free)
    pub price_sol: f64,
}

/// A model entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Short name (e.g. "qwen2.5-coder")
    pub name: String,
    /// Available tags (e.g. ["3b", "7b", "14b"])
    pub tags: Vec<String>,
    /// Default tag if none specified
    pub default_tag: String,
    /// Tier
    pub tier: ModelTier,
    /// Description
    pub description: String,
    /// Variants keyed by tag
    pub variants: std::collections::HashMap<String, ModelVariant>,
}

impl ModelEntry {
    /// Parse "name:tag" or just "name" → (name, tag)
    pub fn parse_name_tag(input: &str) -> (String, Option<String>) {
        if let Some((name, tag)) = input.split_once(':') {
            (name.to_string(), Some(tag.to_string()))
        } else {
            (input.to_string(), None)
        }
    }

    /// Get the variant for a given tag (or default)
    pub fn get_variant(&self, tag: Option<&str>) -> Option<&ModelVariant> {
        let t = tag.unwrap_or(&self.default_tag);
        self.variants.get(t)
    }
}

/// Full registry
#[derive(Debug, Serialize, Deserialize)]
pub struct Registry {
    pub version: String,
    pub models: Vec<ModelEntry>,
}
