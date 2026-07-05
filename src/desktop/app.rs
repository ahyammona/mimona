use eframe::egui::{self, *};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::state::*;
use super::worker;
use super::{
    panel_animation, panel_chat, 
    panel_models, panel_whatsapp,
    panel_automate, panel_website,
    panel_widget, panel_setup
};

#[derive(PartialEq, Clone, Copy)]
pub enum Panel {
    Chat,
    WhatsApp,
    Models,
    Animation,
    Automate,
    Website,
    Widget,
}

pub struct MimonaApp {
    pub state: Arc<Mutex<AppState>>,
    pub cmd_tx: CmdSender,
    pub update_rx: Arc<Mutex<mpsc::UnboundedReceiver<WorkerUpdate>>>,
    pub active_panel: Panel,
    pub wa_poll_timer: f64,
}

impl MimonaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        setup_visuals(&cc.egui_ctx);

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<UiCommand>();
        let (update_tx, update_rx) = mpsc::unbounded_channel::<WorkerUpdate>();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(worker::run_worker(cmd_rx, update_tx));
        });

        let state = Arc::new(Mutex::new(AppState {
            chat_model: String::new(),
            wa_session_state: "idle".to_string(),
            server_port: 11435,
            ..Default::default()
        }));

        Self {
            state,
            cmd_tx,
            update_rx: Arc::new(Mutex::new(update_rx)),
            active_panel: Panel::Chat,
            wa_poll_timer: 0.0,
        }
    }

    fn process_updates(&mut self) {
        let mut rx = self.update_rx.lock().unwrap();
        let mut st = self.state.lock().unwrap();

        while let Ok(update) = rx.try_recv() {
            match update {
                WorkerUpdate::ChatToken(token) => {
                    if let Some(last) = st.chat_history.last_mut() {
                        if last.pending {
                            last.content.push_str(&token);
                        }
                    }
                }
                WorkerUpdate::ChatDone => {
                    if let Some(last) = st.chat_history.last_mut() {
                        last.pending = false;
                    }
                    st.chat_thinking = false;
                }
                WorkerUpdate::ChatError(e) => {
                    if let Some(last) = st.chat_history.last_mut() {
                        last.content = format!("Error: {}", e);
                        last.pending = false;
                    }
                    st.chat_thinking = false;
                }
                WorkerUpdate::ModelsLoaded(models) => {
                    if st.chat_model.is_empty() {
                        if let Some(first) = models.first() {
                            st.chat_model = first.full_name();
                        }
                    }
                    st.local_models = models;
                    st.models_loading = false;
                }
                WorkerUpdate::PullProgress(p) => {
                    st.pull_progress = Some(p);
                }
                WorkerUpdate::PullDone(_) => {
                    st.pull_progress = None;
                }
                WorkerUpdate::PullError(e) => {
                    st.pull_progress = None;
                    st.status_message = Some(format!("Pull failed: {}", e));
                }
                WorkerUpdate::PullCancelled => {
                    st.pull_progress = None;
                    st.status_message = Some("Download cancelled".to_string());
                }
                WorkerUpdate::ModelDeleted(_) => {
                    st.status_message = Some("Model deleted".to_string());
                }
                WorkerUpdate::WaSessionId(sid) => {
                    st.wa_session_id = Some(sid);
                    st.wa_session_state = "awaiting_qr_scan".to_string();
                }
                WorkerUpdate::WaQr(qr) => {
                    st.wa_qr = Some(qr);
                    st.wa_session_state = "qr_ready".to_string();
                }
                WorkerUpdate::WaConnected(phone) => {
                    st.wa_session_state = "connected".to_string();
                    st.wa_selected_phone = Some(phone);
                    st.wa_qr = None;
                }
                WorkerUpdate::WaDisconnected => {
                    st.wa_session_state = "idle".to_string();
                }
                WorkerUpdate::WaUsers(users) => {
                    st.wa_users = users;
                }
                WorkerUpdate::WaPromptSaved => {
                    st.wa_prompt_saved = true;
                }
                WorkerUpdate::WaError(e) => {
                    st.status_message = Some(format!("WhatsApp: {}", e));
                    if st.wa_session_state == "connecting" {
                        st.wa_session_state = "idle".to_string();
                    }
                }
                WorkerUpdate::WalletInfo { address, balance } => {
                    st.wallet_address = Some(address);
                    st.wallet_balance = Some(balance);
                    st.wallet_loading = false;
                }
                WorkerUpdate::WalletCreated(address) => {
                    st.wallet_address = Some(address);
                    st.wallet_balance = Some(0.0);
                    st.wallet_loading = false;
                }
                WorkerUpdate::WalletError(e) => {
                    st.status_message = Some(format!("Wallet error: {}", e));
                    st.wallet_loading = false;
                }
                WorkerUpdate::OllamaStatus(status) => {
                    if status == OllamaStatus::Running {
                        st.setup_dismissed = true;
                    }
                    st.ollama_status = status;
                    st.ollama_check_in_flight = false;
                }
                WorkerUpdate::ServerStarted(port) => {
                    st.server_port = port;
                }
                WorkerUpdate::StatusMessage(msg) => {
                    st.status_message = Some(msg);
                }
                WorkerUpdate::AnimCodeGenerated(code) => {
                    st.anim_generated_code = code;
                    st.anim_status = AnimationStatus::GeneratingCode;
                }
                WorkerUpdate::AnimRendering => {
                    st.anim_status = AnimationStatus::Rendering;
                }
                WorkerUpdate::AnimDone(path) => {
                    st.anim_status = AnimationStatus::Done(path);
                }
                WorkerUpdate::AnimError(e) => {
                    st.anim_status = AnimationStatus::Error(e);
                }
                WorkerUpdate::ManimInstalled(installed) => {
                    st.anim_manim_installed = Some(installed);
                }
                WorkerUpdate::AutomateDone { tool, result } => {
                    match tool.as_str() {
                        "social" => { st.auto_social_result = result; st.auto_social_loading = false; }
                        "email"  => { st.auto_email_result  = result; st.auto_email_loading  = false; }
                        "seo"    => { st.auto_seo_result    = result; st.auto_seo_loading    = false; }
                        _ => {}
                    }
                }
                WorkerUpdate::AutomateError { tool, error } => {
                    let msg = format!("Error: {}", error);
                    match tool.as_str() {
                        "social" => { st.auto_social_result = msg; st.auto_social_loading = false; }
                        "email"  => { st.auto_email_result  = msg; st.auto_email_loading  = false; }
                        "seo"    => { st.auto_seo_result    = msg; st.auto_seo_loading    = false; }
                        _ => {}
                    }
                }
            
                WorkerUpdate::WebsiteGenerated(html) => {
                    st.web_generated_html = html;
                    st.web_status = WebsiteStatus::Generated;
                }
                WorkerUpdate::WebsiteDeployed { local_port, public_url } => {
                    st.web_local_port = local_port;
                    st.web_public_url = Some(public_url);
                    st.web_status = WebsiteStatus::Live;
                }
                WorkerUpdate::WebsiteStopped => {
                    st.web_status = WebsiteStatus::Generated;
                    st.web_public_url = None;
                }
                WorkerUpdate::WebsiteError(e) => {
                    st.web_status = WebsiteStatus::Error(e);
                }

                WorkerUpdate::WidgetSettingsLoaded { bot_name, welcome, system_prompt, color } => {
                    st.widget_bot_name = bot_name;
                    st.widget_welcome = welcome;
                    st.widget_system_prompt = system_prompt;
                    st.widget_color = color;
                }
            }
        }
    }

    fn poll_wa_if_needed(&mut self) {
        let (should_poll, session_id) = {
            let st = self.state.lock().unwrap();
            let should = matches!(
                st.wa_session_state.as_str(),
                "awaiting_qr_scan" | "qr_ready" | "connecting"
            ) && st.wa_session_id.is_some();
            let sid = st.wa_session_id.clone().unwrap_or_default();
            (should, sid)
        };

        if should_poll && !session_id.is_empty() {
            self.wa_poll_timer += 0.016;
            if self.wa_poll_timer >= 2.0 {
                self.wa_poll_timer = 0.0;
                let _ = self.cmd_tx.send(UiCommand::PollWaStatus(session_id));
            }
        } else {
            self.wa_poll_timer = 0.0;
        }
    }
}

// ── Material-style helpers ────────────────────────────────────────────────────

pub fn card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(Color32::WHITE)
        .rounding(Rounding::same(12.0))
        .shadow(epaint::Shadow {
            offset: Vec2::new(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(18),
        })
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| add_contents(ui));
}

pub fn material_button(ui: &mut Ui, label: &str) -> Response {
    let btn = egui::Button::new(
        RichText::new(label)
            .size(13.0)
            .color(Color32::WHITE)
            .strong(),
    )
    .fill(Color32::BLACK)
    .rounding(Rounding::same(8.0))
    .min_size(Vec2::new(80.0, 36.0));
    ui.add(btn)
}

pub fn material_button_outlined(ui: &mut Ui, label: &str) -> Response {
    let btn = egui::Button::new(
        RichText::new(label).size(13.0).color(Color32::BLACK),
    )
    .fill(Color32::WHITE)
    .rounding(Rounding::same(8.0))
    .stroke(Stroke::new(1.0, Color32::from_gray(200)))
    .min_size(Vec2::new(80.0, 36.0));
    ui.add(btn)
}

// ── eframe App ────────────────────────────────────────────────────────────────

impl eframe::App for MimonaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_updates();
        self.poll_wa_if_needed();
        {
            let mut st = self.state.lock().unwrap();
            // Only send CheckOllama once and wait for the result. Previously
            // this fired on every repaint (dozens of times/sec) as long as
            // status stayed `Checking`, and each one spawned a real
            // subprocess to probe for the ollama binary — flooding the
            // screen with ollama.exe windows/crash dialogs on Windows.
            if st.ollama_status == OllamaStatus::Checking && !st.ollama_check_in_flight {
                st.ollama_check_in_flight = true;
                drop(st);
                let _ = self.cmd_tx.send(UiCommand::CheckOllama);
            } else {
                drop(st);
            }
        }
         // Also handle DismissSetup command from setup panel
        {
            let mut st = self.state.lock().unwrap();
            // DismissSetup is handled directly here
            if st.ollama_status == OllamaStatus::Running {
                st.setup_dismissed = true;
            }
        }

        {
            let st = self.state.lock().unwrap();
            if st.chat_thinking
                || !matches!(st.wa_session_state.as_str(), "idle" | "" | "connected")
                || st.pull_progress.is_some()
                || matches!(st.anim_status, AnimationStatus::GeneratingCode | AnimationStatus::Rendering)
                || matches!(st.web_status, WebsiteStatus::Generating | WebsiteStatus::Deploying)
            {
                ctx.request_repaint();
            }
        }

        // ── Top bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar")
            .exact_height(52.0)
            .frame(egui::Frame::none()
                .fill(Color32::WHITE)
                .shadow(epaint::Shadow {
                    offset: Vec2::new(0.0, 1.0),
                    blur: 4.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(15),
                }))
            .show(ctx, |ui| {
                 let rect = ui.max_rect();
                let response = ui.interact(rect, ui.id(), egui::Sense::click_and_drag());
                if response.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::BLACK);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "M",
                        FontId::proportional(17.0),
                        Color32::WHITE,
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Mimona")
                            .size(17.0)
                            .strong()
                            .color(Color32::BLACK),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Your local AI runtime")
                            .size(12.0)
                            .color(Color32::GRAY),
                    );
                    // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //     ui.add_space(16.0);

                    //     if ui.button(RichText::new("x").size(13.0).strong().color(Color32::from_gray(80))).clicked() {
                    //         ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    //     }
                    //     ui.add_space(4.0);

                    //     let is_maximized = ctx.input(|i|i.viewport().maximized.unwrap_or(false));
                    //     let max_icon = if is_maximized { "🗗" } else { "🗖" };
                    //     if ui.button(RichText::new(max_icon).size(13.0).color(Color32::from_gray(80))).clicked() {
                    //         ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    //     }

                    //     ui.add_space(4.0);
                    //     if ui.button(RichText::new("🗕").size(13.0).color(Color32::from_gray(80))).clicked() {
                    //         ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    //     }

                    // });

                });
            });

        // ── Status bar ────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(28.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_gray(248))
                .inner_margin(Margin::symmetric(16.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let st = self.state.lock().unwrap();
                    let dot_color = Color32::from_rgb(34, 197, 94);
                    ui.label(RichText::new("●").size(8.0).color(dot_color));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("API running · http://127.0.0.1:{}", st.server_port))
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                    if let Some(ref msg) = st.status_message {
                        ui.add_space(12.0);
                        ui.label(RichText::new("·").size(11.0).color(Color32::GRAY));
                        ui.add_space(4.0);
                        ui.label(RichText::new(msg).size(11.0).color(Color32::GRAY));
                    }
                });
            });
        let screen_w = ctx.screen_rect().width();
        let sidebar_w = (screen_w * 0.16).clamp(180.0, 240.0);
        // ── Left sidebar ──────────────────────────────────────────────────
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .show_separator_line(false)
            .exact_width(sidebar_w)
            .frame(egui::Frame::none()
                .fill(Color32::from_gray(250))
                .inner_margin(Margin::symmetric(12.0, 16.0)))
            .show(ctx, |ui| {
                ui.add_space(8.0);

                let items: &[(Panel, &str, &str)] = &[
                    (Panel::Chat,      "chat",      "Chat"),
                    (Panel::WhatsApp,  "whatsapp",  "WhatsApp"),
                    (Panel::Automate,  "automate",  "Automate"),
                    (Panel::Models,    "models",    "Models"),
                    (Panel::Animation, "animation", "Animation"),
                    (Panel::Website,   "website",   "Website"),
                    (Panel::Widget,    "widget",    "Widget"),
                ];

                for (panel, _id, label) in items {
                  let selected = self.active_panel == *panel;

                  let icon = match panel {
                      Panel::Chat      => "💬",
                      Panel::WhatsApp  => "📱",
                      Panel::Models    => "🤖",
                      Panel::Animation => "🎬",
                      Panel::Website   => "🌐",
                      Panel::Automate  => "⚡",
                      Panel::Widget    => "🔌",
                 };

               let (bg, text_color) = if selected {
                 (Color32::BLACK, Color32::WHITE)
                    } else {
                (Color32::TRANSPARENT, Color32::from_gray(60))
            };

            // Calculate layout target width (sidebar width minus its inner horizontal margins)
            let item_width = sidebar_w - 24.0;

            // Use a unique ID for each interactable row container scope
            let response = ui.scope_builder(
                egui::UiBuilder::new().id_salt(format!("btn_{}", label)),

                |ui| {
                // Force the layout scope block to exactly match the desired button size
                ui.set_width(item_width);

                // Configure the container background layout styling safely
                   let mut frame = egui::Frame::none()
                        .fill(bg)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(14.0, 10.0));

                    let frame_res = frame.show(ui, |ui| {
                        ui.horizontal(|ui| {    
                        ui.label(RichText::new(icon).size(15.0));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(*label)
                                .size(13.5)
                                .color(text_color)
                                .strong(),
                        );
                    });
                });

                // Convert the entire frame bounds into an active clickable button zone
                ui.interact(frame_res.response.rect, ui.id(), Sense::click())
            },
        ).inner; // Extract the container click response directly

        // Fire state updates on click  
        if response.clicked() {
            self.active_panel = *panel;
            match panel {   
                Panel::Models   => { let _ = self.cmd_tx.send(UiCommand::RefreshModels); }
                Panel::WhatsApp => { let _ = self.cmd_tx.send(UiCommand::RefreshWaUsers); }
                    _ => {}
                }
            }
            ui.add_space(4.0);
            }

            });

        // ── Main content ──────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(Color32::from_gray(245))
                .inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                 // Show setup screen until Ollama is confirmed running
                let (ollama_ok, dismissed) = {
                    let st = self.state.lock().unwrap();
                    (st.ollama_status == OllamaStatus::Running, st.setup_dismissed)
                };
 
                if !ollama_ok && !dismissed {
                    panel_setup::draw(ui, &self.state, &self.cmd_tx);
                    return;
                }
                match self.active_panel {
                    Panel::Chat =>
                        panel_chat::draw(ui, &self.state, &self.cmd_tx),
                    Panel::WhatsApp =>
                        panel_whatsapp::draw(ui, &self.state, &self.cmd_tx, ctx),
                    Panel::Models =>
                        panel_models::draw(ui, &self.state, &self.cmd_tx),
                    Panel::Automate =>
                        panel_automate::draw(ui, &self.state, &self.cmd_tx),  
                    Panel::Animation =>
                        panel_animation::draw(ui, &self.state, &self.cmd_tx),
                    Panel::Website =>
                        panel_website::draw(ui, &self.state, &self.cmd_tx),
                    Panel::Widget =>
                        panel_widget::draw(ui, &self.state, &self.cmd_tx),
                }
            });
    }
}

// ── Theme ─────────────────────────────────────────────────────────────────────

fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill          = Color32::from_gray(245);
    visuals.window_fill         = Color32::WHITE;
    visuals.extreme_bg_color    = Color32::from_gray(245);
    visuals.faint_bg_color      = Color32::from_gray(248);
    visuals.widgets.noninteractive.bg_fill = Color32::from_gray(235);
    visuals.widgets.inactive.bg_fill       = Color32::WHITE;
    visuals.widgets.hovered.bg_fill        = Color32::from_gray(230);
    visuals.widgets.active.bg_fill         = Color32::from_gray(210);
    visuals.selection.bg_fill              = Color32::BLACK;
    visuals.selection.stroke.color         = Color32::WHITE;
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_gray(40);
    visuals.widgets.inactive.fg_stroke.color       = Color32::from_gray(60);
    visuals.override_text_color = Some(Color32::from_gray(20));
    visuals.window_rounding     = Rounding::same(12.0);
    // visuals.popup_shadow = epaint::Shadow::NONE;
    // visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    // visuals.window_stroke = Stroke::NONE;
    visuals.window_shadow       = epaint::Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(20),
    };
    ctx.set_visuals(visuals);
}

fn setup_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading,   FontId::proportional(22.0)),
        (TextStyle::Body,      FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
        (TextStyle::Button,    FontId::proportional(13.5)),
        (TextStyle::Small,     FontId::proportional(11.5)),
    ].into();
    style.spacing.item_spacing   = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.window_margin  = Margin::same(0.0);
    ctx.set_style(style);
}