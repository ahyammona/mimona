use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::{material_button, material_button_outlined};

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
    ui.label(RichText::new("🌐 Website Builder").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Describe your brand and Mimona builds a real website — then publishes it live to the internet.\n\
             Your site stays public as long as Mimona is running."
        )
        .size(13.0)
        .color(Color32::GRAY),
    );
    ui.add_space(16.0);

    let status = st.web_status.clone();

    // ── Live banner ───────────────────────────────────────────────────────
    if let WebsiteStatus::Live = &status {
        let url = st.web_public_url.clone().unwrap_or_default();
        egui::Frame::none()
            .fill(Color32::from_rgb(240, 253, 244))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, Color32::from_rgb(22, 163, 74));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Your website is LIVE")
                                .size(15.0).strong().color(Color32::from_rgb(22, 163, 74)),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&url)
                                    .monospace().size(12.5)
                                    .color(Color32::from_rgb(22, 100, 50)),
                            );
                            ui.add_space(8.0);
                            if ui.small_button("📋 Copy").clicked() {
                                ui.output_mut(|o| o.copied_text = url.clone());
                            }
                            if ui.small_button("🔗 Open").clicked() {
                                let _ = cmd_tx.send(UiCommand::OpenBrowser(url.clone()));
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Share this link — it works on any device, anywhere in the world.")
                                .size(11.5).color(Color32::from_rgb(22, 100, 50)),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let stop_btn = egui::Button::new(
                            RichText::new("Stop").size(12.5).color(Color32::from_rgb(220, 38, 38))
                        )
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(220, 38, 38)))
                        .rounding(Rounding::same(8.0));
                        if ui.add(stop_btn).clicked() {
                            let _ = cmd_tx.send(UiCommand::StopWebsite);
                        }
                    });
                });
            });
        ui.add_space(16.0);
    }

    // ── Build form ────────────────────────────────────────────────────────
    egui::Frame::none()
        .fill(Color32::WHITE)
        .rounding(Rounding::same(12.0))
        .shadow(epaint::Shadow {
            offset: Vec2::new(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(12),
        })
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let is_busy = matches!(status, WebsiteStatus::Generating | WebsiteStatus::Deploying);

            ui.label(RichText::new("Brand details").size(14.0).strong().color(Color32::BLACK));
            ui.add_space(12.0);

            // Brand name
            ui.label(RichText::new("Brand / Business name").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(&mut st.web_brand)
                    .desired_width(f32::INFINITY)
                    .hint_text("e.g. Zara's Boutique, TechBridge Academy, QuickFix Lagos")
                    .interactive(!is_busy),
            );

            ui.add_space(12.0);

            // Description
            ui.label(RichText::new("What does your business do?").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut st.web_description)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2)
                    .hint_text("e.g. We sell premium women's fashion in Abuja. Dresses, bags and shoes ₦5,000–₦80,000.")
                    .interactive(!is_busy),
            );

            ui.add_space(12.0);

            // Services
            ui.label(RichText::new("Services / Products offered").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut st.web_services)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2)
                    .hint_text("e.g. Evening gowns, casual wear, accessories, custom orders, home delivery")
                    .interactive(!is_busy),
            );

            ui.add_space(12.0);

            // Contact
            ui.label(RichText::new("Contact information").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(&mut st.web_contact)
                    .desired_width(f32::INFINITY)
                    .hint_text("e.g. +234 801 234 5678 | info@zarasboutique.com | 12 Wuse Zone 4, Abuja")
                    .interactive(!is_busy),
            );

            ui.add_space(12.0);

            // Site type + color row
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Website type").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    if st.web_site_type.is_empty() {
                        st.web_site_type = "Single page".to_string();
                    }
                    ComboBox::from_id_source("web_type")
                        .selected_text(&st.web_site_type)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for t in ["Single page", "Multi-section", "Landing page"] {
                                ui.selectable_value(&mut st.web_site_type, t.to_string(), t);
                            }
                        });
                });

                ui.add_space(20.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new("Brand color").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    if st.web_color.is_empty() {
                        st.web_color = "#6c5ce7".to_string();
                    }
                    let colors = [
                        ("#6c5ce7", "Purple"),
                        ("#0984e3", "Blue"),
                        ("#00b894", "Green"),
                        ("#e17055", "Orange"),
                        ("#d63031", "Red"),
                        ("#2d3436", "Dark"),
                        ("#f39c12", "Gold"),
                    ];
                    ComboBox::from_id_source("web_color")
                        .selected_text(
                            colors.iter()
                                .find(|(hex, _)| *hex == st.web_color)
                                .map(|(_, name)| *name)
                                .unwrap_or("Custom")
                        )
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for (hex, name) in &colors {
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                                    if let Ok(c) = hex_to_color(hex) {
                                        ui.painter().rect_filled(rect, Rounding::same(3.0), c);
                                    }
                                    ui.selectable_value(&mut st.web_color, hex.to_string(), *name);
                                });
                            }
                        });
                });
            });

            ui.add_space(18.0);

            // Generate button
            let can_generate = !st.web_brand.is_empty()
                && !st.web_description.is_empty()
                && !is_busy;

            ui.horizontal(|ui| {
                let gen_label = match &status {
                    WebsiteStatus::Generating => "✨ Generating website…",
                    WebsiteStatus::Deploying  => "🚀 Deploying…",
                    _                         => "✨ Build Website",
                };

                let btn = egui::Button::new(
                    RichText::new(gen_label)
                        .size(13.5)
                        .color(if can_generate { Color32::WHITE } else { Color32::from_gray(160) })
                        .strong(),
                )
                .fill(if can_generate { Color32::BLACK } else { Color32::from_gray(210) })
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(160.0, 38.0));

                if ui.add_enabled(can_generate, btn).clicked() {
                    st.web_status = WebsiteStatus::Generating;
                    st.web_generated_html.clear();
                    st.web_public_url = None;
                    let _ = cmd_tx.send(UiCommand::GenerateWebsite {
                        brand: st.web_brand.clone(),
                        description: st.web_description.clone(),
                        services: st.web_services.clone(),
                        contact: st.web_contact.clone(),
                        site_type: st.web_site_type.clone(),
                        color: st.web_color.clone(),
                    });
                }

                if is_busy {
                    ui.add_space(12.0);
                    ui.spinner();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(match &status {
                            WebsiteStatus::Generating => "AI is building your website…",
                            WebsiteStatus::Deploying  => "Creating public tunnel…",
                            _ => "",
                        })
                        .size(12.5).color(Color32::GRAY),
                    );
                }
            });
        });

    // ── Result ────────────────────────────────────────────────────────────
    match &status {
        WebsiteStatus::Generated | WebsiteStatus::Live => {
            ui.add_space(16.0);
            egui::Frame::none()
                .fill(Color32::WHITE)
                .rounding(Rounding::same(12.0))
                .shadow(epaint::Shadow {
                    offset: Vec2::new(0.0, 2.0),
                    blur: 8.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(12),
                })
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("✓ Website generated")
                                .size(14.0).strong().color(Color32::BLACK),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let show = st.web_show_code;
                            if material_button_outlined(
                                ui,
                                if show { "Hide code" } else { "View HTML" }
                            ).clicked() {
                                st.web_show_code = !show;
                            }
                        });
                    });

                    ui.add_space(12.0);

                    // Preview box
                    egui::Frame::none()
                        .fill(Color32::from_gray(240))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(0.0))
                        .show(ui, |ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 140.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::from_gray(20));
                            // Browser chrome bar
                            let bar = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 28.0));
                            ui.painter().rect_filled(bar, Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, Color32::from_gray(45));
                            ui.painter().circle_filled(bar.min + Vec2::new(14.0, 14.0), 5.0, Color32::from_rgb(255, 95, 86));
                            ui.painter().circle_filled(bar.min + Vec2::new(28.0, 14.0), 5.0, Color32::from_rgb(255, 189, 46));
                            ui.painter().circle_filled(bar.min + Vec2::new(42.0, 14.0), 5.0, Color32::from_rgb(39, 201, 63));
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                format!("🌐  {}.website", st.web_brand.to_lowercase().replace(' ', "-")),
                                FontId::proportional(13.0),
                                Color32::from_gray(160),
                            );
                        });

                    ui.add_space(14.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        // Open locally
                        if material_button_outlined(ui, "🔍 Preview locally").clicked() {
                            let port = if st.web_local_port > 0 { st.web_local_port } else { 11436 };
                            let _ = cmd_tx.send(UiCommand::OpenBrowser(
                                format!("http://localhost:{}", port)
                            ));
                        }

                        ui.add_space(8.0);

                        // Deploy / already live
                        if matches!(status, WebsiteStatus::Live) {
                            let url = st.web_public_url.clone().unwrap_or_default();
                            if material_button(ui, "🔗 Open live site").clicked() {
                                let _ = cmd_tx.send(UiCommand::OpenBrowser(url));
                            }
                        } else {
                            let deploy_btn = egui::Button::new(
                                RichText::new("🚀 Publish to internet").size(13.0).color(Color32::WHITE).strong()
                            )
                            .fill(Color32::from_rgb(22, 163, 74))
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(180.0, 36.0));

                            if ui.add(deploy_btn).clicked() {
                                st.web_status = WebsiteStatus::Deploying;
                                let brand = st.web_brand.clone();
                                let html = st.web_generated_html.clone();
                                let tx = cmd_tx.clone();
                                // Spawn deploy directly
                                let _ = cmd_tx.send(UiCommand::GenerateWebsite {
                                    brand: brand.clone(),
                                    description: String::new(),
                                    services: String::new(),
                                    contact: String::new(),
                                    site_type: "deploy_only".into(),
                                    color: String::new(),
                                });
                                // Actually trigger deploy via a dedicated path
                                // We repurpose the brand field to signal deploy
                                // The worker will recognize "deploy_only" and skip generation
                            }
                        }
                    });

                    // Cloudflared install hint
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Color32::from_gray(248))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ℹ").size(13.0).color(Color32::GRAY));
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "For a public URL, install cloudflared:  curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o cloudflared && chmod +x cloudflared && sudo mv cloudflared /usr/local/bin/"
                                    )
                                    .size(11.0).color(Color32::GRAY),
                                );
                            });
                        });

                    // Code viewer
                    if st.web_show_code && !st.web_generated_html.is_empty() {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Generated HTML").size(13.0).strong().color(Color32::BLACK));
                            if ui.small_button("📋 Copy").clicked() {
                                ui.output_mut(|o| o.copied_text = st.web_generated_html.clone());
                            }
                        });
                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(Color32::from_gray(248))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                                    ui.add(
                                        TextEdit::multiline(&mut st.web_generated_html)
                                            .desired_width(f32::INFINITY)
                                            .font(TextStyle::Monospace),
                                    );
                                });
                            });
                    }
                });
        }

        WebsiteStatus::Error(msg) => {
            ui.add_space(16.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(254, 242, 242))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✕").size(16.0).color(Color32::from_rgb(220, 38, 38)));
                        ui.add_space(6.0);
                        ui.label(RichText::new("Generation failed").size(14.0).strong().color(Color32::from_rgb(185, 28, 28)));
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(msg.as_str()).size(12.5).color(Color32::from_rgb(127, 29, 29)));
                    ui.add_space(10.0);
                    if material_button(ui, "Try Again").clicked() {
                        st.web_status = WebsiteStatus::Idle;
                    }
                });
        }

        _ => {}
    }

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