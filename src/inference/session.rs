/// Session manager — keeps a loaded model in memory for fast repeated inference.
/// In a real deployment this would be a global Arc<Mutex<...>> or an actor.

use crate::inference::{ChatMessage, InferenceRequest};
use std::path::PathBuf;



#[derive(Default)]
pub struct Session {
    pub model_path: Option<PathBuf>,
    pub history: Vec<ChatMessage>,
}

impl Session {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path: Some(model_path),
            history: Vec::new(),
        }
    }

    pub fn push_user(&mut self, content: String) {
        self.history.push(ChatMessage {
            role: "user".into(),
            content,
        });
    }

    pub fn push_assistant(&mut self, content: String) {
        self.history.push(ChatMessage {
            role: "assistant".into(),
            content,
        });
    }

    pub fn to_request(&self, temperature: f32, max_tokens: u32, system: Option<String>) -> InferenceRequest {
        InferenceRequest {
            model_path: self.model_path.clone().unwrap_or_default(),
            messages: self.history.clone(),
            temperature,
            max_tokens,
            system_prompt: system,
        }
    }
}
