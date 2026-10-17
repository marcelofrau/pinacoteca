//! Application shell: folder scanning, thumbnail grid and the viewer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;

use pinacoteca_core::scan::DiscoveredPhoto;
use pinacoteca_db::Database;
use pinacoteca_render::thumbs::{SourceFingerprint, ThumbKey, ThumbSize};

use crate::workers::{ScanHandle, ScanMessage, ThumbPool};

/// Size of a grid cell, in points.
const CELL: f32 = 168.0;
/// Gap between grid cells, in points.
const GAP: f32 = 8.0;
/// Maximum thumbnail requests queued per frame.
const REQUESTS_PER_FRAME: usize = 64;

/// One photo in the grid.
struct Entry {
    path: PathBuf,
    name: String,
    key: ThumbKey,
}

/// The viewer overlay state.
struct Viewer {
    key: ThumbKey,
    zoom: f32,
    offset: egui::Vec2,
}

/// Top-level application state.
pub struct PinacotecaApp {
    db: Option<Database>,
    folder_input: String,
    scan: Option<ScanHandle>,
    scan_generation: u64,
    scan_root: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    pool: Option<ThumbPool>,
    textures: HashMap<String, egui::TextureHandle>,
    viewer: Option<Viewer>,
    status: String,
}

impl Default for PinacotecaApp {
    fn default() -> Self {
        let mut status = String::from("Type a folder path and press Open.");
        let db = match open_database() {
            Ok(db) => Some(db),
            Err(error) => {
                status = format!("database unavailable: {error}");
                None
            }
        };
        Self {
            db,
            folder_input: std::env::var("PINACOTECA_FOLDER").unwrap_or_default(),
            scan: None,
            scan_generation: 0,
            scan_root: PathBuf::new(),
            entries: Vec::new(),
            selected: 0,
            pool: None,
            textures: HashMap::new(),
            viewer: None,
            status,
        }
    }
}

/// Suggests the number of grid columns that fit in `width` points.
fn columns_for(width: f32, cell: f32, gap: f32) -> usize {
    (((width + gap) / (cell + gap)).floor() as usize).max(1)
}

/// The application data directory (`%LOCALAPPDATA%\Pinacoteca` on Windows).
fn data_dir() -> PathBuf {
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(base).join("Pinacoteca");
    }
    std::env::temp_dir().join("Pinacoteca")
}

fn open_database() -> Result<Database, pinacoteca_db::DbError> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| pinacoteca_db::DbError::Migration(e.to_string()))?;
    Database::open(dir.join("pinacoteca.db"))
}

/// Builds a thumbnail key for a discovered photo.
fn key_for(photo: &DiscoveredPhoto, size: ThumbSize) -> ThumbKey {
    ThumbKey {
        source: photo.path.clone(),
        fingerprint: SourceFingerprint {
            len: photo.len,
            modified_secs: photo.modified_secs,
            modified_nanos: photo.modified_nanos,
        },
        size,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

impl PinacotecaApp {
    /// Starts scanning the folder typed in the input field.
    fn open_folder(&mut self) {
        let root = PathBuf::from(self.folder_input.trim());
        if root.as_os_str().is_empty() {
            self.status = "Enter a folder path first.".into();
            return;
        }
        if !root.is_dir() {
            self.status = format!("Not a folder: {}", root.display());
            return;
        }

        // A new generation invalidates any in-flight scan and stale thumbnails.
        self.scan_generation += 1;
        self.scan_root = root.clone();
        self.entries.clear();
        self.textures.clear();
        self.selected = 0;
        self.viewer = None;
        self.ensure_pool();
        self.scan = Some(ScanHandle::start(root, self.scan_generation));
        self.status = "Scanning…".into();
    }

    fn ensure_pool(&mut self) {
        if self.pool.is_none() {
            let threads = std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).clamp(1, 8))
                .unwrap_or(2);
            let cache = pinacoteca_render::thumbs::ThumbCache::new(data_dir().join("cache"));
            self.pool = Some(ThumbPool::new(cache, threads));
        }
    }

    /// Drains scan and thumbnail results into the UI state.
    fn poll_workers(&mut self) {
        if let Some(scan) = &self.scan {
            let generation = scan.generation();
            let mut finished = false;
            for message in scan.drain() {
                if generation != self.scan_generation {
                    break;
                }
                match message {
                    ScanMessage::Photo(photo) => {
                        if let Some(db) = &self.db {
                            let root = self.scan_root.to_string_lossy().into_owned();
                            let folder = db.upsert_folder(&root).ok();
                            let _ = db.upsert_photo_facts(
                                &photo.path.to_string_lossy(),
                                folder,
                                photo.len as i64,
                                photo.modified_secs,
                                photo.path.extension().and_then(|e| e.to_str()),
                            );
                        }
                        self.entries.push(Entry {
                            name: file_name(&photo.path),
                            path: photo.path.clone(),
                            key: key_for(&photo, ThumbSize::Normal),
                        });
                    }
                    ScanMessage::Done { photos, errors } => {
                        self.status = format!("{photos} photos ({errors} errors)");
                        finished = true;
                    }
                }
            }
            if finished {
                self.scan = None;
            }
        }

        if let Some(pool) = &mut self.pool {
            for _ in pool.drain() {
                // Texture upload happens lazily in `ensure_texture`.
            }
        }
    }

    /// Makes sure a texture exists for `key`, loading it from the cache or
    /// queueing generation. `budget` caps how many new requests may be queued.
    fn ensure_texture(&mut self, ctx: &egui::Context, key: &ThumbKey, budget: &mut usize) {
        let name = texture_name(key);
        if self.textures.contains_key(&name) {
            return;
        }
        let Some(pool) = &mut self.pool else {
            return;
        };
        if let Ok(Some(bytes)) = pool.cache().read(key)
            && let Some(texture) = upload(ctx, &name, &bytes)
        {
            self.textures.insert(name, texture);
            return;
        }
        if *budget > 0 {
            pool.request(key.clone());
            *budget -= 1;
        }
    }
}

/// A stable texture name for a thumbnail key.
fn texture_name(key: &ThumbKey) -> String {
    format!("{}:{}", key.size.dir_name(), key.source.to_string_lossy())
}

/// Decodes JPEG thumbnail bytes and uploads them as an egui texture.
fn upload(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let image = pinacoteca_decoders::jpeg::decode_bytes(bytes).ok()?;
    let color = egui::ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.pixels,
    );
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

const CELL_BG: egui::Color32 = egui::Color32::from_rgb(40, 40, 44);
const SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(37, 100, 139);

impl PinacotecaApp {
    /// Draws the thumbnail grid.
    fn grid_ui(&mut self, ui: &mut egui::Ui) {
        if self.entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No photos. Open a folder above.");
            });
            return;
        }

        let columns = columns_for(ui.available_width(), CELL, GAP);
        let count = self.entries.len();
        let mut cells: Vec<(usize, egui::Rect, egui::Response)> = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut index = 0;
                while index < count {
                    ui.horizontal(|ui| {
                        for _ in 0..columns {
                            if index >= count {
                                break;
                            }
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                            cells.push((index, rect, response));
                            index += 1;
                        }
                    });
                }
            });

        let ctx = ui.ctx().clone();
        let mut budget = REQUESTS_PER_FRAME;
        let mut clicked = None;
        let mut opened = None;

        for (index, rect, response) in cells {
            let key = self.entries[index].key.clone();
            self.ensure_texture(&ctx, &key, &mut budget);

            let background = if index == self.selected {
                SELECTED_BG
            } else {
                CELL_BG
            };
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, background);

            if let Some(texture) = self.textures.get(&texture_name(&key)) {
                draw_fitted(ui.painter(), rect, texture);
            }

            if response.clicked() {
                clicked = Some(index);
            }
            if response.double_clicked() {
                opened = Some(index);
            }
        }

        if let Some(index) = clicked {
            self.selected = index;
        }
        if let Some(index) = opened {
            self.selected = index;
            self.open_viewer(index);
        }
    }

    /// Opens the full-resolution viewer for a grid index.
    fn open_viewer(&mut self, index: usize) {
        let Some(entry) = self.entries.get(index) else {
            return;
        };
        let key = ThumbKey {
            source: entry.path.clone(),
            fingerprint: entry.key.fingerprint,
            size: ThumbSize::Preview,
        };
        self.viewer = Some(Viewer {
            key: key.clone(),
            zoom: 1.0,
            offset: egui::Vec2::ZERO,
        });
        self.ensure_pool();
        if let Some(pool) = &mut self.pool {
            pool.request(key);
        }
    }

    /// Draws the fullscreen viewer overlay.
    fn viewer_ui(&mut self, ctx: &egui::Context) {
        let Some(viewer) = self.viewer.as_ref() else {
            return;
        };
        let key = viewer.key.clone();
        let mut zoom = viewer.zoom;
        let mut offset = viewer.offset;

        let mut budget = REQUESTS_PER_FRAME;
        self.ensure_texture(ctx, &key, &mut budget);

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::BLACK);

            let response = ui.interact(
                rect,
                egui::Id::new("pinacoteca-viewer"),
                egui::Sense::drag(),
            );
            if response.dragged() {
                offset += response.drag_delta();
            }
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                zoom = (zoom * (1.0 + scroll * 0.0015)).clamp(0.05, 8.0);
            }

            if let Some(texture) = self.textures.get(&texture_name(&key)) {
                let available = rect.size();
                let size = texture.size_vec2();
                let fit = (available.x / size.x).min(available.y / size.y);
                let scale = fit * zoom;
                let draw = egui::vec2(size.x * scale, size.y * scale);
                let center = rect.center() + offset;
                let target = egui::Rect::from_center_size(center, draw);
                ui.painter().image(
                    texture.id(),
                    target,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "loading…",
                    egui::FontId::proportional(18.0),
                    egui::Color32::GRAY,
                );
            }
        });

        if let Some(viewer) = self.viewer.as_mut() {
            viewer.zoom = zoom;
            viewer.offset = offset;
        }
    }

    /// Handles keyboard navigation.
    fn keyboard(&mut self, ctx: &egui::Context) {
        let (left, right, up, down, enter, escape, plus, minus, zero, space) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                i.key_pressed(egui::Key::Minus),
                i.key_pressed(egui::Key::Num1),
                i.key_pressed(egui::Key::Space),
            )
        });

        if self.viewer.is_some() {
            if let Some(viewer) = self.viewer.as_mut() {
                if plus {
                    viewer.zoom = (viewer.zoom * 1.25).clamp(0.05, 8.0);
                }
                if minus {
                    viewer.zoom = (viewer.zoom / 1.25).clamp(0.05, 8.0);
                }
                if zero || space {
                    viewer.zoom = 1.0;
                    viewer.offset = egui::Vec2::ZERO;
                }
            }
            if escape {
                self.viewer = None;
            }
            return;
        }

        if self.entries.is_empty() {
            return;
        }
        let columns = columns_for(ctx.content_rect().width(), CELL, GAP);
        let last = self.entries.len() - 1;
        let mut selected = self.selected;

        if left {
            selected = selected.saturating_sub(1);
        }
        if right {
            selected = (selected + 1).min(last);
        }
        if up {
            selected = selected.saturating_sub(columns);
        }
        if down {
            selected = (selected + columns).min(last);
        }
        self.selected = selected;

        if enter {
            self.open_viewer(self.selected);
        }
    }
}

/// Draws a texture centred and fitted inside `rect`, preserving aspect ratio.
fn draw_fitted(painter: &egui::Painter, rect: egui::Rect, texture: &egui::TextureHandle) {
    let size = texture.size_vec2();
    let scale = (rect.width() / size.x).min(rect.height() / size.y);
    let draw = egui::vec2(size.x * scale, size.y * scale);
    let target = egui::Rect::from_center_size(rect.center(), draw);
    painter.image(
        texture.id(),
        target,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

impl eframe::App for PinacotecaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        self.poll_workers();

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Folder:");
                let response =
                    ui.add(egui::TextEdit::singleline(&mut self.folder_input).desired_width(420.0));
                let submitted =
                    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Open").clicked() || submitted {
                    self.open_folder();
                }
                if ui.button("◀").clicked() && self.selected > 0 {
                    self.selected -= 1;
                }
                if ui.button("▶").clicked() && self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let selected = self
                    .entries
                    .get(self.selected)
                    .map(|entry| entry.name.as_str())
                    .unwrap_or("—");
                ui.label(format!("{}   |   {selected}", self.status));
                if self.viewer.is_some() {
                    ui.separator();
                    ui.label("zoom  +/-   1:1  Space/Esc");
                }
            });
        });

        if self.viewer.is_some() {
            self.viewer_ui(ctx);
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.grid_ui(ui);
            });
        }

        self.keyboard(ctx);

        let busy =
            self.scan.is_some() || self.pool.as_ref().is_some_and(|pool| pool.inflight() > 0);
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_fit_available_width() {
        assert_eq!(columns_for(1000.0, 168.0, 8.0), 5);
        assert_eq!(columns_for(100.0, 168.0, 8.0), 1);
    }

    #[test]
    fn default_app_starts_without_entries() {
        let app = PinacotecaApp::default();
        assert!(app.entries.is_empty());
        assert!(app.viewer.is_none());
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn key_for_uses_photo_metadata() {
        let photo = DiscoveredPhoto {
            path: PathBuf::from("C:/photos/a.jpg"),
            len: 1234,
            modified_secs: 42,
            modified_nanos: 7,
        };
        let key = key_for(&photo, ThumbSize::Normal);
        assert_eq!(key.size, ThumbSize::Normal);
        assert_eq!(key.fingerprint.len, 1234);
        assert_eq!(key.fingerprint.modified_secs, 42);
    }

    #[test]
    fn texture_names_are_unique_per_size() {
        let path = PathBuf::from("C:/photos/a.jpg");
        let fingerprint = SourceFingerprint {
            len: 1,
            modified_secs: 1,
            modified_nanos: 0,
        };
        let normal = ThumbKey {
            source: path.clone(),
            fingerprint,
            size: ThumbSize::Normal,
        };
        let preview = ThumbKey {
            source: path,
            fingerprint,
            size: ThumbSize::Preview,
        };
        assert_ne!(texture_name(&normal), texture_name(&preview));
    }
}
