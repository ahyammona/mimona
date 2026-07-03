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
                    ui.set_max_width(760.0);
                    draw_content(ui, state, cmd_tx);
                });
            });
        });
}

fn draw_content(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    // ── Title ─────────────────────────────────────────────────────────────
    ui.label(
        RichText::new("Automate")
            .size(22.0)
            .strong()
            .color(Color32::BLACK),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new("AI-powered business tools — generate content, emails, and SEO copy locally.")
            .size(13.0)
            .color(Color32::GRAY),
    );
    ui.add_space(16.0);

    // ── Tab bar ───────────────────────────────────────────────────────────
    let active_tab = {
        let st = state.lock().unwrap();
        st.auto_tab.clone()
    };

    ui.horizontal(|ui| {
        let tabs = [
            (AutomateTab::Social, "⚡ Social Media"),
            (AutomateTab::Email,  "✉ Cold Email"),
            (AutomateTab::Seo,    "🔍 Local SEO"),
        ];
        for (tab, label) in &tabs {
            let selected = active_tab == *tab;
            let (bg, text_col) = if selected {
                (Color32::BLACK, Color32::WHITE)
            } else {
                (Color32::from_gray(230), Color32::from_gray(60))
            };
            let frame = egui::Frame::none()
                .fill(bg)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(16.0, 8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(*label)
                            .size(13.0)
                            .color(text_col)
                            .strong(),
                    );
                });
            if frame.response.interact(Sense::click()).clicked() {
                let mut st = state.lock().unwrap();
                st.auto_tab = tab.clone();
            }
            ui.add_space(4.0);
        }
    });

    ui.add_space(16.0);

    // ── Active tool ───────────────────────────────────────────────────────
    match active_tab {
        AutomateTab::Social => draw_social(ui, state, cmd_tx),
        AutomateTab::Email  => draw_email(ui, state, cmd_tx),
        AutomateTab::Seo    => draw_seo(ui, state, cmd_tx),
    }
}

// ── Social Media Content Factory ─────────────────────────────────────────────

fn draw_social(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let (brand, topic, platforms, result, loading, model) = {
        let st = state.lock().unwrap();
        (
            st.auto_social_brand.clone(),
            st.auto_social_topic.clone(),
            st.auto_social_platforms.clone(),
            st.auto_social_result.clone(),
            st.auto_social_loading,
            st.chat_model.clone(),
        )
    };

    card(ui, |ui| {
        ui.label(RichText::new("Social Media Content Factory").size(15.0).strong().color(Color32::BLACK));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Give it a brand and topic — get ready-to-post content for every platform.")
                .size(12.5)
                .color(Color32::GRAY),
        );
        ui.add_space(12.0);

        // Brand
        ui.label(RichText::new("Brand / Business name").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut brand_edit = brand.clone();
        let brand_field = egui::TextEdit::singleline(&mut brand_edit)
            .hint_text("e.g. Lagos Grill House")
            .desired_width(f32::INFINITY)
            .margin(Margin::same(8.0));
        if ui.add(brand_field).changed() {
            state.lock().unwrap().auto_social_brand = brand_edit;
        }
        ui.add_space(10.0);

        // Topic
        ui.label(RichText::new("Topic / campaign angle").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut topic_edit = topic.clone();
        let topic_field = egui::TextEdit::singleline(&mut topic_edit)
            .hint_text("e.g. Weekend promo — 20% off all suya orders")
            .desired_width(f32::INFINITY)
            .margin(Margin::same(8.0));
        if ui.add(topic_field).changed() {
            state.lock().unwrap().auto_social_topic = topic_edit;
        }
        ui.add_space(10.0);

        // Platforms
        ui.label(RichText::new("Platforms (comma-separated)").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut plat_edit = platforms.clone();
        let plat_field = egui::TextEdit::singleline(&mut plat_edit)
            .hint_text("e.g. Instagram, Twitter, LinkedIn")
            .desired_width(f32::INFINITY)
            .margin(Margin::same(8.0));
        if ui.add(plat_field).changed() {
            state.lock().unwrap().auto_social_platforms = plat_edit;
        }
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let ready = !brand.trim().is_empty() && !topic.trim().is_empty() && !loading;
            ui.add_enabled_ui(ready, |ui| {
                if material_button(ui, if loading { "Generating…" } else { "Generate Content" }).clicked() {
                    let st = state.lock().unwrap();
                    let _ = cmd_tx.send(UiCommand::GenerateSocialContent {
                        brand: st.auto_social_brand.clone(),
                        topic: st.auto_social_topic.clone(),
                        platforms: if st.auto_social_platforms.trim().is_empty() {
                            "Instagram, Twitter, LinkedIn".to_string()
                        } else {
                            st.auto_social_platforms.clone()
                        },
                        model: model.clone(),
                    });
                    drop(st);
                    state.lock().unwrap().auto_social_loading = true;
                }
            });
            if !result.is_empty() {
                ui.add_space(8.0);
                if material_button_outlined(ui, "Clear").clicked() {
                    let mut st = state.lock().unwrap();
                    st.auto_social_result.clear();
                }
            }
        });
    });

    if !result.is_empty() {
        ui.add_space(12.0);
        draw_result_box(ui, &result, state, "social_copy");
    }
}

// ── Cold Email Writer ─────────────────────────────────────────────────────────

fn draw_email(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let (product, audience, count_str, result, loading, model) = {
        let st = state.lock().unwrap();
        (
            st.auto_email_product.clone(),
            st.auto_email_audience.clone(),
            st.auto_email_count.clone(),
            st.auto_email_result.clone(),
            st.auto_email_loading,
            st.chat_model.clone(),
        )
    };

    card(ui, |ui| {
        ui.label(RichText::new("Cold Email Writer").size(15.0).strong().color(Color32::BLACK));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Describe your product and audience — get personalised cold emails ready to send.")
                .size(12.5)
                .color(Color32::GRAY),
        );
        ui.add_space(12.0);

        // Product
        ui.label(RichText::new("Product / Service").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut prod_edit = product.clone();
        if ui.add(
            egui::TextEdit::multiline(&mut prod_edit)
                .hint_text("e.g. AI chatbot that handles customer support for e-commerce stores")
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_email_product = prod_edit;
        }
        ui.add_space(10.0);

        // Audience
        ui.label(RichText::new("Target audience").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut aud_edit = audience.clone();
        if ui.add(
            egui::TextEdit::singleline(&mut aud_edit)
                .hint_text("e.g. Shopify store owners doing $10k–$50k/month")
                .desired_width(f32::INFINITY)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_email_audience = aud_edit;
        }
        ui.add_space(10.0);

        // Count
        ui.label(RichText::new("Number of email variants").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut count_edit = count_str.clone();
        if count_edit.is_empty() { count_edit = "3".to_string(); }
        if ui.add(
            egui::TextEdit::singleline(&mut count_edit)
                .hint_text("3")
                .desired_width(80.0)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_email_count = count_edit;
        }
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let ready = !product.trim().is_empty() && !audience.trim().is_empty() && !loading;
            ui.add_enabled_ui(ready, |ui| {
                if material_button(ui, if loading { "Writing emails…" } else { "Write Emails" }).clicked() {
                    let st = state.lock().unwrap();
                    let count: u32 = st.auto_email_count.parse().unwrap_or(3).clamp(1, 10);
                    let _ = cmd_tx.send(UiCommand::GenerateColdEmails {
                        product: st.auto_email_product.clone(),
                        audience: st.auto_email_audience.clone(),
                        count,
                        model: model.clone(),
                    });
                    drop(st);
                    state.lock().unwrap().auto_email_loading = true;
                }
            });
            if !result.is_empty() {
                ui.add_space(8.0);
                if material_button_outlined(ui, "Clear").clicked() {
                    state.lock().unwrap().auto_email_result.clear();
                }
            }
        });
    });

    if !result.is_empty() {
        ui.add_space(12.0);
        draw_result_box(ui, &result, state, "email_copy");
    }
}

// ── Local SEO Content ─────────────────────────────────────────────────────────

fn draw_seo(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let (business, location, keywords, result, loading, model) = {
        let st = state.lock().unwrap();
        (
            st.auto_seo_business.clone(),
            st.auto_seo_location.clone(),
            st.auto_seo_keywords.clone(),
            st.auto_seo_result.clone(),
            st.auto_seo_loading,
            st.chat_model.clone(),
        )
    };

    card(ui, |ui| {
        ui.label(RichText::new("Local SEO Content").size(15.0).strong().color(Color32::BLACK));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Generate a full SEO content package — blog post, About Us, FAQs, and Google Business bio.")
                .size(12.5)
                .color(Color32::GRAY),
        );
        ui.add_space(12.0);

        // Business
        ui.label(RichText::new("Business name & type").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut biz_edit = business.clone();
        if ui.add(
            egui::TextEdit::singleline(&mut biz_edit)
                .hint_text("e.g. Mona's Bakery — artisan bread and cakes")
                .desired_width(f32::INFINITY)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_seo_business = biz_edit;
        }
        ui.add_space(10.0);

        // Location
        ui.label(RichText::new("Location").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut loc_edit = location.clone();
        if ui.add(
            egui::TextEdit::singleline(&mut loc_edit)
                .hint_text("e.g. Lekki, Lagos, Nigeria")
                .desired_width(f32::INFINITY)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_seo_location = loc_edit;
        }
        ui.add_space(10.0);

        // Keywords
        ui.label(RichText::new("Target keywords (comma-separated)").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        let mut kw_edit = keywords.clone();
        if ui.add(
            egui::TextEdit::singleline(&mut kw_edit)
                .hint_text("e.g. bakery in Lekki, custom cakes Lagos, fresh bread delivery")
                .desired_width(f32::INFINITY)
                .margin(Margin::same(8.0)),
        ).changed() {
            state.lock().unwrap().auto_seo_keywords = kw_edit;
        }
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let ready = !business.trim().is_empty() && !location.trim().is_empty() && !loading;
            ui.add_enabled_ui(ready, |ui| {
                if material_button(ui, if loading { "Generating…" } else { "Generate SEO Pack" }).clicked() {
                    let st = state.lock().unwrap();
                    let _ = cmd_tx.send(UiCommand::GenerateSeoContent {
                        business: st.auto_seo_business.clone(),
                        location: st.auto_seo_location.clone(),
                        keywords: st.auto_seo_keywords.clone(),
                        model: model.clone(),
                    });
                    drop(st);
                    state.lock().unwrap().auto_seo_loading = true;
                }
            });
            if !result.is_empty() {
                ui.add_space(8.0);
                if material_button_outlined(ui, "Clear").clicked() {
                    state.lock().unwrap().auto_seo_result.clear();
                }
            }
        });
    });

    if !result.is_empty() {
        ui.add_space(12.0);
        draw_result_box(ui, &result, state, "seo_copy");
    }
}

// ── Shared result box ─────────────────────────────────────────────────────────

fn draw_result_box(ui: &mut Ui, result: &str, state: &Arc<Mutex<AppState>>, _id: &str) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Result")
                    .size(13.5)
                    .strong()
                    .color(Color32::BLACK),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if material_button_outlined(ui, "Copy all").clicked() {
                    ui.output_mut(|o| o.copied_text = result.to_string());
                }
            });
        });
        ui.add_space(8.0);

        egui::Frame::none()
            .fill(Color32::from_gray(248))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ScrollArea::vertical()
                    .max_height(420.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(result)
                                .size(12.5)
                                .color(Color32::from_gray(30))
                                .monospace(),
                        );
                    });
            });
    });
}