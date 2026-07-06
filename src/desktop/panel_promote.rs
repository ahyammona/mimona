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
    ui.label(RichText::new("📣 Promote").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "AI-generated promotional posts for Telegram channels and Reddit communities you own or manage."
        )
        .size(13.0)
        .color(Color32::GRAY),
    );

    ui.add_space(8.0);

    // Disclosure notice
    egui::Frame::none()
        .fill(Color32::from_rgb(239, 246, 255))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("ℹ").size(14.0).color(Color32::from_rgb(37, 99, 235)));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "This tool posts via official APIs to channels/communities you own or have posting permission for. \
                         Always disclose sponsored content per platform rules."
                    )
                    .size(12.0)
                    .color(Color32::from_rgb(30, 64, 175)),
                );
            });
        });

    ui.add_space(20.0);

    // ── Platform tabs ─────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        let platforms = [PromotePlatform::Telegram, PromotePlatform::Reddit, PromotePlatform::Both];
        for platform in &platforms {
            let selected = st.promote_platform == *platform;
            let label = match platform {
                PromotePlatform::Telegram => "📱 Telegram",
                PromotePlatform::Reddit   => "🤖 Reddit",
                PromotePlatform::Both     => "🔀 Both",
            };
            let btn = egui::Button::new(
                RichText::new(label).size(13.0)
                    .color(if selected { Color32::WHITE } else { Color32::BLACK })
                    .strong()
            )
            .fill(if selected { Color32::BLACK } else { Color32::from_gray(235) })
            .rounding(Rounding::same(8.0))
            .min_size(Vec2::new(100.0, 34.0));

            if ui.add(btn).clicked() {
                st.promote_platform = platform.clone();
            }
            ui.add_space(6.0);
        }
    });

    ui.add_space(20.0);

    // ── Campaign details ──────────────────────────────────────────────────
    card(ui, |ui| {
        ui.label(RichText::new("Campaign details").size(14.0).strong().color(Color32::BLACK));
        ui.add_space(12.0);

        ui.label(RichText::new("Product / Service name").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        ui.add(
            TextEdit::singleline(&mut st.promote_product)
                .desired_width(f32::INFINITY)
                .hint_text("e.g. Mimona — local AI runtime"),
        );

        ui.add_space(12.0);

        ui.label(RichText::new("What makes it great? (key benefits)").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        ui.add(
            TextEdit::multiline(&mut st.promote_benefits)
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .hint_text("e.g. Runs AI locally, no cloud, private, free after install"),
        );

        ui.add_space(12.0);

        ui.label(RichText::new("Call to action / Link").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        ui.add(
            TextEdit::singleline(&mut st.promote_cta)
                .desired_width(f32::INFINITY)
                .hint_text("e.g. Download free at github.com/Ahyammona/mimona"),
        );

        ui.add_space(12.0);

        ui.label(RichText::new("Post tone").size(12.5).color(Color32::from_gray(60)));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for tone in ["Casual", "Professional", "Excited", "Educational"] {
                let selected = st.promote_tone == tone;
                let btn = egui::Button::new(RichText::new(tone).size(12.0)
                    .color(if selected { Color32::WHITE } else { Color32::BLACK }))
                    .fill(if selected { Color32::BLACK } else { Color32::from_gray(235) })
                    .rounding(Rounding::same(6.0));
                if ui.add(btn).clicked() {
                    st.promote_tone = tone.to_string();
                }
                ui.add_space(4.0);
            }
        });

        ui.add_space(12.0);

        // Disclosure checkbox
        ui.horizontal(|ui| {
            ui.checkbox(&mut st.promote_disclose, "");
            ui.label(
                RichText::new("I confirm this is my channel/community or I have permission to post here, and I will disclose this as promotional content.")
                    .size(12.0)
                    .color(Color32::from_gray(60)),
            );
        });
    });

    ui.add_space(16.0);

    // ── Telegram config ───────────────────────────────────────────────────
    if matches!(st.promote_platform, PromotePlatform::Telegram | PromotePlatform::Both) {
        card(ui, |ui| {
            ui.label(RichText::new("📱 Telegram Bot Setup").size(14.0).strong().color(Color32::BLACK));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Create a bot at t.me/BotFather, add it as admin to your channel, then paste the token below.")
                    .size(12.0).color(Color32::GRAY),
            );
            ui.add_space(12.0);

            ui.label(RichText::new("Bot Token").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::singleline(&mut st.promote_tg_token)
                    .desired_width(f32::INFINITY)
                    .hint_text("1234567890:ABCdefGHI...")
                    .password(true),
            );

            ui.add_space(12.0);

            ui.label(RichText::new("Channel / Group (one per line, use @username or chat ID)").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut st.promote_tg_channels)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("@mychannel\n-1001234567890"),
            );
        });
        ui.add_space(16.0);
    }

    // ── Reddit config ─────────────────────────────────────────────────────
    if matches!(st.promote_platform, PromotePlatform::Reddit | PromotePlatform::Both) {
        card(ui, |ui| {
            ui.label(RichText::new("🤖 Reddit App Setup").size(14.0).strong().color(Color32::BLACK));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Create an app at reddit.com/prefs/apps (script type). You must be a moderator of the subreddits you post to.")
                    .size(12.0).color(Color32::GRAY),
            );
            ui.add_space(12.0);

            let col_w = (ui.available_width() - 12.0) / 2.0;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    ui.label(RichText::new("Client ID").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    ui.add(
                        TextEdit::singleline(&mut st.promote_reddit_client_id)
                            .desired_width(col_w)
                            .hint_text("xXxXxXxXxXxX")
                            .password(true),
                    );
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    ui.label(RichText::new("Client Secret").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    ui.add(
                        TextEdit::singleline(&mut st.promote_reddit_secret)
                            .desired_width(col_w)
                            .hint_text("xXxXxXxXxXxXxXxXxXxX")
                            .password(true),
                    );
                });
            });

            ui.add_space(12.0);

            let col_w2 = (ui.available_width() - 12.0) / 2.0;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(col_w2);
                    ui.label(RichText::new("Reddit Username").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    ui.add(
                        TextEdit::singleline(&mut st.promote_reddit_username)
                            .desired_width(col_w2)
                            .hint_text("u/yourusername"),
                    );
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.set_width(col_w2);
                    ui.label(RichText::new("Password").size(12.5).color(Color32::from_gray(60)));
                    ui.add_space(4.0);
                    ui.add(
                        TextEdit::singleline(&mut st.promote_reddit_password)
                            .desired_width(col_w2)
                            .hint_text("••••••••")
                            .password(true),
                    );
                });
            });

            ui.add_space(12.0);

            ui.label(RichText::new("Subreddits (one per line, must be communities you moderate)").size(12.5).color(Color32::from_gray(60)));
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut st.promote_reddit_subs)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .hint_text("r/myproject\nr/mycommunity"),
            );
        });
        ui.add_space(16.0);
    }

    // ── Generate & Post button ────────────────────────────────────────────
    let can_post = !st.promote_product.is_empty()
        && !st.promote_cta.is_empty()
        && st.promote_disclose
        && !st.promote_loading
        && match st.promote_platform {
            PromotePlatform::Telegram => !st.promote_tg_token.is_empty() && !st.promote_tg_channels.is_empty(),
            PromotePlatform::Reddit   => !st.promote_reddit_client_id.is_empty() && !st.promote_reddit_subs.is_empty(),
            PromotePlatform::Both     => !st.promote_tg_token.is_empty() && !st.promote_reddit_client_id.is_empty(),
        };

    ui.horizontal(|ui| {
        let label = if st.promote_loading { "Generating & posting…" } else { "✨ Generate & Post" };
        let btn = egui::Button::new(
            RichText::new(label).size(13.5)
                .color(if can_post { Color32::WHITE } else { Color32::from_gray(160) })
                .strong()
        )
        .fill(if can_post { Color32::BLACK } else { Color32::from_gray(210) })
        .rounding(Rounding::same(8.0))
        .min_size(Vec2::new(180.0, 40.0));

        if ui.add_enabled(can_post, btn).clicked() {
            st.promote_loading = true;
            st.promote_result.clear();
            let _ = cmd_tx.send(UiCommand::RunPromotion {
                platform: st.promote_platform.clone(),
                product: st.promote_product.clone(),
                benefits: st.promote_benefits.clone(),
                cta: st.promote_cta.clone(),
                tone: st.promote_tone.clone(),
                model: st.chat_model.clone(),
                tg_token: st.promote_tg_token.clone(),
                tg_channels: st.promote_tg_channels.lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                reddit_client_id: st.promote_reddit_client_id.clone(),
                reddit_secret: st.promote_reddit_secret.clone(),
                reddit_username: st.promote_reddit_username.clone(),
                reddit_password: st.promote_reddit_password.clone(),
                reddit_subs: st.promote_reddit_subs.lines()
                    .map(|s| s.trim().trim_start_matches("r/").to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            });
        }

        if st.promote_loading {
            ui.add_space(12.0);
            ui.spinner();
        }
    });

    // ── Results ───────────────────────────────────────────────────────────
    if !st.promote_result.is_empty() {
        ui.add_space(16.0);
        let (is_error, color) = if st.promote_result.starts_with("Error") {
            (true, Color32::from_rgb(254, 242, 242))
        } else {
            (false, Color32::from_rgb(240, 253, 244))
        };

        egui::Frame::none()
            .fill(color)
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let text_color = if is_error {
                    Color32::from_rgb(185, 28, 28)
                } else {
                    Color32::from_rgb(22, 101, 52)
                };
                ui.label(RichText::new(&st.promote_result).size(13.0).color(text_color));
            });
    }

    ui.add_space(24.0);
}