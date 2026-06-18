use crate::server::routes::{
    chat_completions, health, list_models_openai, list_tags, ollama_generate, web_search, AppState,
};
use crate::whatsapp::{
    get_user_route, link, list_users_route, set_model_route, set_prompt_route, set_status_route,
    unlink_route,
};
use anyhow::Result;
use axum::{
    routing::{delete, get, post, put},
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
        // WhatsApp link + config routes
        .route("/api/whatsapp/link", post(link))
        .route("/api/whatsapp/users", get(list_users_route))
        .route("/api/whatsapp/users/:phone", get(get_user_route))
        .route("/api/whatsapp/users/:phone", delete(unlink_route))
        .route("/api/whatsapp/users/:phone/prompt", put(set_prompt_route))
        .route("/api/whatsapp/users/:phone/model", put(set_model_route))
        .route("/api/whatsapp/users/:phone/status", put(set_status_route))
        // Serve frontend/index.html at / (and any static assets, incl. whatsapp.html)
        .fallback_service(ServeDir::new(&frontend_dir).append_index_html_on_directories(true))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!();
    println!("  {} Mimona Server", "✓".green().bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} http://{}", "Web UI:".dimmed(), addr.cyan().bold());
    println!("  {} http://{}/whatsapp.html", "WhatsApp setup:".dimmed(), addr.cyan().bold());
    println!("  {} http://{}/v1/chat/completions", "Chat API:".dimmed(), addr);
    println!("  {} http://{}/api/generate", "Generate:".dimmed(), addr);
    println!("  {} http://{}/v1/models", "Models:".dimmed(), addr);
    println!();

    crate::whatsapp_bridge_launcher::ensure_bridge_running().await;
    println!();

    axum::serve(listener, app).await?;
    Ok(())
}