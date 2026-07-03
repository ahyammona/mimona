use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::{card, material_button, material_button_outlined};

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.set_max_width(720.0);
                    draw_content(ui, state, cmd_tx);
                });
            });
        });
}

fn draw_content(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let mut st = state.lock().unwrap();

    // ── Header ────────────────────────────────────────────────────────────
    ui.label(RichText::new("🔌 Widget / Embed").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Add an AI chat bubble to any website — yours or a client's.\n\
             Paste one line of code and visitors can chat with your local AI."
        )
        .size(13.0)
        .color(Color32::GRAY),
    );
    ui.add_space(20.0);

    // ── Tunnel status ─────────────────────────────────────────────────────
    let tunnel_url = st.web_public_url.clone();
    let is_live = matches!(st.web_status, WebsiteStatus::Live);

    let base_url = if let Some(ref url) = tunnel_url {
        // strip trailing slash
        url.trim_end_matches('/').to_string()
    } else {
        format!("http://localhost:{}", st.server_port)
    };

    let embed_code = format!(
        "<script src=\"{}/widget.js\"></script>",
        base_url
    );

    // Status banner
    if is_live {
        egui::Frame::none()
            .fill(Color32::from_rgb(240, 253, 244))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(), 5.0, Color32::from_rgb(22, 163, 74),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Tunnel active — widget is publicly accessible")
                                .size(13.5).strong().color(Color32::from_rgb(22, 163, 74)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(&base_url)
                                .monospace().size(12.0)
                                .color(Color32::from_rgb(22, 100, 50)),
                        );
                    });
                });
            });
    } else {
        egui::Frame::none()
            .fill(Color32::from_rgb(255, 251, 235))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(16.0).color(Color32::from_rgb(217, 119, 6)));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("No public tunnel — widget only works on your machine")
                                .size(13.5).strong().color(Color32::from_rgb(146, 64, 14)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(
                                "Go to the Website panel → Build a site → Publish to internet \
                                 to get a public URL your visitors can reach."
                            )
                            .size(12.0).color(Color32::from_rgb(180, 100, 20)),
                        );
                    });
                });
            });
    }

    ui.add_space(20.0);

    // ── Embed code ────────────────────────────────────────────────────────
    card(ui, |ui| {
        ui.label(
            RichText::new("Your embed code").size(14.0).strong().color(Color32::BLACK),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Paste this single line before </body> on any webpage.")
                .size(12.5).color(Color32::GRAY),
        );
        ui.add_space(12.0);

        egui::Frame::none()
            .fill(Color32::from_gray(20))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&embed_code)
                            .monospace()
                            .size(12.5)
                            .color(Color32::from_rgb(134, 239, 172)),
                    );
                });
            });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if material_button(ui, "📋 Copy code").clicked() {
                ui.output_mut(|o| o.copied_text = embed_code.clone());
            }
            ui.add_space(8.0);
            if material_button_outlined(ui, "🔍 Test widget").clicked() {
                let _ = cmd_tx.send(UiCommand::OpenBrowser(
                    format!("{}/widget", base_url)
                ));
            }
        });
    });

    ui.add_space(16.0);

    // ── Customize ─────────────────────────────────────────────────────────
    card(ui, |ui| {
        ui.label(
            RichText::new("Customize").size(14.0).strong().color(Color32::BLACK),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("These settings apply to the chat bubble on your website.")
                .size(12.5).color(Color32::GRAY),
        );
        ui.add_space(14.0);

        // Bot name
        ui.label(
            RichText::new("Bot name").size(12.5).color(Color32::from_gray(60)),
        );
        ui.add_space(4.0);
        ui.add(
            TextEdit::singleline(&mut st.widget_bot_name)
                .desired_width(f32::INFINITY)
                .hint_text("AI Assistant"),
        );

        ui.add_space(12.0);

        // Welcome message
        ui.label(
            RichText::new("Welcome message").size(12.5).color(Color32::from_gray(60)),
        );
        ui.add_space(4.0);
        ui.add(
            TextEdit::singleline(&mut st.widget_welcome)
                .desired_width(f32::INFINITY)
                .hint_text("Hi! How can I help you today?"),
        );

        ui.add_space(12.0);

        // System prompt
        ui.label(
            RichText::new("AI personality / system prompt").size(12.5).color(Color32::from_gray(60)),
        );
        ui.add_space(4.0);
        ui.add(
            TextEdit::multiline(&mut st.widget_system_prompt)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .hint_text(
                    "You are a helpful assistant for [Business Name]. \
                     Be friendly and concise. Answer questions about our products and services."
                ),
        );

        ui.add_space(12.0);

        // Bubble color
        ui.label(
            RichText::new("Bubble color").size(12.5).color(Color32::from_gray(60)),
        );
        ui.add_space(4.0);

        let colors = [
            ("#000000", "Black"),
            ("#6c5ce7", "Purple"),
            ("#0984e3", "Blue"),
            ("#00b894", "Green"),
            ("#e17055", "Orange"),
            ("#d63031", "Red"),
        ];

        if st.widget_color.is_empty() {
            st.widget_color = "#000000".to_string();
        }

        ui.horizontal(|ui| {
            for (hex, name) in &colors {
                let selected = st.widget_color == *hex;
                let color = hex_to_color(hex).unwrap_or(Color32::BLACK);

                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                ui.painter().rect_filled(rect, Rounding::same(6.0), color);
                if selected {
                    ui.painter().rect_stroke(
                        rect.expand(2.0),
                        Rounding::same(8.0),
                        Stroke::new(2.0, Color32::BLACK),
                    );
                }
                if resp.clicked() {
                    st.widget_color = hex.to_string();
                }
                if resp.hovered() {
                    ui.painter().rect_stroke(
                        rect,
                        Rounding::same(6.0),
                        Stroke::new(1.5, Color32::WHITE),
                    );
                }
                ui.add_space(4.0);
            }
        });

        ui.add_space(16.0);

        if material_button(ui, "💾 Save settings").clicked() {
            let _ = cmd_tx.send(UiCommand::SaveWidgetSettings {
                bot_name: st.widget_bot_name.clone(),
                welcome: st.widget_welcome.clone(),
                system_prompt: st.widget_system_prompt.clone(),
                color: st.widget_color.clone(),
            });
            st.widget_saved = true;
        }

        if st.widget_saved {
            ui.add_space(6.0);
            ui.label(
                RichText::new("✓ Settings saved — widget updated live")
                    .size(12.5).color(Color32::from_rgb(22, 163, 74)),
            );
        }
    });

    ui.add_space(16.0);

    // ── How to use ────────────────────────────────────────────────────────
    card(ui, |ui| {
        ui.label(
            RichText::new("How to add to your website").size(14.0).strong().color(Color32::BLACK),
        );
        ui.add_space(12.0);

        let steps = [
            ("1", "Start the tunnel", "Go to Website panel → Build a site → Publish to internet"),
            ("2", "Copy the embed code", "Click \"Copy code\" above — it has your live URL already in it"),
            ("3", "Paste into your website", "Add it just before </body> in your HTML, or in your CMS theme footer"),
            ("4", "Done", "Visitors will see a chat bubble. Keep Mimona running while your site is live"),
        ];

        for (num, title, desc) in &steps {
            ui.horizontal(|ui| {
                // Step number circle
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 13.0, Color32::BLACK);
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    num,
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(*title).size(13.0).strong().color(Color32::BLACK),
                    );
                    ui.label(
                        RichText::new(*desc).size(12.0).color(Color32::GRAY),
                    );
                });
            });
            ui.add_space(10.0);
        }

        ui.add_space(4.0);

        // WordPress / Wix / Shopify tips
        egui::Frame::none()
            .fill(Color32::from_gray(248))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Platform-specific:").size(12.0).strong().color(Color32::BLACK),
                );
                ui.add_space(4.0);
                ui.label(RichText::new("• WordPress — Appearance → Theme Editor → footer.php").size(11.5).color(Color32::GRAY));
                ui.label(RichText::new("• Wix — Settings → Custom Code → Add to Body").size(11.5).color(Color32::GRAY));
                ui.label(RichText::new("• Shopify — Online Store → Themes → Edit code → theme.liquid").size(11.5).color(Color32::GRAY));
                ui.label(RichText::new("• Plain HTML — paste before </body> tag").size(11.5).color(Color32::GRAY));
            });
    });

    ui.add_space(24.0);
}

fn hex_to_color(hex: &str) -> Result<Color32, ()> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return Err(()); }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?;
    Ok(Color32::from_rgb(r, g, b))
}