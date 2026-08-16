mod app;
pub mod controller;
pub mod export;
mod fonts;
pub mod presentation;
mod session_root;

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VoxProof v0.2 Review")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "VoxProof v0.2 Review",
        options,
        Box::new(|cc| Ok(Box::new(app::ReviewApp::new(cc)))),
    )
}
