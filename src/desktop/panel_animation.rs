use egui::*;
use std::sync::{Arc, Mutex};

use super::state::*;
use super::app::{material_button, material_button_outlined};

pub fn draw(ui: &mut Ui, state: &Arc<Mutex<AppState>>, cmd_tx: &CmdSender) {
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(24.0);

            // Center content with max width
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

    // Check manim on first load
    if st.anim_manim_installed.is_none() {
        st.anim_manim_installed = Some(false);
        let _ = cmd_tx.send(UiCommand::CheckManimInstalled);
    }

    // ── Title ─────────────────────────────────────────────────────────────
    ui.label(RichText::new("Animation").size(22.0).strong().color(Color32::BLACK));
    ui.add_space(4.0);
    ui.label(
        RichText::new("Describe an animation — Mimona writes the code and renders it as a video.")
            .size(13.0)
            .color(Color32::GRAY),
    );
    ui.add_space(16.0);

    // ── Manim warning ─────────────────────────────────────────────────────
    if st.anim_manim_installed == Some(false) {
        egui::Frame::none()
            .fill(Color32::from_rgb(255, 249, 235))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(16.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Manim not detected — install it to enable rendering")
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(146, 64, 14)),
                        );
                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(Color32::from_rgb(254, 243, 199))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::symmetric(10.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("pip install manim")
                                        .monospace()
                                        .size(12.5)
                                        .color(Color32::BLACK),
                                );
                            });
                        ui.add_space(6.0);
                        if ui.small_button("Check again").clicked() {
                            let _ = cmd_tx.send(UiCommand::CheckManimInstalled);
                        }
                    });
                });
            });
        ui.add_space(16.0);
    }

    // ── Prompt card ───────────────────────────────────────────────────────
    egui::Frame::none()
        .fill(Color32::WHITE)
        .rounding(Rounding::same(12.0))
        .shadow(epaint::Shadow {
            offset: Vec2::new(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(15),
        })
        .inner_margin(Margin::same(20.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(
                RichText::new("Describe your animation")
                    .size(14.0)
                    .strong()
                    .color(Color32::BLACK),
            );
            ui.add_space(10.0);

            ui.add(
                TextEdit::multiline(&mut st.anim_prompt)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .hint_text(
                        "e.g. \"A red circle grows and splits into two blue circles\"\n\
                         or \"Show a binary search tree inserting the value 42\"\n\
                         or \"Animate the Fibonacci sequence building up\""
                    )
                    .font(TextStyle::Body),
            );

            ui.add_space(14.0);

            let is_busy = matches!(
                st.anim_status,
                AnimationStatus::GeneratingCode | AnimationStatus::Rendering
            );
            let can_generate = !is_busy && !st.anim_prompt.trim().is_empty();

            ui.horizontal(|ui| {
                let btn = egui::Button::new(
                    RichText::new(if is_busy { "Working…" } else { "✨  Generate & Render" })
                        .size(13.5)
                        .color(if can_generate { Color32::WHITE } else { Color32::from_gray(160) })
                        .strong(),
                )
                .fill(if can_generate { Color32::BLACK } else { Color32::from_gray(210) })
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(180.0, 38.0));

                if ui.add_enabled(can_generate, btn).clicked() {
                    let prompt = st.anim_prompt.trim().to_string();
                    st.anim_status = AnimationStatus::GeneratingCode;
                    st.anim_generated_code.clear();
                    let _ = cmd_tx.send(UiCommand::GenerateAnimation(prompt));
                }

                if is_busy {
                    ui.add_space(12.0);
                    ui.spinner();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(match &st.anim_status {
                            AnimationStatus::GeneratingCode => "AI is writing the animation code…",
                            AnimationStatus::Rendering      => "Rendering video with Manim…",
                            _                               => "",
                        })
                        .size(12.5)
                        .color(Color32::GRAY),
                    );
                }
            });
        });

    ui.add_space(16.0);

    // ── Result ────────────────────────────────────────────────────────────
    match st.anim_status.clone() {
        AnimationStatus::Done(ref video_path) => {
            egui::Frame::none()
                .fill(Color32::WHITE)
                .rounding(Rounding::same(12.0))
                .shadow(epaint::Shadow {
                    offset: Vec2::new(0.0, 2.0),
                    blur: 8.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(15),
                })
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().circle_filled(
                            dot.center(), 5.0, Color32::from_rgb(22, 163, 74),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Animation ready!")
                                .size(15.0).strong().color(Color32::BLACK),
                        );
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(video_path.as_str())
                            .monospace().size(11.5).color(Color32::GRAY),
                    );
                    ui.add_space(12.0);

                    // Video preview area
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 200.0),
                        Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::from_gray(15));
                    ui.painter().text(
                        rect.center() - Vec2::new(0.0, 14.0),
                        Align2::CENTER_CENTER,
                        "🎬",
                        FontId::proportional(40.0),
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        rect.center() + Vec2::new(0.0, 24.0),
                        Align2::CENTER_CENTER,
                        "Click \"Open Video\" to play",
                        FontId::proportional(13.0),
                        Color32::from_gray(140),
                    );

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        let path = video_path.clone();
                        if material_button(ui, "▶  Open Video").clicked() {
                            let _ = cmd_tx.send(UiCommand::OpenVideo(path));
                        }
                        ui.add_space(8.0);
                        if material_button_outlined(ui, "Generate Another").clicked() {
                            st.anim_status = AnimationStatus::Idle;
                            st.anim_generated_code.clear();
                        }
                        ui.add_space(8.0);
                        let code_label = if st.anim_show_code { "Hide Code" } else { "View Code" };
                        if material_button_outlined(ui, code_label).clicked() {
                            st.anim_show_code = !st.anim_show_code;
                        }
                    });

                    if st.anim_show_code && !st.anim_generated_code.is_empty() {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Generated code").size(13.0).strong().color(Color32::BLACK),
                        );
                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(Color32::from_gray(248))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&st.anim_generated_code)
                                            .monospace().size(12.0).color(Color32::BLACK),
                                    );
                                });
                            });
                    }
                });
        }

        AnimationStatus::Error(ref msg) => {
            egui::Frame::none()
                .fill(Color32::from_rgb(254, 242, 242))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✕").size(16.0).color(Color32::from_rgb(220, 38, 38)));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Render failed")
                                .size(14.0).strong().color(Color32::from_rgb(185, 28, 28)),
                        );
                    });
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(254, 226, 226))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                ui.label(
                                    RichText::new(msg.as_str())
                                        .monospace().size(11.5)
                                        .color(Color32::from_rgb(127, 29, 29)),
                                );
                            });
                        });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if material_button(ui, "Try Again").clicked() {
                            st.anim_status = AnimationStatus::Idle;
                        }
                        if !st.anim_generated_code.is_empty() {
                            ui.add_space(8.0);
                            let lbl = if st.anim_show_code { "Hide Code" } else { "View Code" };
                            if material_button_outlined(ui, lbl).clicked() {
                                st.anim_show_code = !st.anim_show_code;
                            }
                        }
                    });
                    if st.anim_show_code && !st.anim_generated_code.is_empty() {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(Color32::from_gray(248))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::same(10.0))
                            .show(ui, |ui| {
                                ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&st.anim_generated_code)
                                            .monospace().size(11.5).color(Color32::BLACK),
                                    );
                                });
                            });
                    }
                });
        }

        _ => {
            // Idle — example prompts
            egui::Frame::none()
                .fill(Color32::WHITE)
                .rounding(Rounding::same(12.0))
                .shadow(epaint::Shadow {
                    offset: Vec2::new(0.0, 1.0),
                    blur: 4.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(10),
                })
                .inner_margin(Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Try one of these")
                            .size(13.5).strong().color(Color32::BLACK),
                    );
                    ui.add_space(10.0);

                    let examples = [
                        ("🔴", "A red circle grows and splits into two blue circles"),
                        ("🌳", "A binary search tree inserting the value 42"),
                        ("📊", "A bar chart of 5 values animating from zero to final heights"),
                        ("🌀", "The Fibonacci spiral drawing itself step by step"),
                        ("🔢", "The number 42 being factored into prime numbers"),
                    ];

                    for (icon, text) in examples {
                        let resp = egui::Frame::none()
                            .fill(Color32::from_gray(248))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(12.0, 10.0))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).size(16.0));
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new(text)
                                            .size(13.0)
                                            .color(Color32::from_gray(50)),
                                    );
                                });
                            });

                        if resp.response
                            .interact(Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            st.anim_prompt = text.to_string();
                        }
                        ui.add_space(6.0);
                    }
                });
        }
    }

    ui.add_space(24.0);
}