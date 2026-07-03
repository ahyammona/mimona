use crate::config::widget_settings_path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

/// Settings for the embeddable website chat widget. Persisted to
/// ~/.mimona/widget_settings.json so both the desktop UI (which edits them)
/// and the HTTP server (which serves /widget.js and /widget, and answers
/// /api/widget/chat) can read the same source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSettings {
    #[serde(default = "default_bot_name")]
    pub bot_name: String,
    #[serde(default = "default_welcome")]
    pub welcome: String,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_bot_name() -> String { "AI Assistant".to_string() }
fn default_welcome() -> String { "Hi! How can I help you today?".to_string() }
fn default_system_prompt() -> String {
    "You are a helpful, friendly assistant embedded on a website. Be concise and helpful.".to_string()
}
fn default_color() -> String { "#000000".to_string() }
fn default_model() -> String { "tinyllama:1b".to_string() }

impl Default for WidgetSettings {
    fn default() -> Self {
        Self {
            bot_name: default_bot_name(),
            welcome: default_welcome(),
            system_prompt: default_system_prompt(),
            color: default_color(),
            model: default_model(),
        }
    }
}

impl WidgetSettings {
    pub async fn load() -> Self {
        let path = widget_settings_path();
        if let Ok(raw) = fs::read_to_string(&path).await {
            if let Ok(parsed) = serde_json::from_str::<WidgetSettings>(&raw) {
                return parsed;
            }
        }
        Self::default()
    }

    pub async fn save(&self) -> Result<()> {
        crate::config::ensure_dirs().await?;
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(widget_settings_path(), raw).await?;
        Ok(())
    }
}