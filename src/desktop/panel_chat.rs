use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::{card, material_button};

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let mut available = ui.available_rect_before_wrap();
    // On the first frame(s) — especially under software/Zink GL fallback —
    // the window may not have reported a real size yet, which can hand us a
    // degenerate or non-finite rect. Fall back to a sane default so the manual
    // arithmetic below can't produce NaN and crash egui's layout code.
    // this would work.
    if !available.width().is_finite() || available.width() < 1.0
        || !available.height().is_finite() || available.height() < 1.0
    {
        available = Rect::from_min_size(available.min, Vec2::new(800.0, 600.0));
    }
    let input_height = 72.0;
    let header_height = 52.0;

    // ── Header bar ────────────────────────────────────────────────────────
    let header_rect = Rect::from_min_size(
        available.min,
        Vec2::new(available.width(), header_height),
    );
    ui.allocate_ui_at_rect(header_rect, |ui| {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .shadow(epaint::Shadow {
                offset: Vec2::new(0.0, 1.0),
                blur: 4.0,
                spread: 0.0,
                color: Color32::from_black_alpha(12),
            })
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(available.width(), header_height));
                ui.horizontal_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Chat")
                            .size(17.0)
                            .strong()
                            .color(Color32::BLACK),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(20.0);
                        let mut st = state.lock().unwrap();
                        let models: Vec<String> = st.local_models.iter()
                            .map(|m| m.full_name())
                            .collect();
                        if models.is_empty() {
                            ui.label(
                                RichText::new("No models — go to Models tab")
                                    .size(12.0)
                                    .color(Color32::GRAY),
                            );
                        } else {
                            let current = if st.chat_model.is_empty() {
                                models[0].clone()
                            } else {
                                st.chat_model.clone()
                            };
                            egui::ComboBox::from_id_source("chat_model")
                                .selected_text(
                                    RichText::new(&current).size(12.5).color(Color32::BLACK)
                                )
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    for m in &models {
                                        ui.selectable_value(
                                            &mut st.chat_model,
                                            m.clone(),
                                            RichText::new(m).size(12.5),
                                        );
                                    }
                                });
                        }
                    });
                });
            });
    });

    // ── Input bar (bottom-anchored) ───────────────────────────────────────
    let input_rect = Rect::from_min_size(
        Pos2::new(available.min.x, available.max.y - input_height),
        Vec2::new(available.width(), input_height),
    );
    ui.allocate_ui_at_rect(input_rect, |ui| {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .shadow(epaint::Shadow {
                offset: Vec2::new(0.0, -1.0),
                blur: 6.0,
                spread: 0.0,
                color: Color32::from_black_alpha(12),
            })
            .inner_margin(Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(available.width(), input_height));
                let mut st = state.lock().unwrap();
                let thinking = st.chat_thinking;

                ui.horizontal_centered(|ui| {
                    // Text input
                    let input_w = (ui.available_width() - 100.0).max(50.0);
                    let te = egui::TextEdit::singleline(&mut st.chat_input)
                        .hint_text("Type a message…")
                        .font(TextStyle::Body)
                        .frame(true)
                        .desired_width(input_w)
                        .interactive(!thinking);

                    let input_resp = ui.add_sized([input_w, 42.0], te);
                    ui.add_space(8.0);

                    let send_clicked = material_button(ui, if thinking { "…" } else { "Send" }).clicked();
                    let enter = input_resp.lost_focus()
                        && ui.input(|i| i.key_pressed(Key::Enter));

                    if (send_clicked || enter) && !thinking && !st.chat_input.trim().is_empty() {
                        send_message(&mut st, cmd_tx);
                    }
                });
            });
    });

    // ── Message scroll area (between header and input) ────────────────────
    let messages_rect = Rect::from_min_max(
        Pos2::new(available.min.x, available.min.y + header_height),
        Pos2::new(available.max.x, available.max.y - input_height),
    );
    ui.allocate_ui_at_rect(messages_rect, |ui| {
        egui::Frame::none()
            .fill(Color32::from_gray(245))
            .inner_margin(Margin::symmetric(24.0, 16.0))
            .show(ui, |ui| {
                let st = state.lock().unwrap();
                if st.chat_history.is_empty() {
                    // Empty state
                    let center = ui.available_rect_before_wrap().center();
                    ui.allocate_ui_at_rect(
                        Rect::from_center_size(center, Vec2::new(320.0, 120.0)),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(
                                    RichText::new("Start a conversation")
                                        .size(18.0)
                                        .strong()
                                        .color(Color32::BLACK),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("Type a message below to chat with your local AI model.")
                                        .size(13.0)
                                        .color(Color32::GRAY),
                                );
                            });
                        },
                    );
                    return;
                }

                let msgs = st.chat_history.clone();
                drop(st);

                ScrollArea::vertical()
                    .id_source("chat_messages")
                    .stick_to_bottom(true)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for msg in &msgs {
                            draw_bubble(ui, msg);
                            ui.add_space(10.0);
                        }
                    });
            });
    });
}

fn send_message(st: &mut AppState, cmd_tx: &CmdSender) {
    let text = st.chat_input.trim().to_string();
    if text.is_empty() { return; }
    st.chat_input.clear();
    st.chat_thinking = true;

    st.chat_history.push(ChatMessage {
        role: "user".into(),
        content: text,
        pending: false,
    });
    st.chat_history.push(ChatMessage {
        role: "assistant".into(),
        content: String::new(),
        pending: true,
    });

    let model = if st.chat_model.is_empty() {
        st.local_models.first().map(|m| m.full_name()).unwrap_or_default()
    } else {
        st.chat_model.clone()
    };

    let messages: Vec<(String, String)> = st.chat_history.iter()
        .filter(|m| !m.pending)
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect();

    let _ = cmd_tx.send(UiCommand::SendMessage {
        model,
        messages,
        system: "You are a helpful AI assistant. Be concise and direct.".into(),
    });
}

fn draw_bubble(ui: &mut Ui, msg: &ChatMessage) {
    let is_user = msg.role == "user";
    // Avatar width + spacing for assistant bubbles
    let avatar_w = 38.0;
    let total_w = ui.available_width();
    let total_w = if total_w.is_finite() && total_w > 0.0 { total_w } else { 400.0 };
    let max_bubble_w = (total_w * 0.72).max(200.0);

    if is_user {
        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
            // Constrain bubble width before the frame so it wraps correctly
            ui.set_max_width(max_bubble_w);
            egui::Frame::none()
                .fill(Color32::BLACK)
                .rounding(Rounding { nw: 16.0, ne: 4.0, sw: 16.0, se: 16.0 })
                .inner_margin(Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&msg.content)
                            .size(14.0)
                            .color(Color32::WHITE),
                    );
                });
        });
    } else {
        ui.horizontal_top(|ui| {
            // Avatar
            let (av_rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
            ui.painter().circle_filled(av_rect.center(), 15.0, Color32::BLACK);
            ui.painter().text(
                av_rect.center(),
                Align2::CENTER_CENTER,
                "M",
                FontId::proportional(13.0),
                Color32::WHITE,
            );
            ui.add_space(8.0);

            // Bubble capped at max_bubble_w, remainder stays empty
            let bubble_w = ((total_w - avatar_w).min(max_bubble_w)).max(60.0);
            ui.allocate_ui_with_layout(
                Vec2::new(bubble_w, 0.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    egui::Frame::none()
                        .fill(Color32::WHITE)
                        .rounding(Rounding { nw: 4.0, ne: 16.0, sw: 16.0, se: 16.0 })
                        .shadow(epaint::Shadow {
                            offset: Vec2::new(0.0, 1.0),
                            blur: 4.0,
                            spread: 0.0,
                            color: Color32::from_black_alpha(12),
                        })
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_max_width(bubble_w - 28.0); // subtract inner margins
                            if msg.content.is_empty() && msg.pending {
                                ui.spinner();
                            } else {
                                let display = if msg.pending {
                                    format!("{}▊", msg.content)
                                } else {
                                    msg.content.clone()
                                };
                                ui.label(
                                    RichText::new(&display)
                                        .size(14.0)
                                        .color(Color32::BLACK),
                                );
                            }
                        });
                },
            );
        });
    }
}