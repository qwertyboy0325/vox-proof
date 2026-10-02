//! VoxProof UI lab: an experiment in review ergonomics.
//!
//! Demo data and simulated audio only. This binary never reads or writes
//! VoxProof sessions, ledgers, or reviewed output.

mod app;
mod data;
mod matcher;
mod theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VoxProof UI Lab")
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([760.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "VoxProof UI Lab",
        options,
        Box::new(|cc| Ok(Box::new(app::Lab::new(&cc.egui_ctx)))),
    )
}
