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


// ── Widget / Embed ────────────────────────────────────────────────────────────

pub async fn widget_js() -> impl IntoResponse {
    let settings = crate::widget::WidgetSettings::load().await;
    let color = settings.color;
    let bot_name = settings.bot_name;

    let js = format!(r#"(function() {{
  var COLOR = {color:?};
  var BOT_NAME = {bot_name:?};
  var origin = (function() {{
    var s = document.currentScript;
    if (s && s.src) {{ try {{ return new URL(s.src).origin; }} catch (e) {{}} }}
    return '';
  }})();

  var btn = document.createElement('div');
  btn.innerHTML = '💬';
  btn.title = BOT_NAME;
  btn.style.cssText = 'position:fixed;bottom:20px;right:20px;width:56px;height:56px;' +
    'border-radius:50%;background:' + COLOR + ';color:#fff;display:flex;' +
    'align-items:center;justify-content:center;font-size:24px;cursor:pointer;' +
    'box-shadow:0 4px 16px rgba(0,0,0,.25);z-index:999999;';
  document.body.appendChild(btn);

  var iframe = null;
  btn.addEventListener('click', function() {{
    if (!iframe) {{
      iframe = document.createElement('iframe');
      iframe.src = origin + '/widget';
      iframe.style.cssText = 'position:fixed;bottom:88px;right:20px;width:360px;' +
        'max-width:92vw;height:520px;max-height:80vh;border:none;border-radius:16px;' +
        'box-shadow:0 10px 40px rgba(0,0,0,.25);z-index:999998;background:#fff;';
      document.body.appendChild(iframe);
    }} else {{
      iframe.style.display = (iframe.style.display === 'none') ? 'block' : 'none';
    }}
  }});
}})();
"#);

    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        js,
    ).into_response()
}

pub async fn widget_page() -> impl IntoResponse {
    let settings = crate::widget::WidgetSettings::load().await;
    let bot_name = settings.bot_name;
    let welcome = settings.welcome;
    let color = settings.color;

    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{bot_name}</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{ margin:0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; display:flex; flex-direction:column; height:100vh; background:#fff; }}
  header {{ background:{color}; color:#fff; padding:14px 16px; font-weight:600; font-size:15px; }}
  #msgs {{ flex:1; overflow-y:auto; padding:12px; display:flex; flex-direction:column; gap:8px; }}
  .msg {{ max-width:80%; padding:8px 12px; border-radius:14px; font-size:13.5px; line-height:1.4; white-space:pre-wrap; }}
  .bot {{ align-self:flex-start; background:#f1f1f3; color:#111; }}
  .user {{ align-self:flex-end; background:{color}; color:#fff; }}
  #inputRow {{ display:flex; border-top:1px solid #eee; padding:10px; gap:8px; }}
  #inputRow input {{ flex:1; border:1px solid #ddd; border-radius:20px; padding:9px 14px; font-size:13.5px; outline:none; }}
  #inputRow button {{ background:{color}; color:#fff; border:none; border-radius:20px; padding:0 16px; font-size:13.5px; cursor:pointer; }}
</style>
</head>
<body>
<header>{bot_name}</header>
<div id="msgs"></div>
<div id="inputRow">
  <input id="inputBox" type="text" placeholder="Type a message…" />
  <button id="sendBtn">Send</button>
</div>
<script>
  var msgsEl = document.getElementById('msgs');
  var history = [];

  function addMsg(role, text) {{
    var div = document.createElement('div');
    div.className = 'msg ' + (role === 'user' ? 'user' : 'bot');
    div.textContent = text;
    msgsEl.appendChild(div);
    msgsEl.scrollTop = msgsEl.scrollHeight;
  }}

  addMsg('bot', {welcome:?});

  async function send() {{
    var input = document.getElementById('inputBox');
    var text = input.value.trim();
    if (!text) return;
    input.value = '';
    addMsg('user', text);
    history.push({{ role: 'user', content: text }});

    try {{
      var res = await fetch('/api/widget/chat', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{ messages: history }})
      }});
      var data = await res.json();
      var reply = data.reply || "Sorry, something went wrong.";
      addMsg('bot', reply);
      history.push({{ role: 'assistant', content: reply }});
    }} catch (e) {{
      addMsg('bot', "Sorry, I couldn't connect. Please try again.");
    }}
  }}

  document.getElementById('sendBtn').addEventListener('click', send);
  document.getElementById('inputBox').addEventListener('keydown', function(e) {{
    if (e.key === 'Enter') send();
  }});
</script>
</body>
</html>"#);

    axum::response::Html(html)
}

#[derive(Debug, Deserialize)]
pub struct WidgetChatRequest {
    pub messages: Vec<OaiMessage>,
}

pub async fn widget_chat(Json(body): Json<WidgetChatRequest>) -> impl IntoResponse {
    let settings = crate::widget::WidgetSettings::load().await;

    let ollama_model = resolve_ollama_model(&settings.model).await
        .unwrap_or(settings.model);

    let mut messages = vec![json!({"role": "system", "content": settings.system_prompt})];
    for m in &body.messages {
        messages.push(json!({"role": m.role, "content": m.content}));
    }

    let ollama_body = json!({
        "model": ollama_model,
        "messages": messages,
        "stream": false,
        "options": { "temperature": 0.7, "num_predict": 512 }
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
            return Json(json!({
                "reply": format!("Sorry, I couldn't reach the AI right now ({}).", e)
            })).into_response();
        }
    };

    let data: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => {
            return Json(json!({ "reply": "Sorry, something went wrong." })).into_response();
        }
    };

    let reply = data["message"]["content"]
        .as_str()
        .unwrap_or("Sorry, I don't have a response for that.")
        .to_string();

    Json(json!({ "reply": reply })).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn web_search(axum::extract::Query(params): axum::extract::Query<SearchQuery>) -> impl IntoResponse {
    let query = &params.q;

    let api_key = match std::env::var("TAVILY_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            println!("[search] TAVILY_API_KEY not set — set it as an environment variable before starting mimona serve.");
            return Json(json!({"query": query, "results": [], "error": "Search not configured"})).into_response();
        }
    };

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