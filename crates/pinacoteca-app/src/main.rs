//! Pinacoteca desktop application.
//!
//! This binary owns the window, input handling and GPU rendering. It delegates
//! domain logic to `pinacoteca-core` and never writes to the original image
//! files.

mod app;
mod editor;
mod theme;
mod workers;

fn main() -> eframe::Result<()> {
    // An optional folder argument prefills the library path, so
    // `cargo run -p pinacoteca-app -- D:\Pictures` opens straight into it.
    let folder = std::env::args().nth(1).unwrap_or_default();

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
        Box::new(move |_cc| {
            let mut app = app::PinacotecaApp::default();
            if !folder.is_empty() {
                app.set_folder(&folder);
            }
            Ok(Box::new(app))
        }),
    )
}
