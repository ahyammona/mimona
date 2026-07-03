use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;

// The content area (e.g. inside a ScrollArea) can occasionally report a
// non-finite or unbounded width — most often on the first frame(s), or under
// unstable GPU/window-sizing conditions. Feeding that straight into
// `set_min_width` can make a row "infinitely" wide, which then produces NaN
// when a `right_to_left` layout tries to compute where to place a widget.
// This clamps it to something sane first.
fn safe_row_width(ui: &Ui) -> f32 {
    let w = ui.available_width();
    if w.is_finite() && w > 0.0 {
        w
    } else {
        680.0
    }
}

const AVAILABLE_MODELS: &[(&str, &str, &str)] = &[
    ("tinyllama:1b",        "0.7 GB", "Tiny and fast — good for testing"),
    ("qwen2.5-coder:7b",   "4.7 GB", "Excellent for coding"),
    ("qwen2.5-coder:3b",   "2.0 GB", "Smaller Qwen, still capable"),
    ("llama3:8b",           "4.9 GB", "Great all-rounder"),
    ("mistral:7b",          "4.4 GB", "Fast, great for business chat"),
    ("phi3:mini",           "2.2 GB", "Microsoft — surprisingly smart"),
    ("deepseek-coder:6.7b", "4.1 GB", "Top coding model"),
];

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.set_max_width(720.0);
                    draw_models_content(ui, state, cmd_tx);
                });
            });
        });
}

fn draw_models_content(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let mut st = state.lock().unwrap();

    ui.label(RichText::new("Models").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(16.0);

    let local_names: Vec<String> = st.local_models.iter().map(|m| m.full_name()).collect();

    // ── Pull progress ──────────────────────────────────────────────────────
    if let Some(ref p) = st.pull_progress.clone() {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .rounding(Rounding::same(12.0))
            .shadow(epaint::Shadow {
                offset: Vec2::new(0.0, 1.0),
                blur: 4.0,
                spread: 0.0,
                color: Color32::from_black_alpha(10),
            })
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(safe_row_width(ui));

                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Downloading {}…", p.model))
                            .size(14.0).strong().color(Color32::BLACK),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cancel_btn = egui::Button::new(
                            RichText::new("Cancel").size(12.0).color(Color32::from_rgb(220, 38, 38)),
                        )
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(220, 38, 38)))
                        .rounding(Rounding::same(6.0));
                        if ui.add(cancel_btn).clicked() {
                            let _ = cmd_tx.send(UiCommand::CancelPull);
                        }
                    });
                });

                ui.add_space(10.0);

                let frac = if p.total_gb > 0.0 {
                    (p.downloaded_gb / p.total_gb).min(1.0)
                } else {
                    0.0
                };

                // A slim, subtly-colored bar rather than a tall solid block —
                // the progress text is shown separately underneath.
                ui.add(
                    ProgressBar::new(frac)
                        .desired_width(safe_row_width(ui))
                        .desired_height(6.0)
                        .fill(Color32::from_rgb(37, 99, 235))
                        .rounding(Rounding::same(3.0)),
                );

                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{:.2} / {:.2} GB", p.downloaded_gb, p.total_gb))
                        .size(11.5)
                        .color(Color32::GRAY),
                );
            });
        ui.add_space(16.0);
    }

    // ── Available models ───────────────────────────────────────────────────
    ui.label(RichText::new("Available models").size(15.0).strong().color(Color32::BLACK));
    ui.add_space(10.0);

    let mut to_pull: Option<String> = None;

    for (name, size, desc) in AVAILABLE_MODELS {
        let downloaded = local_names.contains(&name.to_string());

        ui.push_id(name, |ui| {
            egui::Frame::none()
                .fill(Color32::WHITE)
                .rounding(Rounding::same(12.0))
                .shadow(epaint::Shadow {
                    offset: Vec2::new(0.0, 1.0),
                    blur: 4.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(10),
                })
                .inner_margin(Margin::symmetric(16.0, 14.0))
                .show(ui, |ui| {
                    ui.set_min_width(safe_row_width(ui));
                    ui.horizontal(|ui| {
                        // Left: name + description
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(*name)
                                        .monospace()
                                        .size(14.0)
                                        .strong()
                                        .color(if downloaded {
                                            Color32::from_rgb(22, 163, 74)
                                        } else {
                                            Color32::BLACK
                                        }),
                                );
                                ui.add_space(6.0);
                                egui::Frame::none()
                                    .fill(Color32::from_gray(235))
                                    .rounding(Rounding::same(4.0))
                                    .inner_margin(Margin::symmetric(6.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(*size)
                                                .size(11.0)
                                                .color(Color32::from_gray(80)),
                                        );
                                    });
                            });
                            ui.add_space(2.0);
                            ui.label(RichText::new(*desc).size(12.5).color(Color32::GRAY));
                        });

                        // Right: action button
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if downloaded {
                                egui::Frame::none()
                                    .fill(Color32::from_rgb(220, 252, 231))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(Margin::symmetric(12.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("✓ Downloaded")
                                                .size(12.5)
                                                .color(Color32::from_rgb(22, 163, 74))
                                                .strong(),
                                        );
                                    });
                            } else {
                                let btn = egui::Button::new(
                                    RichText::new("⬇  Download")
                                        .size(13.0)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(Color32::BLACK)
                                .rounding(Rounding::same(8.0))
                                .min_size(Vec2::new(120.0, 36.0));

                                if ui.add(btn).clicked() {
                                    to_pull = Some(name.to_string());
                                }
                            }
                        });
                    });
                });
        });

        ui.add_space(8.0);
    }

    if let Some(name) = to_pull {
        let _ = cmd_tx.send(UiCommand::PullModel(name));
    }

    // ── Downloaded section ─────────────────────────────────────────────────
    if !local_names.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Downloaded").size(15.0).strong().color(Color32::BLACK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("⟳ Refresh").clicked() {
                    let _ = cmd_tx.send(UiCommand::RefreshModels);
                }
            });
        });
        ui.add_space(10.0);

        let models = st.local_models.clone();
        let mut to_delete: Option<String> = None;

        for m in &models {
            ui.push_id(m.full_name(), |ui| {
                egui::Frame::none()
                    .fill(Color32::WHITE)
                    .rounding(Rounding::same(10.0))
                    .shadow(epaint::Shadow {
                        offset: Vec2::new(0.0, 1.0),
                        blur: 3.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(8),
                    })
                    .inner_margin(Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_min_width(safe_row_width(ui));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(m.full_name())
                                    .monospace()
                                    .size(13.5)
                                    .color(Color32::BLACK),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("{:.2} GB", m.size_gb))
                                    .size(12.5)
                                    .color(Color32::GRAY),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let del = egui::Button::new(
                                    RichText::new("Delete")
                                        .size(12.0)
                                        .color(Color32::from_rgb(220, 38, 38)),
                                )
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(220, 38, 38)))
                                .rounding(Rounding::same(6.0));
                                if ui.add(del).clicked() {
                                    to_delete = Some(m.full_name());
                                }
                            });
                        });
                    });
            });
            ui.add_space(6.0);
        }

        if let Some(name) = to_delete {
            let _ = cmd_tx.send(UiCommand::DeleteModel(name));
        }
    }

    ui.add_space(24.0);
}