//! Pinacoteca desktop application.
//!
//! This binary owns the window, input handling and GPU rendering. It delegates
//! domain logic to `pinacoteca-core` and never writes to the original image
//! files.

mod app;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pinacoteca")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Pinacoteca",
        options,
        Box::new(|_cc| Ok(Box::new(app::PinacotecaApp::default()))),
    )
}
