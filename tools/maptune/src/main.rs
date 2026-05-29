//! Interactive world-generation parameter tuning tool.
//!
//! Launches a native egui window with a parameter form, figure display,
//! and sweep mode. Calls into `worldgen` directly — no subprocess, no
//! disk I/O for the visual output.

#![allow(missing_docs)] // binary crate — no public API to document

mod app;
mod figure_panel;
mod figures;
mod params_panel;
mod runner;
mod sweep;
mod sweep_panel;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("maptune")
            .with_inner_size([1400.0, 900.0]),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "maptune",
        options,
        Box::new(|cc| Ok(Box::new(app::MaptuneApp::new(cc)))),
    )
}
