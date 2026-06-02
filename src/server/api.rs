use crate::server::routes::{
    chat_completions, health, list_models_openai, list_tags, ollama_generate, web_search, AppState,
};
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use colored::*;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub async fn start(host: String, port: u16) -> Result<()> {
    let state = AppState {
        host: host.clone(),
        port,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Resolve frontend/ dir relative to the binary, fallback to cwd for dev
    let frontend_dir = {
        let exe = std::env::current_exe().unwrap_or_default();
        let candidate = exe.parent().unwrap_or(std::path::Path::new("."))
            .join("frontend");
        if candidate.exists() { candidate } else { std::path::PathBuf::from("frontend") }
    };

    let app = Router::new()
        // API routes
        .route("/health", get(health))
        .route("/search", get(web_search))
        .route("/api/tags", get(list_tags))
        .route("/api/generate", post(ollama_generate))
        .route("/v1/models", get(list_models_openai))
        .route("/v1/chat/completions", post(chat_completions))
        // Serve frontend/index.html at / (and any static assets)
        .fallback_service(ServeDir::new(&frontend_dir).append_index_html_on_directories(true))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!();
    println!("  {} Mimona Server", "✓".green().bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} http://{}", "Web UI:".dimmed(), addr.cyan().bold());
    println!("  {} http://{}/v1/chat/completions", "Chat API:".dimmed(), addr);
    println!("  {} http://{}/api/generate", "Generate:".dimmed(), addr);
    println!("  {} http://{}/v1/models", "Models:".dimmed(), addr);
    println!();

    axum::serve(listener, app).await?;
    Ok(())
}