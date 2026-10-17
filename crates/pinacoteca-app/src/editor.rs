//! The edit panel: presets, sliders and a live preview.
//!
//! The preview is the base image at thumbnail resolution with the whole stack
//! replayed over it, so dragging a slider shows the real result of the same code
//! path that will render the saved stack.

use std::path::PathBuf;

use eframe::egui;
use pinacoteca_core::edit::{EditOp, EditStack, Effect, Rotation};
use pinacoteca_decoders::RgbaImage;
use pinacoteca_render::edits;

/// Editing state for one photo.
pub struct Editor {
    /// The file being edited; only its relative identity matters here.
    pub path: PathBuf,
    /// Display name for the panel header.
    pub filename: String,
    /// The unedited preview pixels, at thumbnail resolution.
    base: RgbaImage,
    /// The stack being built.
    pub stack: EditStack,
    texture: Option<egui::TextureHandle>,
    stale: bool,
}

impl Editor {
    /// Opens an editor over an already decoded preview image.
    pub fn new(path: PathBuf, filename: String, base: RgbaImage, stack: EditStack) -> Self {
        Self {
            path,
            filename,
            base,
            stack,
            texture: None,
            stale: true,
        }
    }

    /// The current preview texture, if one has been built.
    pub fn texture(&self) -> Option<&egui::TextureHandle> {
        self.texture.as_ref()
    }

    /// Rebuilds the preview when the stack changed. Cheap to call every frame.
    pub fn refresh(&mut self, ctx: &egui::Context) {
        if !self.stale {
            return;
        }
        let edited = edits::apply(self.base.clone(), &self.stack);
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [edited.width as usize, edited.height as usize],
            &edited.pixels,
        );
        self.texture = Some(ctx.load_texture("edit-preview", image, egui::TextureOptions::LINEAR));
        self.stale = false;
    }

    /// Marks the preview as needing a rebuild.
    pub fn touch(&mut self) {
        self.stale = true;
    }

    /// Draws the panel body.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong(&self.filename);
        ui.separator();

        ui.label("Rotate");
        ui.horizontal(|ui| {
            if ui.button("Left").clicked() {
                self.stack.push(EditOp::Rotate(Rotation::Cw270));
                self.touch();
            }
            if ui.button("Right").clicked() {
                self.stack.push(EditOp::Rotate(Rotation::Cw90));
                self.touch();
            }
        });

        ui.add_space(6.0);
        ui.label("Tuning");
        self.tone_slider(
            ui,
            "Brightness",
            -100,
            |op| match op {
                EditOp::Brightness(v) => Some(*v),
                _ => None,
            },
            EditOp::Brightness,
        );
        self.tone_slider(
            ui,
            "Contrast",
            -100,
            |op| match op {
                EditOp::Contrast(v) => Some(*v),
                _ => None,
            },
            EditOp::Contrast,
        );
        self.tone_slider(
            ui,
            "Saturation",
            -100,
            |op| match op {
                EditOp::Saturation(v) => Some(*v),
                _ => None,
            },
            EditOp::Saturation,
        );
        self.tone_slider(
            ui,
            "Warmth",
            -100,
            |op| match op {
                EditOp::Warmth(v) => Some(*v),
                _ => None,
            },
            EditOp::Warmth,
        );

        ui.add_space(6.0);
        ui.label("One click");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Auto fix").clicked() {
                self.stack.set(EditOp::AutoFix);
                self.touch();
            }
            if ui.button("B&W").clicked() {
                self.stack.set(EditOp::Effect(Effect::Grayscale));
                self.touch();
            }
            if ui.button("Sepia").clicked() {
                self.stack.set(EditOp::Effect(Effect::Sepia));
                self.touch();
            }
        });

        ui.add_space(6.0);
        let mut sharpen = current(&self.stack, |op| match op {
            EditOp::Sharpen(v) => Some(*v as i32),
            _ => None,
        });
        if ui
            .add(egui::Slider::new(&mut sharpen, 0..=20).text("Sharpen"))
            .changed()
        {
            self.stack.set(EditOp::Sharpen(sharpen as u8));
            self.touch();
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Undo").clicked() {
                self.stack.undo();
                self.touch();
            }
            if ui.button("Reset").clicked() {
                self.stack = EditStack::new();
                self.touch();
            }
        });
    }

    /// One `-100..=100` slider bound to a tone operation.
    fn tone_slider(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        minimum: i32,
        read: fn(&EditOp) -> Option<i32>,
        build: fn(i32) -> EditOp,
    ) {
        let mut value = current(&self.stack, read);
        if ui
            .add(egui::Slider::new(&mut value, minimum..=100).text(label))
            .changed()
        {
            self.stack.set(build(value));
            self.touch();
        }
    }
}

/// The value of the first operation of the requested kind, or zero.
fn current(stack: &EditStack, read: fn(&EditOp) -> Option<i32>) -> i32 {
    stack.ops().iter().find_map(read).unwrap_or(0)
}
