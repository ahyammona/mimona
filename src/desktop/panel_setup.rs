use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::material_button;

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    let st = state.lock().unwrap();
    let status = st.ollama_status.clone();
    drop(st);

    // Full screen centered layout
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.set_max_width(480.0);

            // Logo
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
            ui.painter().rect_filled(rect, Rounding::same(16.0), Color32::BLACK);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "M",
                FontId::proportional(36.0),
                Color32::WHITE,
            );

            ui.add_space(20.0);

            ui.label(
                RichText::new("Welcome to Mimona")
                    .size(26.0)
                    .strong()
                    .color(Color32::BLACK),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Your local AI runtime — chat, automate, and build with AI privately on your own machine.")
                    .size(14.0)
                    .color(Color32::GRAY),
            );

            ui.add_space(32.0);

            match status {
                OllamaStatus::Checking => {
                    draw_checking(ui);
                }
                OllamaStatus::NotInstalled => {
                    draw_not_installed(ui, cmd_tx);
                }
                OllamaStatus::NotRunning => {
                    draw_not_running(ui, cmd_tx);
                }
                OllamaStatus::Running => {
                    draw_ready(ui, cmd_tx);
                }
            }
        });
    });
}

fn draw_checking(ui: &mut Ui) {
    egui::Frame::none()
        .fill(Color32::from_gray(248))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(24.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.spinner();
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Checking system…")
                        .size(14.0)
                        .color(Color32::GRAY),
                );
            });
        });
}

fn draw_not_installed(ui: &mut Ui, cmd_tx: &CmdSender) {
    // Status card
    egui::Frame::none()
        .fill(Color32::from_rgb(255, 251, 235))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").size(20.0).color(Color32::from_rgb(217, 119, 6)));
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Ollama not installed")
                            .size(15.0).strong().color(Color32::from_rgb(146, 64, 14)),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Mimona needs Ollama to run AI models locally on your machine.")
                            .size(12.5).color(Color32::from_rgb(180, 100, 20)),
                    );
                });
            });
        });

    ui.add_space(20.0);

    // What is Ollama
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

            ui.label(RichText::new("What is Ollama?").size(14.0).strong().color(Color32::BLACK));
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Ollama lets you run AI models (like Llama, Mistral, Qwen) privately on your \
                     own computer — no internet required, no data sent to the cloud."
                )
                .size(13.0).color(Color32::GRAY),
            );

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Install steps
            let steps = [
                ("1", "Click Install Ollama below"),
                ("2", "Run the installer that downloads"),
                ("3", "Come back here — Mimona will detect it automatically"),
            ];

            for (num, text) in &steps {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 12.0, Color32::BLACK);
                    ui.painter().text(
                        rect.center(), Align2::CENTER_CENTER, num,
                        FontId::proportional(11.0), Color32::WHITE,
                    );
                    ui.add_space(10.0);
                    ui.label(RichText::new(*text).size(13.0).color(Color32::BLACK));
                });
                ui.add_space(8.0);
            }

            ui.add_space(8.0);

            // Install button
            ui.vertical_centered(|ui| {
                let btn = egui::Button::new(
                    RichText::new("⬇  Install Ollama").size(14.0).color(Color32::WHITE).strong()
                )
                .fill(Color32::BLACK)
                .rounding(Rounding::same(10.0))
                .min_size(Vec2::new(200.0, 44.0));

                if ui.add(btn).clicked() {
                    let _ = cmd_tx.send(UiCommand::InstallOllama);
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Opens ollama.com/download in your browser")
                        .size(11.5).color(Color32::GRAY),
                );
            });
        });

    ui.add_space(16.0);

    // Already installed? Re-check
    ui.horizontal_centered(|ui| {
        ui.label(RichText::new("Already installed?").size(12.5).color(Color32::GRAY));
        ui.add_space(6.0);
        if ui.link(RichText::new("Check again").size(12.5)).clicked() {
            let _ = cmd_tx.send(UiCommand::CheckOllama);
        }
    });
}

fn draw_not_running(ui: &mut Ui, cmd_tx: &CmdSender) {
    egui::Frame::none()
        .fill(Color32::from_rgb(239, 246, 255))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("ℹ").size(20.0).color(Color32::from_rgb(37, 99, 235)));
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Ollama is installed but not running")
                            .size(15.0).strong().color(Color32::from_rgb(30, 64, 175)),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Mimona needs Ollama running in the background to use AI models.")
                            .size(12.5).color(Color32::from_rgb(30, 100, 200)),
                    );
                });
            });
        });

    ui.add_space(20.0);

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
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Click the button below and Mimona will start Ollama for you.")
                        .size(13.0).color(Color32::GRAY),
                );
                ui.add_space(20.0);

                let btn = egui::Button::new(
                    RichText::new("▶  Start Ollama").size(14.0).color(Color32::WHITE).strong()
                )
                .fill(Color32::BLACK)
                .rounding(Rounding::same(10.0))
                .min_size(Vec2::new(200.0, 44.0));

                if ui.add(btn).clicked() {
                    let _ = cmd_tx.send(UiCommand::StartOllama);
                }

                ui.add_space(12.0);

                // Manual option
                egui::Frame::none()
                    .fill(Color32::from_gray(248))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Or start it manually:").size(12.0).color(Color32::GRAY));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("ollama serve")
                                .monospace().size(13.0).color(Color32::BLACK),
                        );
                    });

                ui.add_space(12.0);
                if ui.link(RichText::new("Check again").size(12.5)).clicked() {
                    let _ = cmd_tx.send(UiCommand::CheckOllama);
                }
            });
        });
}

fn draw_ready(ui: &mut Ui, cmd_tx: &CmdSender) {
    egui::Frame::none()
        .fill(Color32::from_rgb(240, 253, 244))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("✓").size(36.0).color(Color32::from_rgb(22, 163, 74)));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Ollama is running — you're all set!")
                        .size(16.0).strong().color(Color32::from_rgb(22, 163, 74)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Download a model to get started.")
                        .size(13.0).color(Color32::from_rgb(22, 100, 50)),
                );
                ui.add_space(20.0);

                let btn = egui::Button::new(
                    RichText::new("Get Started →").size(14.0).color(Color32::WHITE).strong()
                )
                .fill(Color32::from_rgb(22, 163, 74))
                .rounding(Rounding::same(10.0))
                .min_size(Vec2::new(200.0, 44.0));

                if ui.add(btn).clicked() {
                    let _ = cmd_tx.send(UiCommand::DismissSetup);
                }
            });
        });
}