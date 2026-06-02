pub mod engine;
pub mod session;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub model_path: std::path::PathBuf,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
}

#[derive(Debug)]
pub struct InferenceResponse {
    pub text: String,
    pub tokens_generated: u32,
    pub duration_ms: u64,
}
