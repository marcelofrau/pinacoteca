//! The Picasa look: light chrome, white thumbnail cards, blue selection.
//!
//! The colours come from the constants recovered during the M1 analysis
//! (`docs/analysis/albums/albums-spec.md`), which read them out of Picasa's own
//! `runtime/constants.ui`:
//!
//! * highlight `#83A7BD`, selection `#25648B`
//! * album-list background `#F3F3F3`
//! * selected-thumbnail outline `#009EFF`
//! * album title: Georgia 20 pt, `#634B45`
//! * gutter 24

use eframe::egui;

/// Hover highlight (`constants.ui` `hicolor`).
pub const HIGHLIGHT: egui::Color32 = egui::Color32::from_rgb(0x83, 0xA7, 0xBD);
/// Selection fill (`constants.ui` `selcolor`).
pub const SELECTION: egui::Color32 = egui::Color32::from_rgb(0x25, 0x64, 0x8B);
/// The library panel background (`constants.ui` `alist bg`).
pub const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(0xF3, 0xF3, 0xF3);
/// The photo area behind the thumbnail cards.
pub const CANVAS_BG: egui::Color32 = egui::Color32::from_rgb(0xE6, 0xE6, 0xE6);
/// Body text.
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0x3C, 0x3C, 0x3C);
/// Secondary text, section headers.
#[allow(dead_code)]
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(0x77, 0x77, 0x77);
/// The white card a thumbnail sits on.
pub const CARD: egui::Color32 = egui::Color32::WHITE;
/// Card border.
pub const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(0xC8, 0xC8, 0xC8);
/// Outline around the selected thumbnail (`constants.ui` `sel thumb outline`).
pub const CARD_SELECTED: egui::Color32 = egui::Color32::from_rgb(0x00, 0x9E, 0xFF);
/// Album title colour.
#[allow(dead_code)]
pub const TITLE: egui::Color32 = egui::Color32::from_rgb(0x63, 0x4B, 0x45);
#[allow(dead_code)]
pub const GUTTER: f32 = 24.0;
/// Vertical gutter between cards.
#[allow(dead_code)]
pub const GUTTER_Y: f32 = 18.0;

/// Registers the title font, if the system has it.
///
/// Georgia is loaded from the machine, never bundled: it is a licensed font and
/// is only used for the album title, exactly as Picasa does.
const GEORGIA_PATHS: [&str; 4] = [
    r"C:\Windows\Fonts\georgia.ttf",
    "/System/Library/Fonts/Supplemental/Georgia.ttf",
    "/usr/share/fonts/truetype/msttcorefonts/Georgia.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
];

/// Installs the light Picasa-like theme.
pub fn apply(ctx: &egui::Context) {
    install_title_font(ctx);

    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = CANVAS_BG;
    visuals.window_fill = PANEL_BG;
    visuals.faint_bg_color = CANVAS_BG;
    visuals.extreme_bg_color = PANEL_BG;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = SELECTION;
    visuals.selection.stroke = egui::Stroke::new(1.5_f32, CARD_SELECTED);
    visuals.hyperlink_color = SELECTION;
    visuals.widgets.hovered.bg_fill = HIGHLIGHT;
    visuals.widgets.active.bg_fill = SELECTION;
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.bg_stroke.color = CARD_BORDER;
    visuals.widgets.inactive.bg_stroke.color = CARD_BORDER;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.hovered.fg_stroke.color = TEXT;
    visuals.widgets.active.fg_stroke.color = TEXT;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    ctx.set_style(style);
}

/// The `FontFamily` name for the album title face.
pub fn title_family() -> egui::FontFamily {
    egui::FontFamily::Name("georgia".into())
}

fn install_title_font(ctx: &egui::Context) {
    let Some(bytes) = GEORGIA_PATHS
        .iter()
        .find_map(|path| std::fs::read(path).ok())
    else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "georgia".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    fonts
        .families
        .insert(title_family(), vec!["georgia".to_owned()]);
    ctx.set_fonts(fonts);
}
