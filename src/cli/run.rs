use crate::inference::engine::run_streaming;
use crate::inference::{ChatMessage, InferenceRequest};
use crate::models::{registry::find_model, storage::is_downloaded, ModelEntry, ModelTier};
use crate::payment::verify::check_and_charge;
use anyhow::{anyhow, Result};
use colored::*;
use std::io::{self, Write};
use tokio::sync::mpsc;

pub async fn handle(
    model_name: String,
    system: Option<String>,
    temperature: f32,
    max_tokens: u32,
) -> Result<()> {
    let (name, tag_opt) = ModelEntry::parse_name_tag(&model_name);
    let entry = find_model(&name).await?;
    let tag = tag_opt.as_deref().unwrap_or(&entry.default_tag).to_string();

    let variant = entry
        .variants
        .get(&tag)
        .ok_or_else(|| anyhow!("Tag '{}' not found", tag))?;

    if !is_downloaded(&name, &tag).await {
        println!("\n  {} Model not found locally. Download it first:\n", "✗".red());
        println!("    mimona pull {}:{}\n", name, tag);
        return Err(anyhow!("Model not downloaded"));
    }

    println!();
    println!("  {} {}:{}", "Running".cyan().bold(), name, tag);
    println!(
        "  {} {}",
        "Tier:".dimmed(),
        if entry.tier == ModelTier::Free {
            "FREE".green().bold()
        } else {
            format!("PAID ({} SOL/query)", variant.price_sol).yellow().bold()
        }
    );
    println!("  {} Web search + file reading enabled", "Tools:".dimmed());
    println!();
    println!("  Type your message. {} to exit.", "Ctrl+C".dimmed());
    println!("  Commands: {} clear context  {} read a file\n", "/clear".dimmed(), "/file <path>".dimmed());
    println!("{}", "─".repeat(60).dimmed());

    let model_path = crate::config::models_dir().join(&variant.filename);
    let mut history: Vec<ChatMessage> = vec![];
    let ollama_model = get_ollama_model().await;

    loop {
        print!("\n  {} ", ">>>".cyan().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() { continue; }

        if matches!(input.as_str(), "/exit" | "/quit" | "exit" | "quit") {
            println!("\n  Goodbye!\n");
            break;
        }

        if input == "/clear" {
            history.clear();
            println!("  {} Context cleared.", "✓".green());
            continue;
        }

        // /file <path> — read a file and inject into context
        if input.starts_with("/file ") {
            let path = input.trim_start_matches("/file ").trim();
            match read_file(path).await {
                Ok(contents) => {
                    let msg = format!("File contents of '{}':\n```\n{}\n```", path, contents);
                    println!("  {} Loaded {} ({} chars)", "✓".green(), path, contents.len());
                    history.push(ChatMessage { role: "user".into(), content: msg });
                    history.push(ChatMessage {
                        role: "assistant".into(),
                        content: format!("I've read the file '{}'. What would you like to do with it?", path),
                    });
                }
                Err(e) => println!("  {} Could not read file: {}", "✗".red(), e),
            }
            continue;
        }

        if entry.tier == ModelTier::Paid {
            match check_and_charge(variant.price_sol).await {
                Ok(_) => {}
                Err(e) => {
                    println!("\n  {} Payment failed: {}\n", "✗".red(), e);
                    continue;
                }
            }
        }

        // --- MIMONA-CONTROLLED SEARCH ---
        // Mimona decides if a search is needed — doesn't rely on model output
        let enriched_input = if should_search(&input) {
            let query = build_search_query(&input);
            print!("\n  {} {}", "🔍".cyan(), query.dimmed());
            io::stdout().flush()?;

            match web_search(&query).await {
                Some(results) => {
                    println!(" {}", "✓".green());
                    format!(
                        "{}\n\n[Web search results for '{}']\n{}\n[End of search results]",
                        input, query, results
                    )
                }
                None => {
                    println!(" {}", "no results".red());
                    input.clone()
                }
            }
        } else {
            input.clone()
        };

        history.push(ChatMessage { role: "user".into(), content: enriched_input });

        print!("\n  {}", "Mimona: ".green().bold());
        io::stdout().flush()?;

        let response = call_model(
            &history,
            &model_path,
            &ollama_model,
            temperature,
            max_tokens,
            &system,
        ).await?;

        history.push(ChatMessage { role: "assistant".into(), content: response });

        println!();
        println!("{}", "─".repeat(60).dimmed());
    }

    Ok(())
}

// ─── Search Decision Engine ───────────────────────────────────────────────────

/// Mimona decides if a query needs live data — no model involvement
fn should_search(input: &str) -> bool {
    let lower = input.to_lowercase();

    // Explicit search requests
    if lower.contains("search") || lower.contains("look up") || lower.contains("find out") {
        return true;
    }

    // Time-sensitive topics
    let live_keywords = [
        "price", "cost", "rate", "exchange", "naira", "dollar", "bitcoin", "crypto",
        "ethereum", "stock", "market", "today", "now", "current", "latest", "recent",
        "news", "weather", "score", "result", "winner", "who won", "happening",
        "2024", "2025", "2026", "this week", "this month", "right now",
        "how much", "what is the", "who is the", "when did", "did they",
    ];

    for kw in &live_keywords {
        if lower.contains(kw) {
            return true;
        }
    }

    // Question mark + factual question pattern
    if lower.ends_with('?') {
        let fact_starters = ["what", "who", "when", "where", "how much", "how many", "is there", "does"];
        for s in &fact_starters {
            if lower.starts_with(s) {
                return true;
            }
        }
    }

    false
}

/// Build an optimized search query from user input
fn build_search_query(input: &str) -> String {
    // Strip filler phrases to get a clean query
    let lower = input.to_lowercase();
    let clean = lower
        .trim_start_matches("can you search for ")
        .trim_start_matches("search for ")
        .trim_start_matches("search ")
        .trim_start_matches("look up ")
        .trim_start_matches("find out ")
        .trim_start_matches("what is the ")
        .trim_start_matches("what is ")
        .trim_start_matches("tell me the ")
        .trim_end_matches('?')
        .trim()
        .to_string();

    // Keep it short for better search results
    let words: Vec<&str> = clean.split_whitespace().take(8).collect();
    words.join(" ")
}

// ─── File Reading ─────────────────────────────────────────────────────────────

async fn read_file(path: &str) -> Result<String> {
    let path = std::path::Path::new(path);

    if !path.exists() {
        return Err(anyhow!("File not found: {}", path.display()));
    }

    let size = std::fs::metadata(path)?.len();
    if size > 2_000_000 {
        return Err(anyhow!("File too large (max 2MB for context)"));
    }

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "pdf" => read_pdf(path).await,
        _ => {
            // Plain text — works for .txt .rs .py .js .md .json .csv etc.
            let contents = tokio::fs::read_to_string(path).await
                .map_err(|e| anyhow!("Cannot read file: {}", e))?;
            // Truncate if very long
            if contents.len() > 50_000 {
                Ok(format!("{}\n\n[... file truncated at 50,000 chars ...]", &contents[..50_000]))
            } else {
                Ok(contents)
            }
        }
    }
}

async fn read_pdf(path: &std::path::Path) -> Result<String> {
    // Try pdftotext (poppler) if available
    let output = tokio::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            if text.len() > 50_000 {
                Ok(format!("{}\n\n[... truncated ...]", &text[..50_000]))
            } else {
                Ok(text)
            }
        }
        _ => {
            // pdftotext not available — read raw bytes and note it
            Err(anyhow!(
                "PDF reading requires 'pdftotext' (install: sudo apt install poppler-utils). \
                For text files, just use the path directly."
            ))
        }
    }
}

// ─── Model Calls ─────────────────────────────────────────────────────────────

const BASE_SYSTEM: &str = "You are a helpful AI assistant. \
When given web search results in [Web search results] blocks, use them to answer accurately. \
For file contents in code blocks, analyze them as requested. \
Be concise and direct. Plain text only, no markdown symbols.";

async fn call_model(
    messages: &[ChatMessage],
    model_path: &std::path::Path,
    ollama_model: &Option<String>,
    temperature: f32,
    max_tokens: u32,
    system: &Option<String>,
) -> Result<String> {
    let system_prompt = match system {
        Some(s) => format!("{}\n\n{}", BASE_SYSTEM, s),
        None => BASE_SYSTEM.to_string(),
    };

    if let Some(model) = ollama_model {
        call_ollama(model, &system_prompt, messages, temperature, max_tokens).await
    } else {
        call_local(messages, model_path, temperature, max_tokens, &system_prompt).await
    }
}

async fn call_ollama(
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    temperature: f32,
    max_tokens: u32,
) -> Result<String> {
    use futures_util::StreamExt;

    let mut all_messages = vec![serde_json::json!({"role": "system", "content": system})];
    for m in messages {
        all_messages.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    let body = serde_json::json!({
        "model": model,
        "messages": all_messages,
        "stream": true,
        "options": { "temperature": temperature, "num_predict": max_tokens }
    });

    let resp = reqwest::Client::new()
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("Inference engine unreachable: {}", e))?;

    let mut full = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Ok(text) = std::str::from_utf8(&chunk) {
            for line in text.lines() {
                if line.trim().is_empty() { continue; }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(token) = val["message"]["content"].as_str() {
                        print!("{}", token);
                        io::stdout().flush()?;
                        full.push_str(token);
                    }
                    if val["done"].as_bool().unwrap_or(false) { break; }
                }
            }
        }
    }

    if full.is_empty() {
        return Err(anyhow!("Empty response from inference engine"));
    }
    Ok(full)
}

async fn call_local(
    messages: &[ChatMessage],
    model_path: &std::path::Path,
    temperature: f32,
    max_tokens: u32,
    system: &str,
) -> Result<String> {
    let req = InferenceRequest {
        model_path: model_path.to_path_buf(),
        messages: messages.to_vec(),
        temperature,
        max_tokens,
        system_prompt: Some(system.to_string()),
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let inference = tokio::spawn(run_streaming(req, tx));

    let mut full = String::new();
    while let Some(token) = rx.recv().await {
        print!("{}", token);
        io::stdout().flush()?;
        full.push_str(&token);
    }
    inference.await??;
    Ok(full)
}

// ─── Web Search ───────────────────────────────────────────────────────────────

async fn web_search(query: &str) -> Option<String> {
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::builder()
        .user_agent("mimona/0.1")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(data) = resp.json::<serde_json::Value>().await {
            let mut results = Vec::new();

            if let Some(t) = data["Abstract"].as_str() {
                if !t.is_empty() { results.push(t.to_string()); }
            }
            if let Some(t) = data["Answer"].as_str() {
                if !t.is_empty() { results.push(format!("Answer: {}", t)); }
            }
            if let Some(topics) = data["RelatedTopics"].as_array() {
                for topic in topics.iter().take(4) {
                    if let Some(text) = topic["Text"].as_str() {
                        if !text.is_empty() { results.push(text.to_string()); }
                    }
                }
            }

            if !results.is_empty() {
                return Some(results.join("\n\n"));
            }
        }
    }

    // Fallback: DuckDuckGo HTML scrape
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    let html = client.get(&url).send().await.ok()?.text().await.ok()?;

    let mut snippets = Vec::new();
    for part in html.split("result__snippet") {
        if let Some(start) = part.find('>') {
            let snippet = &part[start + 1..];
            if let Some(end) = snippet.find('<') {
                let text = snippet[..end].trim().to_string();
                // Decode basic HTML entities
                let text = text
                    .replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&#x27;", "'")
                    .replace("&quot;", "\"");
                if text.len() > 40 {
                    snippets.push(text);
                }
            }
        }
        if snippets.len() >= 5 { break; }
    }

    if snippets.is_empty() { return None; }
    Some(snippets.join("\n\n"))
}

async fn get_ollama_model() -> Option<String> {
    let resp = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .ok()?;

    let data: serde_json::Value = resp.json().await.ok()?;
    data["models"]
        .as_array()?
        .first()?
        ["name"].as_str()
        .map(|s| s.to_string())
}