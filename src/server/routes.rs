use crate::models::storage::load_local_models;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;

const OLLAMA_BASE: &str = "http://localhost:11434";

#[derive(Clone)]
pub struct AppState {
    pub host: String,
    pub port: u16,
}

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "name": "mimona",
    }))
}

pub async fn list_tags() -> impl IntoResponse {
    let models = load_local_models().await.unwrap_or_default();
    let items: Vec<Value> = models.iter().map(|m| json!({
        "name": m.full_name(),
        "size": m.size_bytes,
        "modified_at": m.downloaded_at,
    })).collect();
    Json(json!({ "models": items }))
}

pub async fn list_models_openai() -> impl IntoResponse {
    let models = load_local_models().await.unwrap_or_default();
    let items: Vec<Value> = models.iter().map(|m| json!({
        "id": m.full_name(),
        "object": "model",
        "created": 0,
        "owned_by": "mimona",
    })).collect();
    Json(json!({ "object": "list", "data": items }))
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OaiMessage>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OaiMessage {
    pub role: String,
    pub content: String,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32  { 2048 }

pub async fn chat_completions(
    Json(body): Json<ChatCompletionRequest>,
) -> Response {
    // Strip tag suffix for Ollama (tinyllama:1b → tinyllama:latest or just tinyllama)
    let ollama_model = resolve_ollama_model(&body.model).await;

    match ollama_model {
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!(
                        "Model '{}' not available. Make sure Ollama is running with this model.",
                        body.model
                    )
                })),
            ).into_response();
        }
        Some(model) => {
            proxy_to_ollama(&model, &body).await
        }
    }
}

async fn proxy_to_ollama(ollama_model: &str, body: &ChatCompletionRequest) -> Response {
    let messages: Vec<Value> = body.messages.iter().map(|m| {
        json!({"role": m.role, "content": m.content})
    }).collect();

    let ollama_body = json!({
        "model": ollama_model,
        "messages": messages,
        "stream": false,
        "options": {
            "temperature": body.temperature,
            "num_predict": body.max_tokens,
        }
    });

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/chat", OLLAMA_BASE))
        .json(&ollama_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("Ollama unreachable: {}", e) })),
            ).into_response();
        }
    };

    let ollama_resp: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Bad response from Ollama: {}", e) })),
            ).into_response();
        }
    };

    let content = ollama_resp["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Json(json!({
        "id": "chatcmpl-mimona",
        "object": "chat.completion",
        "model": body.model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0,
        }
    })).into_response()
}

/// Try to find a matching Ollama model for a given Mimona model name
/// e.g. "tinyllama:1b" → "tinyllama:latest", "qwen2.5-coder:7b" → "qwen2.5-coder:7b"
async fn resolve_ollama_model(mimona_model: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/tags", OLLAMA_BASE))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    let data: Value = resp.json().await.ok()?;
    let ollama_models: Vec<String> = data["models"]
        .as_array()?
        .iter()
        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
        .collect();

    // Exact match first
    if ollama_models.iter().any(|m| m == mimona_model) {
        return Some(mimona_model.to_string());
    }

    // Match by base name (strip tag)
    let base = mimona_model.split(':').next().unwrap_or(mimona_model);
    if let Some(matched) = ollama_models.iter().find(|m| m.starts_with(base)) {
        return Some(matched.clone());
    }

    // Just return first available model as fallback
    ollama_models.into_iter().next()
}

#[derive(Debug, Deserialize)]
pub struct OllamaGenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub stream: bool,
}

pub async fn ollama_generate(Json(body): Json<OllamaGenerateRequest>) -> impl IntoResponse {
    let ollama_model = resolve_ollama_model(&body.model).await
        .unwrap_or_else(|| body.model.clone());

    let ollama_body = json!({
        "model": ollama_model,
        "prompt": body.prompt,
        "stream": false,
        "options": { "temperature": body.temperature }
    });

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/generate", OLLAMA_BASE))
        .json(&ollama_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(json!({ "error": format!("Ollama unreachable: {}", e) })).into_response(),
    };

    let data: Value = resp.json().await.unwrap_or(json!({}));
    let response_text = data["response"].as_str().unwrap_or("").to_string();

    Json(json!({
        "model": body.model,
        "response": response_text,
        "done": true,
    })).into_response()
}


#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn web_search(axum::extract::Query(params): axum::extract::Query<SearchQuery>) -> impl IntoResponse {
    let query = &params.q;
    
    let api_key = "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";

    let client = reqwest::Client::new();
    
    // Build the request payload expected by Tavily
    let tavily_body = json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": 5
    });

    println!("[search] Querying Tavily API for: {}", query);

    // Send the POST request to Tavily's endpoint
    match client.post("https://api.tavily.com/search")
        .json(&tavily_body)
        .send()
        .await 
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<Value>().await {
                // Extract and format the results array
                if let Some(arr) = data["results"].as_array() {
                    let results: Vec<Value> = arr.iter().map(|r| json!({
                        "title": r["title"].as_str().unwrap_or(""),
                        "snippet": r["content"].as_str().unwrap_or(""), // Tavily stores main snippet text in "content"
                        "url": r["url"].as_str().unwrap_or(""),
                    })).collect();

                    println!("[search] got {} results from Tavily", results.len());
                    for r in &results {
                        println!("  -> {}", r["title"].as_str().unwrap_or(""));
                    }
                    
                    return Json(json!({"query": query, "results": results})).into_response();
                }
            }
            println!("[search] Failed to parse Tavily JSON response framework.");
        }
        Err(e) => {
            println!("[search] Connection to Tavily failed: {}", e);
        }
    }

    // Fallback if anything goes wrong
    println!("[search] No results found or error occurred for: {}", query);
    Json(json!({"query": query, "results": []})).into_response()
}