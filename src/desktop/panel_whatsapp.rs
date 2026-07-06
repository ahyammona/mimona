use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::{card, material_button, material_button_outlined};

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender, ctx: &egui::Context) {
    let mut available = ui.available_rect_before_wrap();
    if !available.width().is_finite() || available.width() < 1.0
        || !available.height().is_finite() || available.height() < 1.0
    {
        available = Rect::from_min_size(available.min, Vec2::new(800.0, 600.0));
    }

    ui.allocate_ui_at_rect(available, |ui| {
        egui::Frame::none()
            .fill(Color32::from_gray(245))
            .inner_margin(Margin::symmetric(24.0, 20.0))
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.set_max_width(640.0);
                    draw_inner(ui, state, cmd_tx, ctx);
                });
            });
    });
}

fn draw_inner(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender, _ctx: &egui::Context) {
    let mut st = state.lock().unwrap();

    ui.label(RichText::new("WhatsApp").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(16.0);

    // Gate on the bridge status before showing the normal linking UI —
    // previously the panel always rendered "Scan QR Code" regardless of
    // whether the bridge was even reachable, so clicking it just produced
    // a raw "Bridge unreachable: error sending request for url (...)"
    // message. Now we check up front and explain what's actually wrong.
    let bridge_status = st.bridge_status.clone();
    match bridge_status {
        crate::whatsapp_bridge_launcher::BridgeStatus::Running => {
            // fall through to normal panel below
        }
        crate::whatsapp_bridge_launcher::BridgeStatus::Checking => {
            drop(st);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Checking WhatsApp bridge…").size(13.5).color(Color32::GRAY));
                });
            });
            return;
        }
        crate::whatsapp_bridge_launcher::BridgeStatus::Starting => {
            drop(st);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Starting WhatsApp bridge (first run installs dependencies, this can take a minute)…").size(13.5).color(Color32::GRAY));
                });
            });
            return;
        }
        crate::whatsapp_bridge_launcher::BridgeStatus::NotBundled => {
            drop(st);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(20.0).color(Color32::from_rgb(217, 119, 6)));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("WhatsApp bridge component not found").size(15.0).strong().color(Color32::from_rgb(146, 64, 14)));
                        ui.add_space(2.0);
                        ui.label(RichText::new(
                            "Mimona's WhatsApp support ships as a separate download alongside the app. \
                             Grab the assets bundle from the release page and unzip it next to Mimona \
                             (or into ~/.mimona/whatsapp-bridge), then come back here."
                        ).size(12.5).color(Color32::from_rgb(180, 100, 20)));
                    });
                });
            });
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                let btn = egui::Button::new(
                    RichText::new("⬇  Get WhatsApp Bridge").size(14.0).color(Color32::WHITE).strong()
                )
                .fill(Color32::BLACK)
                .rounding(Rounding::same(10.0))
                .min_size(Vec2::new(220.0, 44.0));

                if ui.add(btn).clicked() {
                    let _ = cmd_tx.send(UiCommand::OpenBrowser(
                        "https://github.com/ahyammona/mimona/releases/latest".to_string(),
                    ));
                }

                ui.add_space(12.0);
                if ui.link(RichText::new("Check again").size(12.5)).clicked() {
                    let _ = cmd_tx.send(UiCommand::CheckBridge);
                }
            });
            return;
        }
        crate::whatsapp_bridge_launcher::BridgeStatus::NotRunning => {
            drop(st);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ℹ").size(20.0).color(Color32::from_rgb(37, 99, 235)));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("WhatsApp bridge is installed but not running").size(15.0).strong().color(Color32::from_rgb(30, 64, 175)));
                        ui.add_space(2.0);
                        ui.label(RichText::new(
                            "Click below to start it — first run may take a moment to install dependencies."
                        ).size(12.5).color(Color32::from_rgb(30, 100, 200)));
                    });
                });
            });
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                let btn = egui::Button::new(
                    RichText::new("▶  Start Bridge").size(14.0).color(Color32::WHITE).strong()
                )
                .fill(Color32::BLACK)
                .rounding(Rounding::same(10.0))
                .min_size(Vec2::new(200.0, 44.0));

                if ui.add(btn).clicked() {
                    let _ = cmd_tx.send(UiCommand::StartBridge);
                }

                ui.add_space(12.0);
                if ui.link(RichText::new("Check again").size(12.5)).clicked() {
                    let _ = cmd_tx.send(UiCommand::CheckBridge);
                }
            });
            return;
        }
    }

    let session_state = st.wa_session_state.clone();
    let selected_phone = st.wa_selected_phone.clone();

    match session_state.as_str() {
        "idle" | "" => {
            // ── Linked accounts ───────────────────────────────────────────
            if !st.wa_users.is_empty() {
                let users = st.wa_users.clone();
                let mut to_unlink: Option<String> = None;
                let mut to_select: Option<(String, String)> = None;

                for user in &users {
                    card(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status dot
                            let dot_color = match user.status.as_str() {
                                "connected"    => Color32::from_rgb(22, 163, 74),
                                "disconnected" => Color32::from_rgb(220, 38, 38),
                                _              => Color32::GRAY,
                            };
                            let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                            ui.painter().circle_filled(dot.center(), 5.0, dot_color);
                            ui.add_space(8.0);

                            ui.vertical(|ui| {
                                ui.label(RichText::new(&user.phone_number).size(14.0).strong().color(Color32::BLACK));
                                ui.label(RichText::new(format!("Model: {} · {}", user.model, user.status)).size(12.0).color(Color32::GRAY));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if material_button_outlined(ui, "Unlink").clicked() {
                                    to_unlink = Some(user.phone_number.clone());
                                }
                                ui.add_space(6.0);
                                if material_button_outlined(ui, "Edit Prompt").clicked() {
                                    to_select = Some((user.phone_number.clone(), user.system_prompt.clone()));
                                }
                            });
                        });
                    });
                    ui.add_space(10.0);
                }

                if let Some(phone) = to_unlink {
                    let _ = cmd_tx.send(UiCommand::UnlinkWa(phone));
                }
                if let Some((phone, prompt)) = to_select {
                    st.wa_selected_phone = Some(phone);
                    st.wa_prompt_input = prompt;
                    st.wa_prompt_saved = false;
                }

                ui.add_space(6.0);
            }

            // ── Link new number ───────────────────────────────────────────
            card(ui, |ui| {
                ui.label(RichText::new("Link a WhatsApp number").size(14.0).strong().color(Color32::BLACK));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Scan a QR code with your phone. No phone number needed upfront.")
                        .size(12.5)
                        .color(Color32::GRAY),
                );
                ui.add_space(12.0);
                if material_button(ui, "Scan QR Code").clicked() {
                    st.wa_session_state = "connecting".to_string();
                    st.wa_qr = None;
                    st.wa_session_id = None;
                    let _ = cmd_tx.send(UiCommand::StartWaSession);
                }
            });

            // ── Prompt editor ─────────────────────────────────────────────
            if let Some(ref phone) = selected_phone.clone() {
                ui.add_space(16.0);
                card(ui, |ui| {
                    ui.label(
                        RichText::new(format!("Assistant prompt — {}", phone))
                            .size(14.0).strong().color(Color32::BLACK),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Changes apply on the next WhatsApp message. No relink needed.")
                            .size(12.0).color(Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.add(
                        TextEdit::multiline(&mut st.wa_prompt_input)
                            .desired_width(f32::INFINITY)
                            .desired_rows(5)
                            .hint_text("You are a helpful business assistant…")
                            .font(TextStyle::Body),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if material_button(ui, "Save").clicked() {
                            let prompt = st.wa_prompt_input.clone();
                            st.wa_prompt_saved = false;
                            let _ = cmd_tx.send(UiCommand::SaveWaPrompt {
                                phone: phone.clone(),
                                prompt,
                            });
                        }
                        ui.add_space(8.0);
                        if material_button_outlined(ui, "Cancel").clicked() {
                            st.wa_selected_phone = None;
                        }
                        if st.wa_prompt_saved {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("✓ Saved")
                                    .size(13.0)
                                    .color(Color32::from_rgb(22, 163, 74)),
                            );
                        }
                    });
                });
            }
        }

        "connecting" => {
            card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.spinner();
                    ui.add_space(12.0);
                    ui.label(RichText::new("Starting WhatsApp bridge…").size(14.0).color(Color32::GRAY));
                    ui.add_space(24.0);
                });
            });
        }

        "awaiting_qr_scan" | "qr_ready" => {
            card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("Scan to connect").size(18.0).strong().color(Color32::BLACK));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("WhatsApp → Settings → Linked Devices → Link a Device")
                            .size(12.5).color(Color32::GRAY),
                    );
                    ui.add_space(20.0);

                    if let Some(ref qr_data) = st.wa_qr.clone() {
                        draw_qr(ui, qr_data);
                    } else {
                        ui.add_space(20.0);
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(RichText::new("Generating QR code…").size(13.0).color(Color32::GRAY));
                        ui.add_space(20.0);
                    }

                    ui.add_space(16.0);
                    if material_button_outlined(ui, "Cancel").clicked() {
                        st.wa_session_state = "idle".to_string();
                        st.wa_qr = None;
                        st.wa_session_id = None;
                    }
                    ui.add_space(16.0);
                });
            });
        }

        "connected" => {
            card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("✓").size(40.0).color(Color32::from_rgb(22, 163, 74)));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Connected!").size(20.0).strong().color(Color32::BLACK));
                    if let Some(ref phone) = st.wa_selected_phone {
                        ui.add_space(4.0);
                        ui.label(RichText::new(phone).monospace().size(13.0).color(Color32::GRAY));
                    }
                    ui.add_space(16.0);
                    if material_button(ui, "Done").clicked() {
                        st.wa_session_state = "idle".to_string();
                        let _ = cmd_tx.send(UiCommand::RefreshWaUsers);
                    }
                    ui.add_space(24.0);
                });
            });
        }

        _ => {}
    }
}

fn draw_qr(ui: &mut Ui, data_url: &str) {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let b64 = data_url
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(data_url)
        .trim();

    let size = Vec2::splat(240.0);

    match STANDARD.decode(b64) {
        Ok(bytes) => {
            match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
                Ok(img) => {
                    let rgba = img.into_rgba8();
                    let (w, h) = rgba.dimensions();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        rgba.as_flat_samples().as_slice(),
                    );
                    let texture = ui.ctx().load_texture(
                        "wa_qr_code",
                        color_img,
                        egui::TextureOptions::NEAREST,
                    );
                    ui.add(egui::Image::new((texture.id(), size)));
                }
                Err(e) => {
                    eprintln!("[qr] png decode failed: {}", e);
                    qr_fallback(ui, size);
                }
            }
        }
        Err(e) => {
            eprintln!("[qr] base64 decode failed: {}", e);
            qr_fallback(ui, size);
        }
    }
}

fn qr_fallback(ui: &mut Ui, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::WHITE);
    ui.painter().rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.5, Color32::BLACK));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "QR ready — scan from\nthe terminal window",
        FontId::proportional(13.0),
        Color32::BLACK,
    );
}

fn base64_decode(s: &str) -> Result<Vec<u8>, ()> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let b = s.as_bytes();
    let dc = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => Some(0),
            _ => None,
        }
    };
    let mut i = 0;
    while i + 3 < b.len() {
        let a = dc(b[i]).ok_or(())?;
        let b_ = dc(b[i+1]).ok_or(())?;
        let c = dc(b[i+2]).ok_or(())?;
        let d = dc(b[i+3]).ok_or(())?;
        out.push((a << 2) | (b_ >> 4));
        if b[i+2] != b'=' { out.push((b_ << 4) | (c >> 2)); }
        if b[i+3] != b'=' { out.push((c << 2) | d); }
        i += 4;
    }
    Ok(out)
}