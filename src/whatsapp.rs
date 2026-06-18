use anyhow::{anyhow, Result};
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::fs;
use tokio::sync::RwLock;

use crate::config::whatsapp_users_path;

/// How a given WhatsApp user's bridge is connected.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMethod {
    /// whatsapp-web.js / Baileys style — QR scan, runs against the user's own number.
    Baileys,
    /// Meta Cloud API / Business Solution Provider — requires business verification.
    Official,
}

/// Where a link currently sits in its lifecycle.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkStatus {
    /// Waiting for the user to scan the QR code (Baileys only).
    PendingQrScan,
    /// Connected and actively relaying messages.
    Connected,
    /// Was connected, bridge lost the session (needs relink).
    Disconnected,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhatsAppUser {
    /// E.164 phone number, used as the unique key (e.g. "+15551234567").
    pub phone_number: String,
    pub connection_method: ConnectionMethod,
    pub status: LinkStatus,
    /// The system prompt that defines this user's assistant persona.
    pub system_prompt: String,
    /// Which Mimona model this assistant uses for replies.
    pub model: String,
    pub linked_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_system_prompt() -> String {
    "You are a helpful WhatsApp assistant. Keep replies concise and friendly.".to_string()
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WhatsAppStore {
    /// Keyed by phone_number.
    users: HashMap<String, WhatsAppUser>,
}

/// Process-wide cache so concurrent requests don't race on the JSON file.
/// The file on disk remains the source of truth; this just serializes access.
static STORE_LOCK: RwLock<()> = RwLock::const_new(());

async fn load_store() -> Result<WhatsAppStore> {
    let path = whatsapp_users_path();
    if !path.exists() {
        return Ok(WhatsAppStore::default());
    }
    let raw = fs::read_to_string(&path).await?;
    if raw.trim().is_empty() {
        return Ok(WhatsAppStore::default());
    }
    let store: WhatsAppStore = serde_json::from_str(&raw)?;
    Ok(store)
}

async fn save_store(store: &WhatsAppStore) -> Result<()> {
    crate::config::ensure_dirs().await?;
    let raw = serde_json::to_string_pretty(store)?;
    fs::write(whatsapp_users_path(), raw).await?;
    Ok(())
}

// ─── Public helpers (used by route handlers and, later, the bridge) ──────────

pub async fn get_user(phone_number: &str) -> Result<Option<WhatsAppUser>> {
    let _guard = STORE_LOCK.read().await;
    let store = load_store().await?;
    Ok(store.users.get(phone_number).cloned())
}

pub async fn list_users() -> Result<Vec<WhatsAppUser>> {
    let _guard = STORE_LOCK.read().await;
    let store = load_store().await?;
    Ok(store.users.into_values().collect())
}

pub async fn upsert_user(
    phone_number: &str,
    connection_method: ConnectionMethod,
    model: Option<String>,
) -> Result<WhatsAppUser> {
    let _guard = STORE_LOCK.write().await;
    let mut store = load_store().await?;

    let now = Utc::now();
    let user = store
        .users
        .entry(phone_number.to_string())
        .and_modify(|u| {
            u.connection_method = connection_method.clone();
            u.updated_at = now;
        })
        .or_insert_with(|| WhatsAppUser {
            phone_number: phone_number.to_string(),
            connection_method,
            status: LinkStatus::PendingQrScan,
            system_prompt: default_system_prompt(),
            model: model.unwrap_or_else(|| "tinyllama:1b".to_string()),
            linked_at: now,
            updated_at: now,
        })
        .clone();

    save_store(&store).await?;
    Ok(user)
}

pub async fn set_status(phone_number: &str, status: LinkStatus) -> Result<()> {
    let _guard = STORE_LOCK.write().await;
    let mut store = load_store().await?;
    let user = store
        .users
        .get_mut(phone_number)
        .ok_or_else(|| anyhow!("No linked WhatsApp user for {}", phone_number))?;
    user.status = status;
    user.updated_at = Utc::now();
    save_store(&store).await?;
    Ok(())
}

pub async fn set_prompt(phone_number: &str, prompt: &str) -> Result<WhatsAppUser> {
    let _guard = STORE_LOCK.write().await;
    let mut store = load_store().await?;
    let user = store
        .users
        .get_mut(phone_number)
        .ok_or_else(|| anyhow!("No linked WhatsApp user for {}", phone_number))?;
    user.system_prompt = prompt.to_string();
    user.updated_at = Utc::now();
    let updated = user.clone();
    save_store(&store).await?;
    Ok(updated)
}

pub async fn set_model(phone_number: &str, model: &str) -> Result<WhatsAppUser> {
    let _guard = STORE_LOCK.write().await;
    let mut store = load_store().await?;
    let user = store
        .users
        .get_mut(phone_number)
        .ok_or_else(|| anyhow!("No linked WhatsApp user for {}", phone_number))?;
    user.model = model.to_string();
    user.updated_at = Utc::now();
    let updated = user.clone();
    save_store(&store).await?;
    Ok(updated)
}

pub async fn unlink_user(phone_number: &str) -> Result<()> {
    let _guard = STORE_LOCK.write().await;
    let mut store = load_store().await?;
    store.users.remove(phone_number);
    save_store(&store).await?;
    Ok(())
}

// ─── HTTP route handlers (mounted under /api/whatsapp) ───────────────────────

#[derive(Debug, Deserialize)]
pub struct LinkRequest {
    pub phone_number: String,
    pub connection_method: ConnectionMethod,
    #[serde(default)]
    pub model: Option<String>,
}

/// POST /api/whatsapp/link
/// Called by the link UI when a user picks Baileys or Official and submits
/// their number. Creates (or updates) the user record; the bridge service
/// picks this up on its own polling/connect cycle and starts the QR flow
/// or the Cloud API handshake.
pub async fn link(Json(body): Json<LinkRequest>) -> Response {
    if body.phone_number.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "phone_number is required" })),
        )
            .into_response();
    }

    match upsert_user(&body.phone_number, body.connection_method, body.model).await {
        Ok(user) => Json(json!({ "user": user })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/whatsapp/users/:phone
pub async fn get_user_route(Path(phone): Path<String>) -> Response {
    match get_user(&phone).await {
        Ok(Some(user)) => Json(json!({ "user": user })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No linked WhatsApp user for this number" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/whatsapp/users
pub async fn list_users_route() -> Response {
    match list_users().await {
        Ok(users) => Json(json!({ "users": users })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub system_prompt: String,
}

/// PUT /api/whatsapp/users/:phone/prompt
/// This is what lets a user change their assistant's behavior at any time,
/// even long after the initial link — the bridge reads system_prompt fresh
/// on every incoming message, so there's no relink needed.
pub async fn set_prompt_route(
    Path(phone): Path<String>,
    Json(body): Json<PromptRequest>,
) -> Response {
    if body.system_prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "system_prompt cannot be empty" })),
        )
            .into_response();
    }

    match set_prompt(&phone, &body.system_prompt).await {
        Ok(user) => Json(json!({ "user": user })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ModelRequest {
    pub model: String,
}

/// PUT /api/whatsapp/users/:phone/model
pub async fn set_model_route(
    Path(phone): Path<String>,
    Json(body): Json<ModelRequest>,
) -> Response {
    match set_model(&phone, &body.model).await {
        Ok(user) => Json(json!({ "user": user })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct StatusRequest {
    pub status: LinkStatus,
}

/// PUT /api/whatsapp/users/:phone/status
/// Called by the bridge service to report connection lifecycle changes
/// (e.g. QR scanned → Connected, session dropped → Disconnected).
pub async fn set_status_route(
    Path(phone): Path<String>,
    Json(body): Json<StatusRequest>,
) -> Response {
    match set_status(&phone, body.status).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// DELETE /api/whatsapp/users/:phone
pub async fn unlink_route(Path(phone): Path<String>) -> Response {
    match unlink_user(&phone).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}