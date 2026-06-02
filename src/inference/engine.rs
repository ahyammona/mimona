use crate::inference::{InferenceRequest, InferenceResponse};
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::time::Instant;
use tokio::sync::mpsc;

const OLLAMA_BASE: &str = "http://localhost:11434";

pub async fn run_streaming(
    req: InferenceRequest,
    tx: mpsc::UnboundedSender<String>,
) -> Result<InferenceResponse> {
    let start = Instant::now();

    ensure_ollama_running().await?;

    let model_name = gguf_path_to_model_name(&req.model_path);
    ensure_model_loaded(&model_name).await?;

    let (text, tokens) = stream_from_ollama(&model_name, &req, tx).await?;

    Ok(InferenceResponse {
        text,
        tokens_generated: tokens,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn ensure_ollama_running() -> Result<()> {
    if ping_ollama().await {
        return Ok(());
    }

    tokio::process::Command::new("ollama")
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!(
            "Could not start inference engine.\nInstall from https://ollama.com\nError: {}", e
        ))?;

    for i in 0..20u32 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if ping_ollama().await {
            return Ok(());
        }
        if i == 10 {
            eprintln!("  Starting inference engine...");
        }
    }

    Err(anyhow!("Inference engine took too long to start."))
}

async fn ping_ollama() -> bool {
    reqwest::Client::new()
        .get(format!("{}/api/tags", OLLAMA_BASE))
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Return all model names currently loaded in Ollama
async fn ollama_loaded_models() -> Vec<String> {
    let client = reqwest::Client::new();
    if let Ok(resp) = client.get(format!("{}/api/tags", OLLAMA_BASE)).send().await {
        if let Ok(data) = resp.json::<serde_json::Value>().await {
            return data["models"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    vec![]
}

/// Find the actual name Ollama registered this model under (handles tag mismatches)
async fn resolve_actual_ollama_name(model_name: &str) -> String {
    let loaded = ollama_loaded_models().await;
    let base = model_name.split(':').next().unwrap_or(model_name);

    // Exact match
    if loaded.contains(&model_name.to_string()) {
        return model_name.to_string();
    }
    // Prefix match (e.g. "tinyllama:1b" matches "tinyllama:latest")
    if let Some(found) = loaded.iter().find(|m| m.starts_with(base)) {
        return found.clone();
    }
    // Fallback
    model_name.to_string()
}

/// Make sure model is available in Ollama.
/// Uses Mimona's already-downloaded GGUF — no second download.
async fn ensure_model_loaded(model_name: &str) -> Result<()> {
    let loaded = ollama_loaded_models().await;
    let base = model_name.split(':').next().unwrap_or(model_name);

    if loaded.iter().any(|m| m.starts_with(base) || m == model_name) {
        return Ok(()); // Already loaded
    }

    // Find Mimona's downloaded GGUF file
    let models_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mimona")
        .join("models");

    let gguf = std::fs::read_dir(&models_dir)
        .ok()
        .and_then(|rd| {
            rd.filter_map(|e| e.ok())
                .find(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    name.ends_with(".gguf")
                        && name.contains(base.split('-').next().unwrap_or(base))
                })
                .map(|e| e.path())
        });

    let gguf_path = match gguf {
        Some(p) => std::fs::canonicalize(&p).unwrap_or(p),
        None => {
            return Err(anyhow!(
                "Model '{}' not downloaded. Run: mimona pull {}",
                model_name, model_name
            ));
        }
    };

    // Write a temp Modelfile and use `ollama create`
    eprintln!("  Registering model with inference engine (one-time setup)...");

    let modelfile_content = format!("FROM {}\n", gguf_path.to_string_lossy());
    let modelfile_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mimona")
        .join("Modelfile");

    tokio::fs::write(&modelfile_path, &modelfile_content)
        .await
        .map_err(|e| anyhow!("Cannot write Modelfile: {}", e))?;

    let status = tokio::process::Command::new("ollama")
        .arg("create")
        .arg(model_name)
        .arg("-f")
        .arg(&modelfile_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| anyhow!("Failed to register model: {}", e))?;

    let _ = tokio::fs::remove_file(&modelfile_path).await;

    if !status.success() {
        return Err(anyhow!(
            "Could not register model '{}' with inference engine.",
            model_name
        ));
    }

    eprintln!("  Model ready.");
    Ok(())
}

async fn stream_from_ollama(
    model_name: &str,
    req: &InferenceRequest,
    tx: mpsc::UnboundedSender<String>,
) -> Result<(String, u32)> {
    let mut messages = vec![];

    if let Some(sys) = &req.system_prompt {
        messages.push(serde_json::json!({"role": "system", "content": sys}));
    }
    for m in &req.messages {
        messages.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    // Discover the actual name Ollama has for this model
    let ollama_name = resolve_actual_ollama_name(model_name).await;

    let body = serde_json::json!({
        "model": ollama_name,
        "messages": messages,
        "stream": true,
        "options": {
            "temperature": req.temperature,
            "num_predict": req.max_tokens,
        }
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/api/chat", OLLAMA_BASE))
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("Inference engine unreachable: {}", e))?;

    // Check for HTTP errors
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Inference engine error {}: {}\nModel used: {}",
            status, text, ollama_name
        ));
    }

    let mut full = String::new();
    let mut tokens = 0u32;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Ok(text) = std::str::from_utf8(&chunk) {
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(token) = val["message"]["content"].as_str() {
                        tx.send(token.to_string()).ok();
                        full.push_str(token);
                        tokens += 1;
                    }
                    if val["done"].as_bool().unwrap_or(false) {
                        break;
                    }
                    // Surface model errors from stream
                    if let Some(err) = val["error"].as_str() {
                        return Err(anyhow!("Model error: {}", err));
                    }
                }
            }
        }
    }

    if full.is_empty() {
        return Err(anyhow!(
            "Empty response from inference engine.\n\
             Model '{}' may have failed to load. Check: ollama list",
            ollama_name
        ));
    }

    Ok((full, tokens))
}

pub fn gguf_path_to_model_name(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");

    let lower = stem.to_lowercase();
    if lower.contains("tinyllama")                               { return "tinyllama:1b".to_string(); }
    if lower.contains("qwen2.5-coder") && lower.contains("7b")  { return "qwen2.5-coder:7b".to_string(); }
    if lower.contains("qwen2.5-coder") && lower.contains("3b")  { return "qwen2.5-coder:3b".to_string(); }
    if lower.contains("qwen2.5-coder") && lower.contains("14b") { return "qwen2.5-coder:14b".to_string(); }
    if lower.contains("llama-3") && lower.contains("8b")        { return "llama3:8b".to_string(); }
    if lower.contains("mistral")                                 { return "mistral:7b".to_string(); }
    if lower.contains("phi-3")                                   { return "phi3:mini".to_string(); }
    if lower.contains("deepseek")                                { return "deepseek-coder:6.7b".to_string(); }

    stem.to_string()
}