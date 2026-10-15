//! Application shell.
//!
//! At M0 this is a placeholder that proves the workspace wiring: it renders a
//! window and reads the core domain types. The grid and viewer are built in M3.

use eframe::egui;

use pinacoteca_core::Rating;

/// The top-level application state.
#[derive(Default)]
pub struct PinacotecaApp {
    /// Rating used to exercise the core domain wiring at M0.
    demo_rating: u8,
}

impl PinacotecaApp {
    /// Short status line shown in the placeholder window.
    fn status_line(&self) -> String {
        let max = Rating::MAX;
        format!(
            "Pinacoteca {} — placeholder shell (rating scale 0–{max})",
            env!("CARGO_PKG_VERSION")
        )
    }
}

impl eframe::App for PinacotecaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pinacoteca");
            ui.label(self.status_line());
            ui.separator();

            ui.add(egui::Slider::new(&mut self.demo_rating, 0..=Rating::MAX).text("stars"));
            let rated = Rating::new(self.demo_rating)
                .map(|r| r.stars())
                .unwrap_or(0);
            ui.label(format!("Selected rating: {rated} star(s)"));

            ui.separator();
            ui.label("Viewer, albums and maintenance arrive in milestones M3–M5.");
            ui.label("See docs/roadmap.md.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_includes_version_and_range() {
        let app = PinacotecaApp::default();
        let line = app.status_line();
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
        assert!(line.contains(&Rating::MAX.to_string()));
    }
}
