use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::state::*;

const MIMONA_API: &str = "http://localhost:11435";
const BRIDGE_API: &str = "http://localhost:3344";

pub async fn run_worker(
    mut cmd_rx: mpsc::UnboundedReceiver<UiCommand>,
    update_tx: UpdateSender,
) {
    // Start the Axum server and WhatsApp bridge automatically
    let tx = update_tx.clone();
    tokio::spawn(async move {
        start_server_and_bridge(tx).await;
    });

    // Shared flag for cancelling whichever model pull is currently running.
    // Only one pull can be in flight at a time (a single `pull_progress` slot
    // in AppState), so a single reusable flag is enough.
    let pull_cancel = Arc::new(AtomicBool::new(false));

    while let Some(cmd) = cmd_rx.recv().await {
        let tx = update_tx.clone();
        match cmd {
            UiCommand::SendMessage { model, messages, system } => {
                tokio::spawn(async move {
                    handle_chat(model, messages, system, tx).await;
                });
            }
            UiCommand::RefreshModels => {
                tokio::spawn(async move {
                    handle_refresh_models(tx).await;
                });
            }
            UiCommand::PullModel(name) => {
                pull_cancel.store(false, Ordering::SeqCst);
                let cancel = pull_cancel.clone();
                tokio::spawn(async move {
                    handle_pull_model(name, tx, cancel).await;
                });
            }
            UiCommand::CancelPull => {
                pull_cancel.store(true, Ordering::SeqCst);
            }
            UiCommand::DeleteModel(name) => {
                tokio::spawn(async move {
                    handle_delete_model(name, tx).await;
                });
            }
            UiCommand::StartWaSession => {
                tokio::spawn(async move {
                    handle_wa_start_session(tx).await;
                });
            }
            UiCommand::PollWaStatus(session_id) => {
                tokio::spawn(async move {
                    poll_wa_status(&session_id, tx).await;
                });
            }
            UiCommand::RefreshWaUsers => {
                tokio::spawn(async move {
                    handle_wa_refresh_users(tx).await;
                });
            }
            UiCommand::SaveWaPrompt { phone, prompt } => {
                tokio::spawn(async move {
                    handle_wa_save_prompt(phone, prompt, tx).await;
                });
            }
            UiCommand::SetWaModel { phone, model } => {
                tokio::spawn(async move {
                    handle_wa_set_model(phone, model, tx).await;
                });
            }
            UiCommand::UnlinkWa(phone) => {
                tokio::spawn(async move {
                    handle_wa_unlink(phone, tx).await;
                });
            }
            UiCommand::GenerateAnimation(prompt) => {
                tokio::spawn(async move {
                    handle_generate_animation(prompt, tx).await;
                });
            }
            UiCommand::CheckManimInstalled => {
                tokio::spawn(async move {
                    handle_check_manim(tx).await;
                });
            }
            UiCommand::CheckOllama => {
                tokio::spawn(async move {
                    handle_check_ollama(tx).await;
                });
            }
            UiCommand::CheckBridge => {
                tokio::spawn(async move {
                    let status = crate::whatsapp_bridge_launcher::check_status().await;
                    let _ = tx.send(WorkerUpdate::BridgeStatus(status));
                });
            }
            UiCommand::StartBridge => {
                let _ = tx.send(WorkerUpdate::BridgeStatus(
                    crate::whatsapp_bridge_launcher::BridgeStatus::Starting,
                ));
                tokio::spawn(async move {
                    let status = crate::whatsapp_bridge_launcher::start_bridge_and_wait().await;
                    let _ = tx.send(WorkerUpdate::BridgeStatus(status));
                });
            }
            UiCommand::InstallOllama => {
                // Open download page in browser
                let url = if cfg!(target_os = "windows") {
                    "https://ollama.com/download/windows"
                } else if cfg!(target_os = "macos") {
                    "https://ollama.com/download/mac"
                } else {
                    "https://ollama.com/download/linux"
                };
                let _ = tokio::process::Command::new(
                    if cfg!(target_os = "windows") { "cmd" } else { "xdg-open" }
                )
                .args(if cfg!(target_os = "windows") { vec!["/c", "start", url] } else { vec![url] })
                .spawn();
            }
             UiCommand::StartOllama => {
                tokio::spawn(async move {
                    handle_start_ollama(tx).await;
                });
            }
            UiCommand::DismissSetup => {
                let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::Running));
            }
            UiCommand::OpenBrowser(url) => {
                let _ = tokio::process::Command::new(
                    if cfg!(target_os = "windows") { "cmd" } else { "xdg-open" }
                )
                .args(if cfg!(target_os = "windows") { vec!["/c", "start", &url] } else { vec![url.as_str()] })
                .spawn();
            }
            UiCommand::OpenVideo(path) => {
                // Open video with system default player
                let _ = tokio::process::Command::new("xdg-open")
                    .arg(&path)
                    .spawn();
            }
            UiCommand::GenerateWebsite { brand, description, services, contact, site_type, color } => {
                tokio::spawn(async move {
                    handle_generate_website(brand, description, services, contact, site_type, color, tx).await;
                });
            }
            UiCommand::StopWebsite => {
                let _ = tx.send(WorkerUpdate::WebsiteStopped);
            }
            UiCommand::OpenBrowser(url) => {
                let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();
            }
            UiCommand::SaveWidgetSettings { bot_name, welcome, system_prompt, color } => {
                tokio::spawn(async move {
                    handle_save_widget_settings(bot_name, welcome, system_prompt, color).await;
                });
            }
               UiCommand::GenerateSocialContent { brand, topic, platforms, model } => {
                tokio::spawn(async move {
                    handle_automate_social(brand, topic, platforms, model, tx).await;
                });
            }
            UiCommand::GenerateColdEmails { product, audience, count, model } => {
                tokio::spawn(async move {
                    handle_automate_email(product, audience, count, model, tx).await;
                });
            }
            UiCommand::GenerateSeoContent { business, location, keywords, model } => {
                tokio::spawn(async move {
                    handle_automate_seo(business, location, keywords, model, tx).await;
                });
            }
            UiCommand::SaveWidgetSettings { bot_name, welcome, system_prompt, color } => {
                // Save to config file
                tokio::spawn(async move {
                    handle_save_widget_settings(bot_name, welcome, system_prompt, color).await;//tx).await;
                });
            }
            UiCommand::DeployWebsite => {
                // DeployWebsite is triggered from panel with brand already known
                // The panel sends GenerateWebsite first, then DeployWebsite separately
                // This is a no-op placeholder — deploy is triggered by a dedicated panel button
            }
            UiCommand::RefreshWallet => {
                tokio::spawn(async move {
                    handle_refresh_wallet(tx).await;
                });
            }
            UiCommand::CreateWallet => {
                tokio::spawn(async move {
                    handle_create_wallet(tx).await;
                });
            }
            UiCommand::StartServer => {
                // Already started on boot, ignore
            }
        }
    }
}

// ── Server bootstrap ──────────────────────────────────────────────────────────

async fn handle_check_ollama(tx: UpdateSender) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(r) if r.status().is_success() => {
            let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::Running));
        }
         _ => {
                // Check if ollama binary exists
            let exists = tokio::process::Command::new("ollama")
                .arg("--version")
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
 
            if exists {
                let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::NotRunning));
            } else {
                let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::NotInstalled));
            }
        }
    }
}
 
async fn handle_start_ollama(tx: UpdateSender) {
    let _ = tx.send(WorkerUpdate::StatusMessage("Starting Ollama…".into()));
 
    // Start ollama serve in background
    let result = tokio::process::Command::new("ollama")
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
 
    if result.is_err() {
        let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::NotInstalled));
        return;
    }
 
    // Wait up to 5 seconds for it to start
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap_or_default();
        if let Ok(r) = client.get("http://localhost:11434/api/tags").send().await {
            if r.status().is_success() {
                let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::Running));
                return;
            }
        }
    }
 
    let _ = tx.send(WorkerUpdate::OllamaStatus(OllamaStatus::NotRunning));
}


// async fn handle_save_widget_settings(
//     bot_name: String,
//     welcome: String,
//     system_prompt: String,
//     color: String,
//     tx: UpdateSender,
// ) {
//     // Save to ~/.mimona/widget_settings.json
//     let settings = serde_json::json!({
//         "bot_name": bot_name,
//         "welcome": welcome,
//         "system_prompt": system_prompt,
//         "color": color,
//     });
 
//     let path = dirs::home_dir()
//         .unwrap_or_default()
//         .join(".mimona")
//         .join("widget_settings.json");
 
//     tokio::fs::create_dir_all(path.parent().unwrap()).await.ok();
//     if let Ok(s) = serde_json::to_string_pretty(&settings) {
//         tokio::fs::write(&path, s).await.ok();
//     }
 
//     let _ = tx.send(WorkerUpdate::StatusMessage("Widget settings saved".into()));
// }
 
// ── 

async fn start_server_and_bridge(tx: UpdateSender) {
    // Start the Axum API server on a background task
    let port = 11435u16;
    let host = "127.0.0.1".to_string();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::server::api::start(host, port).await {
            let _ = tx2.send(WorkerUpdate::StatusMessage(
                format!("Server error: {}", e)
            ));
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let _ = tx.send(WorkerUpdate::ServerStarted(port));

    // Let the server settle, then load initial data
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    handle_refresh_models(tx.clone()).await;
    handle_wa_refresh_users(tx.clone()).await;
    handle_refresh_wallet(tx.clone()).await;
    handle_load_widget_settings(tx).await;
}

// ── Chat ──────────────────────────────────────────────────────────────────────

async fn handle_chat(
    model: String,
    messages: Vec<(String, String)>,
    system: String,
    tx: UpdateSender,
) {
    let client = reqwest::Client::new();

    let mut all_msgs = vec![serde_json::json!({"role": "system", "content": system})];
    for (role, content) in &messages {
        all_msgs.push(serde_json::json!({"role": role, "content": content}));
    }

    // Find actual model name from Ollama
    let ollama_model = resolve_ollama_model(&model).await.unwrap_or(model.clone());

    let body = serde_json::json!({
        "model": ollama_model,
        "messages": all_msgs,
        "stream": true,
        "options": { "temperature": 0.7, "num_predict": 1024 }
    });

    let resp = match client
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::ChatError(format!("Connection failed: {}", e)));
            return;
        }
    };

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let _ = tx.send(WorkerUpdate::ChatError(format!("Model error: {}", text)));
        return;
    }

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(WorkerUpdate::ChatError(e.to_string()));
                return;
            }
        };
        if let Ok(text) = std::str::from_utf8(&chunk) {
            for line in text.lines() {
                if line.trim().is_empty() { continue; }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(token) = val["message"]["content"].as_str() {
                        let _ = tx.send(WorkerUpdate::ChatToken(token.to_string()));
                    }
                    if val["done"].as_bool().unwrap_or(false) {
                        let _ = tx.send(WorkerUpdate::ChatDone);
                        return;
                    }
                }
            }
        }
    }
    let _ = tx.send(WorkerUpdate::ChatDone);
}


async fn ollama_generate_text(model: &str, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
 
    let ollama_model = resolve_ollama_model(model).await.unwrap_or(model.to_string());
 
    let body = serde_json::json!({
        "model": ollama_model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "temperature": 0.8, "num_predict": 2048 }
    });
 
    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;
 
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Model error: {}", text));
    }
 
    let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(val["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

async fn resolve_ollama_model(name: &str) -> Option<String> {
    let resp = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;
    let models: Vec<String> = data["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
        .collect();

    let base = name.split(':').next().unwrap_or(name);
    if models.contains(&name.to_string()) {
        return Some(name.to_string());
    }
    models.into_iter().find(|m| m.starts_with(base))
}

// ── Models ────────────────────────────────────────────────────────────────────

async fn handle_refresh_models(tx: UpdateSender) {
    match crate::models::storage::load_local_models().await {
        Ok(models) => {
            let local: Vec<LocalModel> = models
                .into_iter()
                .map(|m| LocalModel {
                    name: m.name.clone(),
                    tag: m.tag.clone(),
                    size_gb: m.size_bytes as f32 / 1_073_741_824.0,
                })
                .collect();
            let _ = tx.send(WorkerUpdate::ModelsLoaded(local));
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::StatusMessage(format!("Failed to load models: {}", e)));
        }
    }
}

async fn handle_pull_model(name: String, tx: UpdateSender, cancel: Arc<AtomicBool>) {
    let (model_name, tag_opt) = crate::models::ModelEntry::parse_name_tag(&name);

    let entry = match crate::models::registry::find_model(&model_name).await {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::PullError(format!("Model not found: {}", e)));
            return;
        }
    };

    let tag = tag_opt.unwrap_or_else(|| entry.default_tag.clone());
    let variant = match entry.variants.get(&tag) {
        Some(v) => v.clone(),
        None => {
            let _ = tx.send(WorkerUpdate::PullError(format!("Tag '{}' not found", tag)));
            return;
        }
    };

    let full_name = format!("{}:{}", model_name, tag);
    let fallback_total_gb = variant.size_gb as f32;

    let _ = tx.send(WorkerUpdate::PullProgress(PullProgress {
        model: full_name.clone(),
        downloaded_gb: 0.0,
        total_gb: fallback_total_gb,
        done: false,
    }));

    // Report progress as it streams in, but throttle to avoid flooding the
    // UI channel with one message per network chunk.
    let progress_tx = tx.clone();
    let progress_name = full_name.clone();
    let mut last_reported: u64 = 0;
    const REPORT_EVERY_BYTES: u64 = 4 * 1024 * 1024; // 4 MB

    let on_progress = move |downloaded: u64, total: u64| {
        let is_done = total > 0 && downloaded >= total;
        if !is_done && downloaded.saturating_sub(last_reported) < REPORT_EVERY_BYTES {
            return;
        }
        last_reported = downloaded;

        let downloaded_gb = downloaded as f32 / 1_073_741_824.0;
        let total_gb = if total > 0 {
            total as f32 / 1_073_741_824.0
        } else {
            fallback_total_gb
        };

        let _ = progress_tx.send(WorkerUpdate::PullProgress(PullProgress {
            model: progress_name.clone(),
            downloaded_gb,
            total_gb,
            done: false,
        }));
    };

    match crate::models::downloader::download_model_ui(&entry, &tag, cancel, on_progress).await {
        Ok(_) => {
            let _ = tx.send(WorkerUpdate::PullDone(full_name));
            handle_refresh_models(tx).await;
        }
        Err(e) if e.to_string() == "cancelled" => {
            let _ = tx.send(WorkerUpdate::PullCancelled);
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::PullError(e.to_string()));
        }
    }
}

async fn handle_delete_model(name: String, tx: UpdateSender) {
    let (model_name, tag_opt) = crate::models::ModelEntry::parse_name_tag(&name);
    let tag = tag_opt.unwrap_or_else(|| "latest".to_string());

    match crate::models::storage::remove_model(&model_name, &tag).await {
        Ok(_) => {
            let _ = tx.send(WorkerUpdate::ModelDeleted(name));
            handle_refresh_models(tx).await;
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::StatusMessage(format!("Delete failed: {}", e)));
        }
    }
}

// ── WhatsApp ──────────────────────────────────────────────────────────────────

async fn handle_wa_start_session(tx: UpdateSender) {
    // Check first instead of letting a raw "connection refused" surface —
    // this is what previously showed as "Bridge unreachable: error sending
    // request for url (http://localhost:3344/baileys/start)" with no
    // indication of what to actually do about it.
    match crate::whatsapp_bridge_launcher::check_status().await {
        crate::whatsapp_bridge_launcher::BridgeStatus::Running => {}
        status => {
            let _ = tx.send(WorkerUpdate::BridgeStatus(status));
            return;
        }
    }

    let client = reqwest::Client::new();
    match client
        .post(format!("{}/baileys/start", BRIDGE_API))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(sid) = data["session_id"].as_str() {
                    let _ = tx.send(WorkerUpdate::WaSessionId(sid.to_string()));
                }
            }
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WaError(format!("Bridge unreachable: {}", e)));
        }
    }
}

pub async fn poll_wa_status(session_id: &str, tx: UpdateSender) {
    let client = reqwest::Client::new();
    let url = format!("{}/baileys/status?session_id={}", BRIDGE_API,
        urlencoding::encode(session_id));
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let state = data["state"].as_str().unwrap_or("unknown").to_string();
                if let Some(qr) = data["qr"].as_str() {
                    let _ = tx.send(WorkerUpdate::WaQr(qr.to_string()));
                }
                if state == "connected" {
                    if let Some(phone) = data["phone_number"].as_str() {
                        let _ = tx.send(WorkerUpdate::WaConnected(phone.to_string()));
                    }
                } else if state == "disconnected" {
                    let _ = tx.send(WorkerUpdate::WaDisconnected);
                }
            }
        }
        Err(_) => {
            let _ = tx.send(WorkerUpdate::WaError("Bridge unreachable".into()));
        }
    }
}

async fn handle_wa_refresh_users(tx: UpdateSender) {
    let client = reqwest::Client::new();
    match client
        .get(format!("{}/api/whatsapp/users", MIMONA_API))
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let users: Vec<WaUser> = data["users"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|u| WaUser {
                        phone_number: u["phone_number"].as_str().unwrap_or("").to_string(),
                        status: u["status"].as_str().unwrap_or("unknown").to_string(),
                        system_prompt: u["system_prompt"].as_str().unwrap_or("").to_string(),
                        model: u["model"].as_str().unwrap_or("tinyllama:1b").to_string(),
                    })
                    .collect();
                let _ = tx.send(WorkerUpdate::WaUsers(users));
            }
        }
        Err(_) => {
            let _ = tx.send(WorkerUpdate::WaUsers(vec![]));
        }
    }
}

async fn handle_wa_save_prompt(phone: String, prompt: String, tx: UpdateSender) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/whatsapp/users/{}/prompt", MIMONA_API,
        urlencoding::encode(&phone));
    match client
        .put(&url)
        .json(&serde_json::json!({"system_prompt": prompt}))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            let _ = tx.send(WorkerUpdate::WaPromptSaved);
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            let _ = tx.send(WorkerUpdate::WaError(format!("Save failed: {}", text)));
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WaError(e.to_string()));
        }
    }
}

async fn handle_wa_set_model(phone: String, model: String, tx: UpdateSender) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/whatsapp/users/{}/model", MIMONA_API,
        urlencoding::encode(&phone));
    let _ = client
        .put(&url)
        .json(&serde_json::json!({"model": model}))
        .send()
        .await;
    handle_wa_refresh_users(tx).await;
}

async fn handle_wa_unlink(phone: String, tx: UpdateSender) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/whatsapp/users/{}", MIMONA_API,
        urlencoding::encode(&phone));
    let _ = client.delete(&url).send().await;
    handle_wa_refresh_users(tx).await;
}

// ── Wallet ────────────────────────────────────────────────────────────────────

async fn handle_refresh_wallet(tx: UpdateSender) {
    match crate::payment::wallet::load_wallet().await {
        Ok(w) => {
            let balance = crate::payment::wallet::get_balance(&w.address)
                .await
                .unwrap_or(0.0);
            let _ = tx.send(WorkerUpdate::WalletInfo {
                address: w.address,
                balance,
            });
        }
        Err(_) => {
            // No wallet yet — UI will show "Create wallet" button
        }
    }
}

async fn handle_create_wallet(tx: UpdateSender) {
    match crate::payment::wallet::create_wallet().await {
        Ok(w) => {
            let _ = tx.send(WorkerUpdate::WalletCreated(w.address));
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WalletError(e.to_string()));
        }
    }
}

async fn handle_check_manim(tx: UpdateSender) {
    let result = tokio::process::Command::new("python3")
        .args(["-c", "import manim; print('ok')"])
        .output()
        .await;
    let installed = result.map(|o| o.status.success()).unwrap_or(false);
    let _ = tx.send(WorkerUpdate::ManimInstalled(installed));
}

async fn handle_generate_animation(prompt: String, tx: UpdateSender) {
    let system = r#"You are an expert at writing Manim Community Edition Python animations.
When given a description, write ONLY valid Python code using Manim CE.

Rules:
- Always start with: from manim import *
- Define exactly one Scene class named GeneratedScene
- Use self.play() and self.wait() for animations
- Keep animations under 15 seconds total
- Output ONLY the Python code, no explanation, no markdown fences, no backticks"#;

    let user_msg = format!("Create a Manim animation: {}", prompt);

    let ollama_model = resolve_ollama_model("tinyllama:1b").await
        .unwrap_or_else(|| "tinyllama:1b".to_string());

    let body = serde_json::json!({
        "model": ollama_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_msg}
        ],
        "stream": false,
        "options": {"temperature": 0.3, "num_predict": 2048}
    });

    let resp = match reqwest::Client::new()
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::AnimError(format!("AI unreachable: {}", e)));
            return;
        }
    };

    let data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::AnimError(format!("Bad AI response: {}", e)));
            return;
        }
    };

    let raw_code = data["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let code = strip_code_fences(&raw_code);

    if code.trim().is_empty() {
        let _ = tx.send(WorkerUpdate::AnimError("AI returned empty code".to_string()));
        return;
    }

    let _ = tx.send(WorkerUpdate::AnimCodeGenerated(code.clone()));
    let _ = tx.send(WorkerUpdate::AnimRendering);

    let anim_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mimona")
        .join("animations");

    tokio::fs::create_dir_all(&anim_dir).await.ok();

    let scene_file = anim_dir.join("generated_scene.py");
    let output_dir = anim_dir.join("output");
    tokio::fs::create_dir_all(&output_dir).await.ok();

    if let Err(e) = tokio::fs::write(&scene_file, &code).await {
        let _ = tx.send(WorkerUpdate::AnimError(format!("Could not write scene: {}", e)));
        return;
    }

    let result = tokio::process::Command::new("python3")
        .args([
            "-m", "manim",
            scene_file.to_str().unwrap_or(""),
            "GeneratedScene",
            "--format=mp4",
            "--media_dir", output_dir.to_str().unwrap_or(""),
            "-q", "m",
            "--disable_caching",
        ])
        .output()
        .await;

    match result {
        Ok(out) if out.status.success() => {
            match find_output_video(&output_dir).await {
                Some(path) => { let _ = tx.send(WorkerUpdate::AnimDone(path)); }
                None => {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let _ = tx.send(WorkerUpdate::AnimError(
                        format!("No video found.\n{}", &stderr[..stderr.len().min(300)])
                    ));
                }
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let _ = tx.send(WorkerUpdate::AnimError(
                format!("Render failed:\n{}", &stderr[..stderr.len().min(500)])
            ));
        }
        Err(e) => {
            let _ = tx.send(WorkerUpdate::AnimError(
                format!("Could not run manim: {}\n\nInstall with: pip install manim", e)
            ));
        }
    }
}

fn strip_code_fences(code: &str) -> String {
    let mut lines: Vec<&str> = code.lines().collect();
    if lines.first().map(|l| l.starts_with("```")).unwrap_or(false) {
        lines.remove(0);
    }
    if lines.last().map(|l| l.trim() == "```").unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}
 
async fn handle_automate_social(
    brand: String,
    topic: String,
    platforms: String,
    model: String,
    tx: UpdateSender,
) {
    let prompt = format!(
        r#"You are a professional social media content creator.
 
Brand: {brand}
Topic: {topic}
Platforms: {platforms}
 
Create ready-to-post content for each platform listed. For each platform write:
- A headline
- The post body (with emojis where appropriate)
- 5–10 relevant hashtags
 
Separate each platform's content with a clear header like "── Instagram ──".
Keep the tone engaging, concise, and on-brand. Do not add commentary — output the posts only."#
    );
 
    match ollama_generate_text(&model, &prompt).await {
        Ok(result) => { let _ = tx.send(WorkerUpdate::AutomateDone { tool: "social".into(), result }); }
        Err(e) => { let _ = tx.send(WorkerUpdate::AutomateError { tool: "social".into(), error: e }); }
    }
}
 
async fn handle_automate_email(
    product: String,
    audience: String,
    count: u32,
    model: String,
    tx: UpdateSender,
) {
    let prompt = format!(
        r#"You are an expert cold email copywriter.
 
Product/Service: {product}
Target Audience: {audience}
Number of emails to write: {count}
 
Write {count} distinct cold email variants. Each email should have:
- Subject line (labeled "Subject:")
- Body (3–5 short paragraphs: hook, problem, solution, CTA)
- A clear call-to-action
 
Number each email (Email 1, Email 2, ...). Vary the angle and hook for each.
Output only the emails — no commentary or preamble."#
    );
 
    match ollama_generate_text(&model, &prompt).await {
        Ok(result) => { let _ = tx.send(WorkerUpdate::AutomateDone { tool: "email".into(), result }); }
        Err(e) => { let _ = tx.send(WorkerUpdate::AutomateError { tool: "email".into(), error: e }); }
    }
}
 
async fn handle_automate_seo(
    business: String,
    location: String,
    keywords: String,
    model: String,
    tx: UpdateSender,
) {
    let prompt = format!(
        r#"You are a local SEO content specialist.
 
Business: {business}
Location: {location}
Keywords to target: {keywords}
 
Write a complete local SEO content package:
1. A 300-word blog post optimised for the keywords above
2. A 100-word "About Us" paragraph naturally using the location and keywords
3. 5 FAQ entries (Q&A format) that customers commonly search for
4. A 60-word Google Business description
 
Label each section clearly. Write naturally — avoid keyword stuffing.
Output only the content — no meta-commentary."#
    );
 
    match ollama_generate_text(&model, &prompt).await {
        Ok(result) => { let _ = tx.send(WorkerUpdate::AutomateDone { tool: "seo".into(), result }); }
        Err(e) => { let _ = tx.send(WorkerUpdate::AutomateError { tool: "seo".into(), error: e }); }
    }
}
 

async fn find_output_video(output_dir: &std::path::Path) -> Option<String> {
    let mut stack = vec![output_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().map(|e| e == "mp4").unwrap_or(false) {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

// ── Website generation & deployment ──────────────────────────────────────────

pub async fn handle_generate_website(
    brand: String,
    description: String,
    services: String,
    contact: String,
    site_type: String,
    color: String,
    tx: UpdateSender,
) {
    let system = r#"You are an expert web designer who writes beautiful, modern single-file HTML websites.
Output ONLY valid HTML — no markdown, no explanation, no backticks.
The HTML must be complete and self-contained: all CSS in <style> tags, no external dependencies except Google Fonts.
Make it look professional, modern, and mobile-responsive."#;

    let type_instructions = match site_type.as_str() {
        "Landing page" => "Create a focused landing page with: hero section with strong headline and CTA button, benefits/features section, social proof, and contact form.",
        "Multi-section" => "Create a full business site with sticky nav, hero, about, services grid, testimonials placeholder, and contact section.",
        _ => "Create a single-page site with: hero, about section, services/offerings, and contact details at the bottom.",
    };

    let prompt = format!(
        r#"Create a complete, beautiful HTML website for this business:

Brand: {}
Description: {}
Services/Products: {}
Contact: {}
Site type: {}
Primary color: {} (use this as the main accent color)

{}

Design requirements:
- Use Google Fonts (Inter or Plus Jakarta Sans)
- Clean, modern design with lots of white space
- Mobile responsive with media queries
- Smooth scroll behavior
- Primary color {} for buttons, accents, headings
- Professional hover effects on buttons and cards
- Include a sticky navigation bar
- Footer with copyright

Output ONLY the complete HTML file starting with <!DOCTYPE html>"#,
        brand, description, services, contact, site_type, color,
        type_instructions, color
    );

    let ollama_model = resolve_ollama_model("tinyllama:1b").await
        .unwrap_or_else(|| "tinyllama:1b".to_string());

    let body = serde_json::json!({
        "model": ollama_model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ],
        "stream": false,
        "options": {"temperature": 0.4, "num_predict": 4096}
    });

    let resp = match reqwest::Client::new()
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WebsiteError(format!("AI unreachable: {}", e)));
            return;
        }
    };

    let data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WebsiteError(format!("Bad response: {}", e)));
            return;
        }
    };

    let raw = data["message"]["content"].as_str().unwrap_or("").to_string();

    // Strip any markdown fences if the model added them
    let html = {
        let s = raw.trim();
        let s = s.strip_prefix("```html").unwrap_or(s);
        let s = s.strip_prefix("```").unwrap_or(s);
        let s = if let Some(end) = s.rfind("```") { &s[..end] } else { s };
        s.trim().to_string()
    };

    if html.is_empty() || !html.contains("<html") {
        let _ = tx.send(WorkerUpdate::WebsiteError(
            "AI didn't return valid HTML. Try a more capable model (mistral:7b or llama3:8b).".to_string()
        ));
        return;
    }

    // Save to ~/.mimona/sites/<brand>/index.html
    let safe_name = brand.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let site_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".mimona")
        .join("sites")
        .join(&safe_name);

    tokio::fs::create_dir_all(&site_dir).await.ok();

    let index_path = site_dir.join("index.html");
    if let Err(e) = tokio::fs::write(&index_path, &html).await {
        let _ = tx.send(WorkerUpdate::WebsiteError(format!("Could not save site: {}", e)));
        return;
    }

    let _ = tx.send(WorkerUpdate::WebsiteGenerated(html));
}

pub async fn handle_deploy_website(brand: String, tx: UpdateSender) {
    let safe_name = brand.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();

    let site_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".mimona")
        .join("sites")
        .join(&safe_name);

    let local_port = 11436u16;

    // Start a simple HTTP server serving the site directory
    let site_dir_clone = site_dir.clone();
    tokio::spawn(async move {
        use axum::{Router, routing::get_service};
        use tower_http::services::ServeDir;

        let app = Router::new()
            .fallback_service(
                get_service(ServeDir::new(&site_dir_clone)
                    .append_index_html_on_directories(true))
            );

        let addr = format!("127.0.0.1:{}", local_port);
        if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
            let _ = axum::serve(listener, app).await;
        }
    });

    // Give the server a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Check if cloudflared is available
    let cf_check = tokio::process::Command::new("cloudflared")
        .arg("--version")
        .output()
        .await;

    if cf_check.is_err() {
        // cloudflared not installed — just give local URL
        let _ = tx.send(WorkerUpdate::WebsiteDeployed {
            local_port,
            public_url: format!(
                "http://localhost:{} (install cloudflared for a public URL)",
                local_port
            ),
        });
        return;
    }

    // Start cloudflared tunnel
    let _ = tx.send(WorkerUpdate::StatusMessage("Starting public tunnel…".into()));

    let mut child = match tokio::process::Command::new("cloudflared")
        .args(["tunnel", "--url", &format!("http://localhost:{}", local_port)])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(WorkerUpdate::WebsiteDeployed {
                local_port,
                public_url: format!("http://localhost:{}", local_port),
            });
            return;
        }
    };

    // Read cloudflared stderr to find the public URL
    // cloudflared prints: "https://xxxx.trycloudflare.com"
    if let Some(mut stderr) = child.stderr.take() {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut reader = BufReader::new(stderr).lines();
        let mut found_url: Option<String> = None;

        // Wait up to 15 seconds for the URL to appear
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                reader.next_line()
            ).await {
                Ok(Ok(Some(line))) => {
                    if line.contains("trycloudflare.com") {
                        // Extract the URL
                        if let Some(url) = line.split_whitespace()
                            .find(|s| s.contains("trycloudflare.com"))
                        {
                            found_url = Some(url.to_string());
                            break;
                        }
                    }
                    // Also check for the tunnel URL format
                    if line.contains("https://") && line.contains(".com") {
                        let words: Vec<&str> = line.split_whitespace().collect();
                        for word in &words {
                            if word.starts_with("https://") {
                                found_url = Some(word.to_string());
                                break;
                            }
                        }
                        if found_url.is_some() { break; }
                    }
                }
                _ => {}
            }
        }

        let public_url = found_url.unwrap_or_else(|| format!("http://localhost:{}", local_port));
        let _ = tx.send(WorkerUpdate::WebsiteDeployed { local_port, public_url });
    }
}

// ── Widget / Embed ────────────────────────────────────────────────────────────

async fn handle_save_widget_settings(
    bot_name: String,
    welcome: String,
    system_prompt: String,
    color: String,
) {
    // Load existing settings first so blank fields (the user left a hint
    // placeholder untouched) don't overwrite a good default with "".
    let mut settings = crate::widget::WidgetSettings::load().await;

    if !bot_name.trim().is_empty() {
        settings.bot_name = bot_name;
    }
    if !welcome.trim().is_empty() {
        settings.welcome = welcome;
    }
    if !system_prompt.trim().is_empty() {
        settings.system_prompt = system_prompt;
    }
    if !color.trim().is_empty() {
        settings.color = color;
    }

    let _ = settings.save().await;
}

async fn handle_load_widget_settings(tx: UpdateSender) {
    let settings = crate::widget::WidgetSettings::load().await;
    let _ = tx.send(WorkerUpdate::WidgetSettingsLoaded {
        bot_name: settings.bot_name,
        welcome: settings.welcome,
        system_prompt: settings.system_prompt,
        color: settings.color,
    });
}