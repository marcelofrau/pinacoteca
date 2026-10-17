//! Application shell: folder scanning, thumbnail grid and the viewer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;

use pinacoteca_core::scan::DiscoveredPhoto;
use pinacoteca_db::albums::AlbumRow;
use pinacoteca_db::catalog::PhotoRow;
use pinacoteca_db::{Database, albums::Smart};
use pinacoteca_render::thumbs::{SourceFingerprint, ThumbKey, ThumbSize};

use crate::editor::Editor;
use crate::workers::{ScanHandle, ScanMessage, ThumbPool};

/// Size of a grid cell, in points.
const CELL: f32 = 168.0;
/// Gap between grid cells, in points.
const GAP: f32 = 8.0;
/// Maximum thumbnail requests queued per frame.
const REQUESTS_PER_FRAME: usize = 64;

/// One photo in the grid.
#[derive(Clone)]
struct Entry {
    path: PathBuf,
    name: String,
    key: ThumbKey,
    /// Stored rating, 0..=5.
    rating: u8,
}

/// The viewer overlay state.
struct Viewer {
    key: ThumbKey,
    zoom: f32,
    offset: egui::Vec2,
}

/// Which photos the grid is currently showing.
#[derive(Clone, PartialEq, Eq)]
enum View {
    /// Every photo found by the last scan.
    All,
    /// Photos inside one manual album.
    Album(i64),
    /// Photos selected by a saved query.
    Smart(Smart),
    /// Photos from one directory of the scanned tree.
    Folder(PathBuf),
}

/// Editable detail fields for one album.
#[derive(Debug, Clone)]
struct AlbumEdit {
    id: i64,
    description: String,
    place_taken: String,
    music_path: String,
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
    /// The open edit panel, if any.
    editor: Option<Editor>,
    status: String,
    /// Album list shown in the sidebar.
    albums: Vec<AlbumRow>,
    /// What the grid is showing.
    view: View,
    /// Name typed for a new album.
    new_album: String,
    /// Detail fields being edited for the selected album.
    album_edit: Option<AlbumEdit>,
    /// Photo currently being dragged from the grid, if any.
    dragging: Option<PathBuf>,
    /// Photos shown under the current [`View`].
    visible: Vec<Entry>,
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
        let albums = db
            .as_ref()
            .and_then(|db| db.list_albums().ok())
            .unwrap_or_default();
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
            editor: None,
            albums,
            view: View::All,
            visible: Vec::new(),
            new_album: String::new(),
            album_edit: None,
            dragging: None,
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

/// Turns an empty string into `None`, so clearing a field clears the column.
fn empty_as_none(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Builds a grid entry for a stored photo.
///
/// The file's fingerprint is re-read here instead of being persisted, so an
/// album entry gets exactly the same thumbnail key the scanner produced and
/// both views share one cache file per photo.
fn entry_for(row: &PhotoRow) -> Option<Entry> {
    let path = PathBuf::from(&row.path);
    let metadata = std::fs::metadata(&path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let fingerprint = SourceFingerprint {
        len: metadata.len(),
        modified_secs: modified.as_secs() as i64,
        modified_nanos: modified.subsec_nanos(),
    };
    Some(Entry {
        name: file_name(&path),
        key: ThumbKey {
            source: path.clone(),
            fingerprint,
            size: ThumbSize::Normal,
        },
        path,
        rating: row.rating,
    })
}

impl PinacotecaApp {
    /// Reloads the album list from the database.
    fn refresh_albums(&mut self) {
        let Some(db) = &self.db else { return };
        match db.list_albums() {
            Ok(albums) => self.albums = albums,
            Err(error) => self.status = format!("albums: {error}"),
        }
    }

    /// Rebuilds the photo list the grid shows from the current [`View`].
    fn rebuild_visible(&mut self) {
        self.selected = 0;
        match &self.view {
            View::All => self.visible = self.entries.clone(),
            View::Folder(dir) => {
                self.visible = self
                    .entries
                    .iter()
                    .filter(|entry| entry.path.parent() == Some(dir.as_path()))
                    .cloned()
                    .collect();
            }
            View::Album(id) => {
                let rows = self.db.as_ref().and_then(|db| db.album_photos(*id).ok());
                self.visible = rows
                    .unwrap_or_default()
                    .iter()
                    .filter_map(entry_for)
                    .collect();
            }
            View::Smart(smart) => {
                let rows = self.db.as_ref().and_then(|db| db.smart_photos(smart).ok());
                self.visible = rows
                    .unwrap_or_default()
                    .iter()
                    .filter_map(entry_for)
                    .collect();
            }
        }
    }

    /// Imports stars, captions and keywords from each folder's `.picasa.ini`.
    ///
    /// Read-only: the INI is never rewritten, so a library that is still
    /// managed by Picasa keeps working and unknown keys survive untouched.
    /// Album membership is deliberately not imported — Picasa stores opaque
    /// tokens there and resolves the names in its own obfuscated database.
    fn import_picasa_ini(&mut self) {
        let mut folders: Vec<PathBuf> = self
            .entries
            .iter()
            .filter_map(|entry| entry.path.parent().map(Path::to_path_buf))
            .collect();
        folders.sort();
        folders.dedup();

        let mut imported = 0usize;
        for folder in folders {
            let ini = match pinacoteca_core::picasa_ini::read(&folder.join(".picasa.ini")) {
                Ok(Some(ini)) => ini,
                _ => match pinacoteca_core::picasa_ini::read(&folder.join("Picasa.ini")) {
                    Ok(Some(ini)) => ini,
                    _ => continue,
                },
            };

            for photo in &ini.photos {
                let Some(db) = &self.db else { break };
                let path = folder.join(&photo.file).to_string_lossy().into_owned();
                if let Some(star) = photo.star {
                    let _ = db.set_photo_rating(&path, star);
                }
                if let Some(caption) = &photo.caption {
                    let _ = db.set_photo_caption(&path, caption);
                }
                for keyword in &photo.keywords {
                    if let Ok(tag_id) = db.ensure_tag(keyword) {
                        let _ = db.tag_photo(&path, tag_id);
                    }
                }
                imported += 1;
            }
        }

        if imported > 0 {
            self.status = format!("Imported metadata from .picasa.ini for {imported} photo(s)");
        }
        self.apply_stored_ratings();
    }

    /// Paints the stored ratings onto the scanned entries.
    fn apply_stored_ratings(&mut self) {
        let Some(ratings) = self.db.as_ref().and_then(|db| db.ratings_by_path().ok()) else {
            return;
        };
        for entry in &mut self.entries {
            let key = entry.path.to_string_lossy().into_owned();
            if let Some(rating) = ratings.get(&key) {
                entry.rating = *rating;
            }
        }
    }

    /// Rates the selected photo; 0 clears its star.
    fn rate_selected(&mut self, rating: u8) {
        let Some(selected) = self.visible.get(self.selected) else {
            self.status = "Select a photo first.".into();
            return;
        };
        let path = selected.path.clone();
        let name = selected.name.clone();

        if let Some(db) = &self.db
            && let Err(error) = db.set_photo_rating(&path.to_string_lossy(), rating)
        {
            self.status = format!("rating: {error}");
            return;
        }
        if let Some(entry) = self.visible.get_mut(self.selected) {
            entry.rating = rating;
        }
        for entry in &mut self.entries {
            if entry.path == path {
                entry.rating = rating;
            }
        }
        self.status = format!("{name}: {rating} star(s)");
    }

    /// Removes the selected photo from an album. Disk is never touched.
    fn remove_selected_from_album(&mut self, album_id: i64) {
        let Some(entry) = self.visible.get(self.selected) else {
            return;
        };
        let path = entry.path.to_string_lossy().into_owned();
        if let Some(db) = &self.db
            && let Err(error) = db.remove_photo(album_id, &path)
        {
            self.status = format!("album: {error}");
            return;
        }
        self.status = "Removed from album (file untouched).".into();
        self.refresh_albums();
        self.rebuild_visible();
    }

    /// Loads the editable detail fields for an album.
    fn load_album_edit(&mut self, album_id: i64) {
        let Some(album) = self.albums.iter().find(|album| album.id == album_id) else {
            self.album_edit = None;
            return;
        };
        self.album_edit = Some(AlbumEdit {
            id: album.id,
            description: album.description.clone().unwrap_or_default(),
            place_taken: album.place_taken.clone().unwrap_or_default(),
            music_path: album.music_path.clone().unwrap_or_default(),
        });
    }

    /// Persists the edited detail fields.
    fn save_album_edit(&mut self) {
        let Some(edit) = self.album_edit.clone() else {
            return;
        };
        let saved = self.db.as_ref().map(|db| {
            db.set_album_detail(
                edit.id,
                empty_as_none(&edit.description),
                empty_as_none(&edit.place_taken),
                empty_as_none(&edit.music_path),
                None,
            )
        });
        match saved {
            Some(Ok(())) => {
                self.refresh_albums();
                self.status = "Album details saved".into();
            }
            Some(Err(error)) => self.status = format!("album: {error}"),
            None => {}
        }
    }

    /// The album sidebar: all photos, smart albums and manual albums.
    fn sidebar_ui(&mut self, ctx: &egui::Context) {
        let mut pick: Option<View> = None;
        let mut create = false;
        let mut add_selected: Option<i64> = None;
        let mut delete: Option<i64> = None;
        let mut save = false;
        let mut drop_on: Option<i64> = None;
        let released = ctx.input(|i| i.pointer.any_released());

        egui::SidePanel::left("albums")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Albums");
                ui.separator();

                let all_label = format!("All photos ({})", self.entries.len());
                if ui
                    .selectable_label(self.view == View::All, all_label)
                    .clicked()
                {
                    pick = Some(View::All);
                }

                ui.add_space(8.0);
                ui.strong("Smart");
                for (smart, label) in [
                    (Smart::Recent { limit: 100 }, "Recently taken"),
                    (Smart::Starred { min_rating: 1 }, "Starred"),
                ] {
                    if ui
                        .selectable_label(self.view == View::Smart(smart), label)
                        .clicked()
                    {
                        pick = Some(View::Smart(smart));
                    }
                }

                ui.add_space(8.0);
                ui.strong("Folders");
                let mut folders: Vec<PathBuf> = self
                    .entries
                    .iter()
                    .filter_map(|entry| entry.path.parent().map(Path::to_path_buf))
                    .collect();
                folders.sort();
                folders.dedup();
                for folder in folders {
                    let label = folder
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| folder.to_string_lossy().into_owned());
                    if ui
                        .selectable_label(self.view == View::Folder(folder.clone()), label)
                        .on_hover_text(folder.to_string_lossy())
                        .clicked()
                    {
                        pick = Some(View::Folder(folder));
                    }
                }

                ui.add_space(8.0);
                ui.strong("Albums");
                for album in &self.albums {
                    ui.horizontal(|ui| {
                        let label = format!("{} ({})", album.name, album.photo_count);
                        let row = ui.selectable_label(self.view == View::Album(album.id), label);
                        if self.dragging.is_some() && row.hovered() && released {
                            drop_on = Some(album.id);
                        }
                        if row.clicked() {
                            pick = Some(View::Album(album.id));
                        }
                        if ui
                            .small_button("+")
                            .on_hover_text("Add selected photo")
                            .clicked()
                        {
                            add_selected = Some(album.id);
                        }
                        if ui.small_button("x").on_hover_text("Delete album").clicked() {
                            delete = Some(album.id);
                        }
                    });
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.new_album)
                            .hint_text("New album")
                            .desired_width(150.0),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create = true;
                    }
                    if ui.button("+").on_hover_text("Create album").clicked() {
                        create = true;
                    }
                });

                if let Some(edit) = &mut self.album_edit {
                    ui.add_space(10.0);
                    ui.strong("Album details");
                    ui.label("Description");
                    ui.text_edit_multiline(&mut edit.description);
                    ui.label("Place taken");
                    ui.text_edit_singleline(&mut edit.place_taken);
                    ui.label("Music");
                    ui.text_edit_singleline(&mut edit.music_path);
                    if ui.button("Save details").clicked() {
                        save = true;
                    }
                }
            });

        if let Some(view) = pick {
            match view {
                View::Album(id) => self.load_album_edit(id),
                _ => self.album_edit = None,
            }
            self.view = view;
            self.rebuild_visible();
        }
        if save {
            self.save_album_edit();
        }
        if let Some(album_id) = drop_on
            && let Some(path) = self.dragging.clone()
        {
            let path = path.to_string_lossy().into_owned();
            match self.db.as_ref().map(|db| db.add_photos(album_id, &[path])) {
                Some(Ok(_)) => {
                    self.status = "Added to album".into();
                    self.refresh_albums();
                    if self.view == View::Album(album_id) {
                        self.rebuild_visible();
                    }
                }
                Some(Err(error)) => self.status = format!("album: {error}"),
                None => {}
            }
        }
        if released {
            self.dragging = None;
        }
        if create {
            let name = self.new_album.trim().to_string();
            if !name.is_empty() {
                match self.db.as_ref().map(|db| db.create_album(&name)) {
                    Some(Ok(_)) => {
                        self.new_album.clear();
                        self.refresh_albums();
                        self.status = format!("album '{name}' created");
                    }
                    Some(Err(error)) => self.status = format!("album: {error}"),
                    None => {}
                }
            }
        }
        if let Some(album_id) = add_selected {
            let path = self
                .visible
                .get(self.selected)
                .map(|entry| entry.path.to_string_lossy().into_owned());
            let Some(path) = path else {
                self.status = "Select a photo first.".into();
                return;
            };
            match self.db.as_ref().map(|db| db.add_photos(album_id, &[path])) {
                Some(Ok(_)) => {
                    self.refresh_albums();
                    if self.view == View::Album(album_id) {
                        self.rebuild_visible();
                    }
                }
                Some(Err(error)) => self.status = format!("album: {error}"),
                None => {}
            }
        }
        if let Some(album_id) = delete {
            if let Some(db) = &self.db {
                let _ = db.delete_album(album_id);
            }
            if self.view == View::Album(album_id) {
                self.view = View::All;
            }
            self.refresh_albums();
            self.rebuild_visible();
        }
    }

    /// Prefills the library path and starts scanning it.
    ///
    /// Used by the command line, so a script can launch the app directly into a
    /// folder.
    pub fn set_folder(&mut self, folder: &str) {
        self.folder_input = folder.to_string();
        self.open_folder();
    }

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
                            rating: 0,
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
                self.import_picasa_ini();
                self.view = View::All;
                self.rebuild_visible();
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
        if self.visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No photos. Open a folder above.");
            });
            return;
        }

        let columns = columns_for(ui.available_width(), CELL, GAP);
        let count = self.visible.len();
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
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(CELL, CELL),
                                egui::Sense::click_and_drag(),
                            );
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
        let mut dragged = None;
        let mut reorder_to = None;

        for (index, rect, response) in cells {
            let key = self.visible[index].key.clone();
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

            let rating = self.visible[index].rating;
            if rating > 0 {
                let painter = ui.painter();
                let strip = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), rect.bottom() - 12.0),
                    egui::vec2(rect.width(), 12.0),
                );
                painter.rect_filled(
                    strip,
                    egui::CornerRadius::ZERO,
                    egui::Color32::from_black_alpha(140),
                );
                let star = egui::vec2(7.0, 7.0);
                let mut at = egui::pos2(rect.left() + 4.0, rect.bottom() - 9.5);
                for _ in 0..rating {
                    painter.rect_filled(
                        egui::Rect::from_min_size(at, star),
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_rgb(255, 196, 32),
                    );
                    at.x += 9.0;
                }
            }

            if response.clicked() {
                clicked = Some(index);
            }
            if response.double_clicked() {
                opened = Some(index);
            }
            if response.drag_started() {
                dragged = Some(index);
            }
            if self.dragging.is_some()
                && response.hovered()
                && ctx.input(|i| i.pointer.any_released())
            {
                reorder_to = Some(index);
            }
        }

        if let Some(index) = clicked {
            self.selected = index;
        }
        if let Some(index) = opened {
            self.selected = index;
            self.open_viewer(index);
        }
        if let Some(index) = dragged {
            self.selected = index;
            self.dragging = self.visible.get(index).map(|entry| entry.path.clone());
        }
        if let Some(target) = reorder_to {
            self.reorder_album_to(target);
        }
    }

    /// Moves the dragged photo to `target`'s slot inside the current album.
    fn reorder_album_to(&mut self, target: usize) {
        let View::Album(album_id) = self.view.clone() else {
            return;
        };
        let Some(source) = self.dragging.clone() else {
            return;
        };
        let mut order: Vec<String> = self
            .visible
            .iter()
            .map(|entry| entry.path.to_string_lossy().into_owned())
            .collect();
        let Some(from) = order.iter().position(|path| Path::new(path) == source) else {
            return;
        };
        let moved = order.remove(from);
        order.insert(target.min(order.len()), moved);
        if let Some(db) = &self.db
            && let Err(error) = db.reorder(album_id, &order)
        {
            self.status = format!("album: {error}");
            return;
        }
        self.status = "Album reordered".into();
        self.dragging = None;
        self.rebuild_visible();
    }

    /// Opens the full-resolution viewer for a grid index.
    fn open_viewer(&mut self, index: usize) {
        let Some(entry) = self.visible.get(index) else {
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

    /// Opens the edit panel over the selected photo.
    ///
    /// The preview base is decoded once at preview resolution; every later
    /// change replays the stack over those pixels, so the original file is only
    /// ever read.
    fn open_editor(&mut self) {
        let Some(entry) = self.visible.get(self.selected) else {
            return;
        };
        let path = entry.path.clone();
        let filename = entry.name.clone();

        let registry = pinacoteca_decoders::default_registry();
        let Some(decoder) = registry.decoder_for(&path) else {
            self.status = "no decoder for this file".into();
            return;
        };
        let base = match decoder.decode(&path) {
            Ok(image) => {
                pinacoteca_render::imageops::resize_to_fit(&image, ThumbSize::Preview.pixels())
            }
            Err(error) => {
                self.status = format!("edit: {error}");
                return;
            }
        };

        let stack = self
            .db
            .as_ref()
            .and_then(|db| db.load_edit_stack(&path.to_string_lossy()).ok())
            .unwrap_or_default();

        self.status = format!("editing {filename}");
        self.editor = Some(Editor::new(path, filename, base, stack));
    }

    /// Writes the open editor's stack to the catalog.
    fn save_editor(&mut self) {
        let Some(editor) = &self.editor else {
            return;
        };
        let path = editor.path.to_string_lossy().into_owned();
        let ops = editor.stack.len();
        if let Some(db) = &self.db
            && let Err(error) = db.save_edit_stack(&path, &editor.stack)
        {
            self.status = format!("edit: {error}");
            return;
        }
        self.status = format!("saved {ops} edit operation(s)");
    }

    /// Draws the editor's live preview in the central panel.
    fn editor_preview(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(
                ui.max_rect(),
                egui::CornerRadius::ZERO,
                egui::Color32::BLACK,
            );
            if let Some(editor) = &self.editor {
                if let Some(texture) = editor.texture() {
                    draw_fitted(ui.painter(), ui.max_rect(), texture);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("building preview…");
                    });
                }
            }
        });
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
        let (left, right, up, down, enter, escape, plus, minus, zero, space, rating, edit_key) =
            ctx.input(|i| {
                let rating = [
                    (egui::Key::Num0, 0),
                    (egui::Key::Num1, 1),
                    (egui::Key::Num2, 2),
                    (egui::Key::Num3, 3),
                    (egui::Key::Num4, 4),
                    (egui::Key::Num5, 5),
                ]
                .into_iter()
                .find(|(key, _)| i.key_pressed(*key))
                .map(|(_, value)| value);
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
                    rating,
                    i.key_pressed(egui::Key::E),
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

        if self.editor.is_some() {
            if escape {
                self.editor = None;
            }
            return;
        }

        if self.visible.is_empty() {
            return;
        }

        if edit_key {
            self.open_editor();
            return;
        }
        if let Some(rating) = rating {
            self.rate_selected(rating);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete))
            && let View::Album(album_id) = self.view.clone()
        {
            self.remove_selected_from_album(album_id);
        }
        let columns = columns_for(ctx.content_rect().width(), CELL, GAP);
        let last = self.visible.len() - 1;
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
                if ui.button("▶").clicked() && self.selected + 1 < self.visible.len() {
                    self.selected += 1;
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let selected = self
                    .visible
                    .get(self.selected)
                    .map(|entry| entry.name.as_str())
                    .unwrap_or("—");
                ui.label(format!("{}   |   {selected}", self.status));
                ui.separator();
                ui.label("0-5 stars   E edit   Enter open   Esc close   Del remove   drag: reorder / drop on album");
                if self.viewer.is_some() {
                    ui.separator();
                    ui.label("zoom  +/-   1:1  Space/Esc");
                }
            });
        });

        if self.viewer.is_some() {
            self.viewer_ui(ctx);
        } else if self.editor.is_some() {
            let mut save = false;
            let mut close = false;
            {
                let editor = self.editor.as_mut().expect("checked above");
                editor.refresh(ctx);
                egui::SidePanel::right("editor")
                    .default_width(280.0)
                    .show(ctx, |ui| {
                        editor.ui(ui);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                save = true;
                            }
                            if ui.button("Close").clicked() {
                                close = true;
                            }
                        });
                    });
            }
            self.editor_preview(ctx);
            if save {
                self.save_editor();
            }
            if close {
                self.editor = None;
            }
        } else {
            self.sidebar_ui(ctx);
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
    use pinacoteca_core::edit::{EditOp, Effect, Rotation};

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

    /// Builds an `Entry` for an already-written file.
    fn entry(path: &Path, rating: u8) -> Entry {
        let metadata = std::fs::metadata(path).expect("metadata");
        let modified = metadata
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch");
        Entry {
            path: path.to_path_buf(),
            name: file_name(path),
            key: ThumbKey {
                source: path.to_path_buf(),
                fingerprint: SourceFingerprint {
                    len: metadata.len(),
                    modified_secs: modified.as_secs() as i64,
                    modified_nanos: modified.subsec_nanos(),
                },
                size: ThumbSize::Normal,
            },
            rating,
        }
    }

    /// The album's stored order, as file names.
    fn album_order(app: &PinacotecaApp, album: i64) -> Vec<String> {
        app.db
            .as_ref()
            .expect("db")
            .album_photos(album)
            .expect("photos")
            .into_iter()
            .map(|row| {
                Path::new(&row.path)
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
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

    /// An album must reuse the scanner's thumbnail key, otherwise one photo on
    /// disk would be decoded twice and cached twice.
    #[test]
    fn album_entries_share_the_scanner_thumbnail_key() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-key-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("photo.jpg");
        std::fs::write(&path, b"not really a jpeg").expect("fixture");

        let metadata = std::fs::metadata(&path).expect("metadata");
        let modified = metadata
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch");
        let scanned = DiscoveredPhoto {
            path: path.clone(),
            len: metadata.len(),
            modified_secs: modified.as_secs() as i64,
            modified_nanos: modified.subsec_nanos(),
        };

        let row = PhotoRow {
            id: 1,
            path: path.to_string_lossy().into_owned(),
            folder_id: None,
            file_size: Some(scanned.len as i64),
            modified_secs: Some(scanned.modified_secs),
            width: None,
            height: None,
            orientation: 1,
            date_taken: None,
            format: Some("jpg".into()),
            caption: None,
            hidden: false,
            rating: 3,
        };

        let entry = entry_for(&row).expect("entry");
        assert_eq!(entry.key, key_for(&scanned, ThumbSize::Normal));
        assert_eq!(entry.rating, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The import must persist the user state and must never rewrite the INI,
    /// so a library still managed by Picasa keeps working.
    #[test]
    fn picasa_ini_import_restores_stars_and_keywords() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-ini-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let photo = dir.join("IMG_0001.JPG");
        std::fs::write(&photo, b"not really a jpeg").expect("photo");
        let ini_text =
            "[Picasa]\r\nname=Trip\r\n\r\n[IMG_0001.JPG]\r\nstar=4\r\nkeywords=family,beach\r\n";
        std::fs::write(dir.join(".picasa.ini"), ini_text).expect("ini");

        let db = Database::open_in_memory().expect("db");
        db.upsert_folder("root").expect("folder");
        db.upsert_photo_facts(&photo.to_string_lossy(), Some(1), 1, 1, Some("jpg"))
            .expect("facts");

        let metadata = std::fs::metadata(&photo).expect("metadata");
        let modified = metadata
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch");
        let key = ThumbKey {
            source: photo.clone(),
            fingerprint: SourceFingerprint {
                len: metadata.len(),
                modified_secs: modified.as_secs() as i64,
                modified_nanos: modified.subsec_nanos(),
            },
            size: ThumbSize::Normal,
        };

        let mut app = PinacotecaApp {
            db: Some(db),
            entries: vec![Entry {
                path: photo.clone(),
                name: file_name(&photo),
                key,
                rating: 0,
            }],
            ..PinacotecaApp::default()
        };
        app.import_picasa_ini();

        let db = app.db.as_ref().expect("db");
        let stored = db
            .photo_by_path(&photo.to_string_lossy())
            .expect("row")
            .expect("photo");
        assert_eq!(stored.rating, 4);
        assert_eq!(app.entries[0].rating, 4);

        let mut names: Vec<String> = db
            .list_tags()
            .expect("tags")
            .into_iter()
            .map(|t| t.name)
            .collect();
        names.sort();
        assert_eq!(names, ["beach", "family"]);

        // Read-only: the file on disk is byte-for-byte unchanged.
        assert_eq!(
            std::fs::read_to_string(dir.join(".picasa.ini")).expect("ini"),
            ini_text
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Drag-reorder inside an album: the dragged photo lands on the target slot
    /// and every other photo keeps its relative order.
    #[test]
    fn dragging_inside_an_album_moves_that_photo_to_the_slot() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-reorder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let db = Database::open_in_memory().expect("db");
        db.upsert_folder("root").expect("folder");
        let album = db.create_album("Trip").expect("album");

        let mut entries = Vec::new();
        let mut paths = Vec::new();
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            let path = dir.join(name);
            std::fs::write(&path, name).expect("photo");
            db.upsert_photo_facts(&path.to_string_lossy(), Some(1), 1, 1, Some("jpg"))
                .expect("facts");
            paths.push(path.to_string_lossy().into_owned());
            entries.push(entry(&path, 0));
        }
        db.add_photos(album, &paths).expect("add");

        let mut app = PinacotecaApp {
            db: Some(db),
            entries: entries.clone(),
            view: View::Album(album),
            visible: entries.clone(),
            ..PinacotecaApp::default()
        };
        app.rebuild_visible();
        assert_eq!(album_order(&app, album), ["a.jpg", "b.jpg", "c.jpg"]);

        // Drag c.jpg onto the first slot.
        app.dragging = Some(dir.join("c.jpg"));
        app.reorder_album_to(0);

        assert_eq!(album_order(&app, album), ["c.jpg", "a.jpg", "b.jpg"]);
        assert!(app.dragging.is_none(), "the drag ends once it is dropped");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The whole point of the stack: editing must not touch the file.
    #[test]
    fn editing_never_rewrites_the_original_file() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-edit-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        // A real JPEG, so the editor's decode path runs for real.
        let pixels: Vec<u8> = (0..8 * 8)
            .flat_map(|i| [(i * 3) as u8, 40, 200, 255])
            .collect();
        let source = pinacoteca_decoders::RgbaImage::new(8, 8, pixels).expect("source");
        let mut encoded = Vec::new();
        jpeg_encoder::Encoder::new(&mut encoded, 90)
            .encode(&source.pixels, 8, 8, jpeg_encoder::ColorType::Rgba)
            .expect("encode");

        let photo = dir.join("IMG_0007.JPG");
        std::fs::write(&photo, &encoded).expect("write");
        let before = std::fs::read(&photo).expect("read back");

        let db = Database::open_in_memory().expect("db");
        db.upsert_folder("C:/pics").expect("folder");
        db.upsert_photo_facts(
            &photo.to_string_lossy(),
            Some(1),
            encoded.len() as i64,
            1,
            Some("jpg"),
        )
        .expect("facts");

        let metadata = std::fs::metadata(&photo).expect("metadata");
        let modified = metadata
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("epoch");
        let entry = Entry {
            path: photo.clone(),
            name: file_name(&photo),
            key: ThumbKey {
                source: photo.clone(),
                fingerprint: SourceFingerprint {
                    len: metadata.len(),
                    modified_secs: modified.as_secs() as i64,
                    modified_nanos: modified.subsec_nanos(),
                },
                size: ThumbSize::Normal,
            },
            rating: 0,
        };

        let mut app = PinacotecaApp {
            db: Some(db),
            entries: vec![entry.clone()],
            visible: vec![entry],
            ..PinacotecaApp::default()
        };

        app.open_editor();
        let editor = app.editor.as_mut().expect("editor opened");
        assert!(editor.stack.is_empty(), "a fresh photo has no edits");

        editor.stack.push(EditOp::Rotate(Rotation::Cw90));
        editor.stack.push(EditOp::Brightness(30));
        editor.stack.set(EditOp::Effect(Effect::Sepia));
        let touched = editor.stack.clone();

        app.save_editor();

        // The file on disk is byte-identical.
        let after = std::fs::read(&photo).expect("read again");
        assert_eq!(before, after, "editing must not rewrite the original");

        // And the stack round-trips through the catalog.
        let stored = app
            .db
            .as_ref()
            .expect("db")
            .load_edit_stack(&photo.to_string_lossy())
            .expect("load");
        assert_eq!(stored, touched);
        assert_eq!(touched.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_folder_prefills_the_box_and_starts_a_scan() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-setfolder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let mut app = PinacotecaApp::default();
        app.set_folder(&dir.to_string_lossy());

        assert_eq!(app.folder_input, dir.to_string_lossy());
        assert!(app.scan.is_some(), "set_folder starts the scan");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_folder_refuses_a_path_that_is_not_a_folder() {
        let mut app = PinacotecaApp::default();
        app.set_folder("C:/definitely/not/here");

        assert!(app.scan.is_none(), "no scan starts for a missing folder");
        assert!(app.status.contains("Not a folder"), "{}", app.status);
    }
}
