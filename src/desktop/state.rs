use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// A single chat message displayed in the chat panel.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,   // "user" or "assistant"
    pub content: String,
    pub pending: bool,  // true while still streaming tokens in
}


#[derive(Clone, Debug, PartialEq, Default)]
pub enum OllamaStatus {
    #[default]
    Checking,
    Running,
    NotInstalled,
    NotRunning,
}

/// Everything a panel needs to read or mutate, wrapped in Arc<Mutex<>>
/// so both the egui main thread and the Tokio background tasks can
/// access it safely.
#[derive(Default)]
pub struct AppState {
      // ── Setup ────────────────────────────────────────────────────────────
    pub ollama_status: OllamaStatus,
    pub setup_dismissed: bool,
    // Guards against re-sending CheckOllama on every single egui frame while
    // a check is already in flight (this used to spawn a new subprocess
    // check ~60x/second, which is what caused the flood of ollama.exe
    // windows/crash dialogs). Cleared once the WorkerUpdate::OllamaStatus
    // result comes back.
    pub ollama_check_in_flight: bool,

    // ── Chat ────────────────────────────────────────────────────────────
    pub chat_history: Vec<ChatMessage>,
    pub chat_input: String,
    pub chat_model: String,
    pub chat_thinking: bool,

    // ── Models ──────────────────────────────────────────────────────────
    pub local_models: Vec<LocalModel>,
    pub models_loading: bool,
    pub pull_model_input: String,
    pub pull_progress: Option<PullProgress>,

    // ── WhatsApp ─────────────────────────────────────────────────────────
    pub wa_users: Vec<WaUser>,
    pub wa_qr: Option<String>,          // base64 PNG data URL
    pub wa_session_id: Option<String>,
    pub wa_session_state: String,       // "idle" | "connecting" | "awaiting_qr_scan" | "connected"
    pub wa_selected_phone: Option<String>,
    pub wa_prompt_input: String,
    pub wa_prompt_saved: bool,

    // ── Automate ───────────────────────────────────────────────────────────
    pub auto_tab: AutomateTab,
    // ── Wallet ───────────────────────────────────────────────────────────
    pub auto_social_brand: String,
    pub auto_social_topic: String,
    pub auto_social_platforms: String,
    pub auto_social_result: String,
    pub auto_social_loading: bool,
    // Cold Email
    pub auto_email_product: String,
    pub auto_email_audience: String,
    pub auto_email_count: String,
    pub auto_email_result: String,
    pub auto_email_loading: bool,
    // Local SEO
    pub auto_seo_business: String,
    pub auto_seo_location: String,
    pub auto_seo_keywords: String,
    pub auto_seo_result: String,
    pub auto_seo_loading: bool,
    pub wallet_address: Option<String>,
    pub wallet_balance: Option<f64>,
    pub wallet_loading: bool,

    // ── Animation ────────────────────────────────────────────────────────
    pub anim_prompt: String,
    pub anim_status: AnimationStatus,
    pub anim_generated_code: String,
    pub anim_show_code: bool,
    pub anim_manim_installed: Option<bool>,

    // ── Website ───────────────────────────────────────────────────────────
    pub web_brand: String,
    pub web_description: String,
    pub web_services: String,
    pub web_contact: String,
    pub web_site_type: String,
    pub web_color: String,
    pub web_status: WebsiteStatus,
    pub web_public_url: Option<String>,
    pub web_local_port: u16,
    pub web_generated_html: String,
    pub web_show_code: bool,

    // ── Widget / Embed ───────────────────────────────────────────────────
    pub widget_bot_name: String,
    pub widget_welcome: String,
    pub widget_system_prompt: String,
    pub widget_color: String,
    pub widget_saved: bool,

    // ── Global ───────────────────────────────────────────────────────────
    pub status_message: Option<String>,
    pub server_port: u16,
}

#[derive(Clone, Debug)]
pub struct LocalModel {
    pub name: String,
    pub tag: String,
    pub size_gb: f32,
}

impl LocalModel {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}

#[derive(Clone, Debug)]
pub struct PullProgress {
    pub model: String,
    pub downloaded_gb: f32,
    pub total_gb: f32,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub struct WaUser {
    pub phone_number: String,
    pub status: String,
    pub system_prompt: String,
    pub model: String,
}

/// Commands the UI sends to the async Tokio worker.
pub enum UiCommand {
       // Setup
    CheckOllama,
    InstallOllama,
    StartOllama,
    DismissSetup,
    // Chat
    SendMessage { model: String, messages: Vec<(String, String)>, system: String },

    // Models
    RefreshModels,
    PullModel(String),
    CancelPull,
    DeleteModel(String),

    // WhatsApp
    StartWaSession,
    PollWaStatus(String),  // session_id
    RefreshWaUsers,
    SaveWaPrompt { phone: String, prompt: String },
    SetWaModel { phone: String, model: String },
    UnlinkWa(String),

       // Automate
    GenerateSocialContent { brand: String, topic: String, platforms: String, model: String },
    GenerateColdEmails { product: String, audience: String, count: u32, model: String },
    GenerateSeoContent { business: String, location: String, keywords: String, model: String },
 

    // Animation
    GenerateAnimation(String),
    CheckManimInstalled,
    OpenVideo(String),

    // Wallet
    RefreshWallet,
    CreateWallet,

    // Website
    GenerateWebsite {
        brand: String,
        description: String,
        services: String,
        contact: String,
        site_type: String,
        color: String,
    },
    DeployWebsite,
    StopWebsite,
    OpenBrowser(String),

    // Widget
    SaveWidgetSettings {
        bot_name: String,
        welcome: String,
        system_prompt: String,
        color: String,
    },

    // Server
    StartServer,
}

/// Updates the async worker sends back to the UI.
pub enum WorkerUpdate {
    OllamaStatus(OllamaStatus),

    // Chat
    ChatToken(String),
    ChatDone,
    ChatError(String),

    // Models
    ModelsLoaded(Vec<LocalModel>),
    PullProgress(PullProgress),
    PullDone(String),
    PullError(String),
    PullCancelled,
    ModelDeleted(String),

    // WhatsApp
    WaUsers(Vec<WaUser>),
    WaQr(String),
    WaConnected(String),    // resolved phone number
    WaDisconnected,
    WaSessionId(String),
    WaPromptSaved,
    WaError(String),
    // Automate
    AutomateDone { tool: String, result: String },
    AutomateError { tool: String, error: String },

    // Animation
    AnimCodeGenerated(String),
    AnimRendering,
    AnimDone(String),
    AnimError(String),
    ManimInstalled(bool),

    // Wallet
    WalletInfo { address: String, balance: f64 },
    WalletCreated(String),
    WalletError(String),

    // Website
    WebsiteGenerated(String),   // generated HTML
    WebsiteDeployed { local_port: u16, public_url: String },
    WebsiteStopped,
    WebsiteError(String),

    // Widget
    WidgetSettingsLoaded {
        bot_name: String,
        welcome: String,
        system_prompt: String,
        color: String,
    },

    // Server
    ServerStarted(u16),
    StatusMessage(String),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum AutomateTab {
    #[default]
    Social,
    Email,
    Seo,
}
 

pub type SharedState = Arc<Mutex<AppState>>;
pub type CmdSender = mpsc::UnboundedSender<UiCommand>;
pub type UpdateSender = mpsc::UnboundedSender<WorkerUpdate>;
pub type UpdateReceiver = mpsc::UnboundedReceiver<WorkerUpdate>;


// ── Animation state (added to AppState below via manual fields) ──────────────

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationStatus {
    Idle,
    GeneratingCode,
    Rendering,
    Done(String),  // path to output video
    Error(String),
}

impl Default for AnimationStatus {
    fn default() -> Self { AnimationStatus::Idle }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum WebsiteStatus {
    #[default]
    Idle,
    Generating,
    Generated,
    Deploying,
    Live,
    Error(String),
}