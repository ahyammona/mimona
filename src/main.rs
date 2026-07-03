mod assistant;
mod config;
mod desktop;
mod inference;
mod models;
mod node;
mod payment;
mod server;
mod whatsapp;
mod whatsapp_bridge_launcher;
mod widget;

use anyhow::Result;

fn main() -> Result<()> {
    // Suppress noisy logs — users don't see a terminal
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Mimona")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            // Fix 1: Disables native OS decorations to prevent WSLg window border glitching
            .with_decorations(false) 
            .with_icon(
                eframe::icon_data::from_png_bytes(
                    include_bytes!("../assets/icon.png")
                ).unwrap_or_default()
            ),
        // Fix 2: Hardcodes Dark theme to bypass the slow 100ms XDG portal timeout check
        //theme: eframe::Theme::Dark, 
        ..Default::default()
    };
    

    // let options = eframe::NativeOptions {
    //     viewport: eframe::egui::ViewportBuilder::default()
    //         .with_title("Mimona")
    //         .with_inner_size([1100.0, 700.0])
    //         .with_min_inner_size([800.0, 500.0])
    //         .with_icon(
    //             eframe::icon_data::from_png_bytes(
    //                 include_bytes!("../assets/icon.png")
    //             ).unwrap_or_default()
    //         ),
    //     ..Default::default()
    // };

    eframe::run_native(
        "Mimona",
        options,
        Box::new(|cc| Ok(Box::new(desktop::MimonaApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("Window error: {}", e))?;

    Ok(())
}