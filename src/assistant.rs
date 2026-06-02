use crate::models::storage::load_local_models;
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

const SYSTEM_PROMPT: &str = r#"You are Mona, the built-in assistant for Mimona — a local AI runtime with blockchain payments.

You help users manage local AI models. You are smart, friendly, and concise. No markdown, plain text only.

Available commands users can type:
- list                 → see downloaded models
- pull <model>         → download a model  
- run <model>          → chat with a model
- serve                → start API server on port 11435
- node start           → become a node provider, earn SOL
- wallet create        → create Solana wallet
- wallet balance       → check SOL balance

Available models: qwen2.5-coder:7b (0.7GB), qwen2.5-coder:7b (4.7GB), llama3:8b (4.9GB), mistral:7b (4.4GB), phi3:mini (2.2GB), deepseek-coder:6.7b (4.1GB)

When someone says "pull qwen" resolve it to qwen2.5-coder:7b.
Keep responses short and helpful. Plain text only, no bullet symbols or markdown."#;

fn resolve_model_name(input: &str) -> Option<&'static str> {
    let s = input.to_lowercase();
    let s = s.trim();
    if s.contains("qwen")                          { return Some("qwen2.5-coder:7b"); }
    if s.contains("llama3") || s == "llama"        { return Some("llama3:8b"); }
    if s.contains("tiny")                          { return Some("qwen2.5-coder:7b"); }
    if s.contains("mistral")                       { return Some("mistral:7b"); }
    if s.contains("phi")                           { return Some("phi3:mini"); }
    if s.contains("deepseek") || s.contains("deep"){ return Some("deepseek-coder:6.7b"); }
    if s.contains("small") || s.contains("smallest"){ return Some("qwen2.5-coder:7b"); }
    None
}

fn interpret_as_command(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let lower = lower.trim();

    if matches!(lower, "list" | "list models" | "ls" | "models") {
        return Some("mimona list".into());
    }
    if matches!(lower, "serve" | "start server" | "server" | "api" | "start api") {
        return Some("mimona serve".into());
    }
    if matches!(lower, "wallet" | "balance" | "wallet balance") {
        return Some("mimona wallet balance".into());
    }
    if matches!(lower, "create wallet" | "wallet create" | "new wallet") {
        return Some("mimona wallet create".into());
    }
    if matches!(lower, "node" | "node start" | "start node" | "earn" | "earn sol") {
        return Some("mimona node start".into());
    }
    if matches!(lower, "earnings" | "node earnings") {
        return Some("mimona node earnings".into());
    }

    // pull / download
    if lower.starts_with("pull") || lower.starts_with("download") || lower.starts_with("get") {
        let after = lower
            .trim_start_matches("pull")
            .trim_start_matches("download")
            .trim_start_matches("get")
            .trim()
            .to_string();
        if after.is_empty() {
            return Some("mimona pull qwen2.5-coder:7b".into());
        }
        if after.contains(':') {
            return Some(format!("mimona pull {}", after));
        }
        let resolved = resolve_model_name(&after).unwrap_or("qwen2.5-coder:7b");
        return Some(format!("mimona pull {}", resolved));
    }

    // run / chat
    if lower.starts_with("run") || lower.starts_with("chat with") || lower.starts_with("talk to") {
        let after = lower
            .trim_start_matches("run")
            .trim_start_matches("chat with")
            .trim_start_matches("talk to")
            .trim()
            .to_string();
        if after.is_empty() {
            return Some("mimona list".into());
        }
        let model = if after.contains(':') {
            after.clone()
        } else {
            resolve_model_name(&after).unwrap_or("qwen2.5-coder:7b").to_string()
        };
        return Some(format!("mimona run {}", model));
    }

    None
}

async fn execute_command(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let binary = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("./target/release/mimona"));
    match tokio::process::Command::new(&binary).args(&parts[1..]).status().await {
        Ok(_) => {}
        Err(e) => println!("  Could not run: {} ({})", cmd.yellow(), e),
    }
}

async fn check_ollama() -> Option<String> {
    let resp = reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .ok()?;

    let data: serde_json::Value = resp.json().await.ok()?;

    // Return first model name if available, otherwise a placeholder
    // so we know the server is UP even if no models downloaded yet
    let model = data["models"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["name"].as_str())
        .unwrap_or("__server_running__")
        .to_string();

    Some(model)
}

/// Web search via DuckDuckGo instant answer API (no key needed)
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

    let resp = client.get(&url).send().await.ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;

    let mut results = Vec::new();

    // Abstract (main answer)
    if let Some(abstract_text) = data["Abstract"].as_str() {
        if !abstract_text.is_empty() {
            results.push(format!("{}", abstract_text));
        }
    }

    // Answer (short direct answer e.g. prices, facts)
    if let Some(answer) = data["Answer"].as_str() {
        if !answer.is_empty() {
            results.push(format!("Answer: {}", answer));
        }
    }

    // Related topics
    if let Some(topics) = data["RelatedTopics"].as_array() {
        for topic in topics.iter().take(3) {
            if let Some(text) = topic["Text"].as_str() {
                if !text.is_empty() {
                    results.push(text.to_string());
                }
            }
        }
    }

    if results.is_empty() {
        // Fallback: scrape DuckDuckGo HTML search for snippet
        return web_search_html(query).await;
    }

    Some(results.join("\n"))
}

/// Fallback: fetch a plain text snippet from DuckDuckGo HTML
async fn web_search_html(query: &str) -> Option<String> {
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; mimona/0.1)")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    let html = client.get(&url).send().await.ok()?.text().await.ok()?;

    // Very simple extraction — grab first result snippet
    let mut snippets = Vec::new();
    for part in html.split("result__snippet") {
        if let Some(start) = part.find('>') {
            let snippet = &part[start + 1..];
            if let Some(end) = snippet.find('<') {
                let text = snippet[..end].trim().to_string();
                if text.len() > 30 {
                    snippets.push(text);
                }
            }
        }
        if snippets.len() >= 3 { break; }
    }

    if snippets.is_empty() { return None; }
    Some(snippets.join(" | "))
}

/// Detect if user is asking something that needs a web search
fn needs_web_search(input: &str) -> Option<String> {
    let lower = input.to_lowercase();

    // Time-sensitive keywords
    let search_triggers = [
        "price", "today", "current", "news", "latest", "now",
        "weather", "stock", "who is", "what is the", "when did",
        "how much", "bitcoin", "crypto", "score", "result",
        "released", "announcement", "update", "2024", "2025", "2026",
    ];

    for trigger in &search_triggers {
        if lower.contains(trigger) {
            return Some(input.to_string());
        }
    }

    // Question words on factual topics
    if (lower.starts_with("what") || lower.starts_with("who") || lower.starts_with("when"))
        && lower.ends_with('?')
    {
        return Some(input.to_string());
    }

    None
}

async fn call_ollama(model: &str, system: &str, messages: &[serde_json::Value]) -> Result<String> {
    use futures_util::StreamExt;

    let mut all_messages = vec![serde_json::json!({"role": "system", "content": system})];
    all_messages.extend_from_slice(messages);

    let body = serde_json::json!({
        "model": model,
        "messages": all_messages,
        "stream": true,
        "options": { "temperature": 0.7, "num_predict": 512 }
    });

    let resp = reqwest::Client::new()
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await?;

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
                    // Check for done
                    if val["done"].as_bool().unwrap_or(false) {
                        break;
                    }
                }
            }
        }
    }

    if full.is_empty() {
        return Err(anyhow::anyhow!("Empty response from Ollama"));
    }

    Ok(full)
}

pub async fn run_assistant() -> Result<()> {
    print_welcome();

    let local_models = load_local_models().await.unwrap_or_default();
    let ollama_model = check_ollama().await;

    match &ollama_model {
        Some(ref model) if model == "__server_running__" => {
            println!(
                "  {} Mimona server is running but no models downloaded yet.",
                "Mona:".cyan().bold(),
            );
            println!("       Type {} to get started.\n", "'pull tinyllama'".yellow());
        }
        Some(model) => {
            println!(
                "  {} Mimona server running with {}. I'm fully powered up!",
                "Mona:".cyan().bold(),
                model.yellow()
            );
            println!("       Ask me anything, or type {} to see options.\n", "'help'".yellow());
        }
        None if local_models.is_empty() => {
            println!("  {} No models found yet.", "Mona:".cyan().bold());
            println!("       Type {} to download a small model to get started.\n", "'pull tinyllama'".yellow());
        }
        None => {
            let names: Vec<String> = local_models.iter().map(|m| m.full_name()).collect();
            println!("  {} Found: {}", "Mona:".cyan().bold(), names.join(", ").yellow());
            println!("       What would you like to do?\n");
        }
    }

    let mut history: Vec<serde_json::Value> = vec![];

    loop {
        print!("  {} ", "You:".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let input = input.trim().to_string();
        if input.is_empty() { continue; }

        if matches!(input.to_lowercase().as_str(), "exit" | "quit" | "bye" | "q") {
            println!("\n  {} Goodbye!\n", "Mona:".cyan().bold());
            break;
        }

        // Direct command shortcut
        if let Some(cmd) = interpret_as_command(&input) {
            println!();
            println!("  {} Running: {}", "Mona:".cyan().bold(), cmd.yellow());
            println!();
            execute_command(&cmd).await;
            println!();
            history.push(serde_json::json!({"role": "user", "content": input}));
            history.push(serde_json::json!({"role": "assistant", "content": format!("Ran: {}", cmd)}));
            continue;
        }

        history.push(serde_json::json!({"role": "user", "content": input.clone()}));

        print!("\n  {} ", "Mona:".cyan().bold());
        io::stdout().flush()?;

        // Check if we need to search the web first
        let system = if let Some(search_query) = needs_web_search(&input) {
            print!("{}", "(searching web...) ".dimmed());
            io::stdout().flush()?;

            match web_search(&search_query).await {
                Some(results) => {
                    format!(
                        "{}\n\nWeb search results for '{}':\n{}\n\nUse these results to answer the user. Be concise.",
                        SYSTEM_PROMPT, search_query, results
                    )
                }
                None => {
                    print!("{}", "(search failed, answering from memory) ".dimmed());
                    SYSTEM_PROMPT.to_string()
                }
            }
        } else {
            SYSTEM_PROMPT.to_string()
        };

        // Call Ollama with full context
        if let Some(ref model) = ollama_model {
            match call_ollama(model, &system, &history).await {
                Ok(response) => {
                    println!("\n");
                    history.push(serde_json::json!({"role": "assistant", "content": response}));
                }
                Err(e) => {
                    println!("{}\n", rule_based_response(&input));
                }
            }
        } else {
            println!("{}\n", rule_based_response(&input));
        }
    }

    Ok(())
}

fn rule_based_response(input: &str) -> String {
    let lower = input.to_lowercase();
    if lower.contains("pull") || lower.contains("download") {
        "Try: pull qwen  or  pull tinyllama".into()
    } else if lower.contains("run") || lower.contains("chat") {
        "First pull a model, then type: run <model-name>".into()
    } else if lower.contains("list") || lower.contains("model") {
        "Type 'list' to see downloaded models.".into()
    } else if lower.contains("serve") || lower.contains("api") {
        "Type 'serve' to start the API server on port 11435.".into()
    } else if lower.contains("earn") || lower.contains("node") {
        "Type 'node start' to earn SOL as a node provider.".into()
    } else if lower.contains("help") {
        "Commands: list, pull <model>, run <model>, serve, node start, wallet create".into()
    } else {
        "I need a model to answer that. Type: pull tinyllama".into()
    }
}

fn print_welcome() {
    println!(
        "{}",
        r#"
  ███╗   ███╗██╗███╗   ███╗ ██████╗ ███╗   ██╗ █████╗
  ████╗ ████║██║████╗ ████║██╔═══██╗████╗  ██║██╔══██╗
  ██╔████╔██║██║██╔████╔██║██║   ██║██╔██╗ ██║███████║
  ██║╚██╔╝██║██║██║╚██╔╝██║██║   ██║██║╚██╗██║██╔══██║
  ██║ ╚═╝ ██║██║██║ ╚═╝ ██║╚██████╔╝██║ ╚████║██║  ██║
  ╚═╝     ╚═╝╚═╝╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝  ╚═╝
"#.cyan().bold()
    );
    println!("  {} — Your local AI assistant with web search\n", "Mona".cyan().bold());
    println!("  {}", "─".repeat(55).dimmed());
    println!(
        "  Type {}, {}, {} or ask anything — I can search the web too.",
        "'list'".yellow(), "'pull qwen'".yellow(), "'serve'".yellow()
    );
    println!("  Type {} to exit.\n", "'exit'".dimmed());
    println!("  {}", "─".repeat(55).dimmed());
    println!();
}
