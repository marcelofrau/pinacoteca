## M1 status (end of pass 1)
- Resource-driven + string-located: PhotoViewer zoomtext at 0x726b90 with xrefs; viewer UI commands identified.
- Draft specs created: viewer-spec-draft.md, albums-thumbnails-draft.md, maintenance-draft.md, pmp-notes.md.
- M1 checklist updated (structure done, resources done). Need deeper REA: trace xrefs to handlers, collect more string->xref for key actions (fullscreen, filmstrip, slideshow, rotate/crop/enhance).
- Plan next: PhotoViewer - open binary, trace xrefs from key strings to see handler names/structure; Picasa3 for maintenance/albums.

Overall M1 in progress. Draft specs sufficient for initial review; deeper behavioral extraction needed for clean-room impl.

## M1 status (end of pass 2 — PhotoViewer deep REA)
- Opened PicasaPhotoViewer.exe in Ghidra via REA. Confirmed PE identity: internal name GooglePhotoViewer.exe, comctl32 v6, PDB present but not admitted.
- RTTI haul: FilmstripHandler (FSHEntry, CacheIndex), C3DFilmstrip (+ClickHandler), SlideshowTransitionHandler, ZoomStatus@GrowHandler, ytArray wiring. Filmstrip enumerates directory at runtime (FindFirstFile pattern) -> navigation order = directory enumeration order.
- Recovered full viewer command set from dispatcher decompile + string xrefs: ss/next, ss/prev, ss/zoom_plus, ss/zoom_minus, ss/actualsize (zoom=1.0 animated), ss/rotate_cw, ss/rotate_ccw, ss/fullscreen, ss/maximize, ss/play_slideshow, ss/star (toggle + 0.6->1.0 anim), ss/launch_picasa (locate), ss/to_web, ss/config, ss/more, ss/exit, ss/close.
- UI events: filmstripmove, 3dfilmstrip_clicked, viewermousewheelzoom, slingshot_*, contextmenu, lb_*popup.
- viewer-spec-draft.md rewritten with command table + zoom/filmstrip models + pass-3 candidates (keyboard map, slideshow timing, pan drag).
- Pass 3 candidates logged; REA session closed.

M1 remains in progress; pass 3 (keyboard/slideshow timing) optional before M2 gate.

## M1 status (pass 3 — keyboard/slideshow/pan)
- Keyboard mapping: static investigation concluded NOT statically enumerable. No populated accelerator resource (dummy accel table at main); every `ss/*` string has a single xref from the command dispatcher; window creation goes through the Fen framework `CreateWindowExW` wrapper `FUN_004d23a0`, so `WM_KEYDOWN` handling lives in Fen's window class. Documented in viewer-spec-draft.md with a runtime-probe plan (inject keys, log dispatcher) and a hypothesized 1:1 key->`ss/*` binding set to verify.
- Fen framework identity confirmed (Win32WindowImp-style window wrapper). PDB referenced but not admitted, so original symbol names unavailable.
- Next: slideshow timing + pan drag (string/xref), then Picasa3.exe albums/grid/thumbnails + maintenance.
- Slideshow: transition-driven (`SlideshowTransitionHandler` + `ytTransitionNode`/`CTransitions`); no literal interval string -> interval is config/DB-driven (default to define in Pinacoteca). Documented.
- Pan-when-zoomed: no dedicated command string; drag-based (`selectiondrag`/`windrag`/`mousedown`) -> runtime-probe candidate. Documented.
- UI framework likened: viewer uses in-house Fen/yt framework (Fen::TabPanel/MultiPanel/Win32TabPanelImp, ytPanel/ytBasePanel, ytSelectionDragHandler, HSliderFastDrag, transition nodes, Win32WindowImp). Added "UI framework & infrastructure" section to viewer-spec-draft.md.
- Pass 3 viewer spec now covers: command set, UI events, filmstrip, zoom, keyboard (static-negative + runtime plan), slideshow, pan, framework. Viewer spec pass 3 CLOSED.

## M1 status (pass 4 — Picasa3 maintenance/effects catalog)
- Picasa3.exe (10 MB) cannot complete REA Ghidra full auto-analysis within the 330s startup deadline; each Ghidra op re-imports ephemerally. Rizin unavailable on Windows native. Pivoted to raw ASCII corpus extraction via PowerShell (84,838 strings, Latin-1 `[\x20-\x7E]{4,}`).
- Recovered maintenance surface from strings + RTTI: `ID_PICTURE_*` menu command IDs, `IImageFilter`/`CImageFilterFactory`/`CFilterStack`/`EditStack`/`FilterUndoRedoShow`, `RedEyeEdit`/`RetouchEdit`/`PatchEdit`/`CSmartCrop`/`CollagePanel`, and the `glimmer::` effect engine (`CropImageOperation`, `SharpenImageOperation`, `ColorMatrixImageOperation`, EffectParser effect VM instructions).
- Identified the shipped source of truth for the maintenance catalog: `runtime/filterdesc.xml` (63 KB) — defines every filter/effect with id, mode, zerostate, label, tooltip, sliders, and optional Glimmer recipe.
- Extracted the FULL catalog: **84 filter entries** with mode (history/soft/oneclick/effect/tool), zerostate, label, tooltip, and numeric slider definitions (range/offset/log/default). Native parameterised filters (Sharpen, Soft Focus, Focal B&W, Glow, Saturation, Color Temperature, Shadow & Highlight, Contrast, Gamma, Blur, Backlight, Directional *, Focal Pixelate, etc.) plus 36 recipe-driven Glimmer creative effects (Boost…Vignette).
- Clean-room boundary set (PROJECT_RULES #1204): spec documents the CATALOG only; Glimmer effect recipe bodies are reference-only and must be re-derived.
- Wrote `maintenance/maintenance-spec.md` with the full catalog, mode/zerostate semantics, slider semantics, control types, edit-engine architecture, menu IDs, and domain-model mapping. `maintenance-draft.md` superseded.
- Remaining: albums/grid/thumbnails spec (resource + string survey), geometry sub-spec (crop/straighten/rotate), PMP/DB schema.

M1 in progress; maintenance spec pass 4 CLOSED.

## M1 status (pass 5 — Picasa3 albums/grid/thumbnails)
- Recovered the library view model: full `ID_VIEW_*` set (Timeline, Folders, All, Watched, People, Places, Keywords, Search, Lightbox, Thumbnails small/normal/large, Picture, Properties, Slideshow, Edit, colour-managed, show-hidden, plus monitor display-profile toggles).
- Recovered the album command set and album model: `IAlbumStore`/`CAlbumList`/`CAlbumSelectionNode`/`CMultiAlbumNode`; album actions (New, EditCaptions, Delete, LocateOnDisk, Slideshow, SelectAll, Move/CopyToCollection, Combine, Make_Web, PrintContactSheet, RefreshThumbnails, SendTo_*, Labels).
- Album metadata fields from `album.fen`: Name, Date (+Automatic date), Music (slideshow/movie), Place taken, Description.
- Special albums from `CThumbDB`: Starred, RecentUpdate, Emailed, Exported, Screensaver, Unknown/IgnoredPeople (faces scope-excluded).
- Star/rating model: `CThumbUI::sstarred`, `CStarredEnabler`, XMP::Rating, star icons.
- Thumbnail grid + layout constants from `constants.ui`: gutters, Georgia title/body fonts, selected-thumbnail outline colours.
- Thumbnail pipeline: `CThumbDB`/`IThumbDB`/`CThumbnailProvider`, EXIF-thumbnail reuse option, refresh progress UI.
- DB/store filenames recovered (thumbs.db, thumbs2.db, bigthumbs.db, previews.db, albums.db, profilephotos.db, index/thumbs_index/thumbindex, facetemplates, distims, `.picasa.ini`) — documented in formats/pmp-notes.md.
- Wrote `albums/albums-spec.md`; enriched `formats/pmp-notes.md`. `albums-thumbnails-draft.md` superseded.

M1 in progress; albums/grid/thumbnails spec pass 5 CLOSED.

## M1 status (pass 6 — geometry: crop/straighten/rotate)
- Crop stored as normalized `rect64` (four Q16 coords packed in 64 bits) via `crop=rect64(...)`; geometry record `width,height,crop,fr`.
- Crop UI recovered: selection, preview, apply/cancel/discard, aspect menu, rotate-then-crop, custom-ratio delete; guides `xhairs.png`, `bezier.txt`.
- Full aspect-ratio preset catalog + custom aspect dialog (`CAspectRatioList`, `CustomAspectRatioDlg`) under `Preferences\AspectRatios`.
- Smart crop engine `CSmartCrop` with suggestion strategies (horizon/variance/colour/faces/border) and `cropsug1..3` previews.
- Straighten = `tilt` tool, angle slider (`_sldrAngle`, `Angle: %d`, `radAngle`), auto-crop of empty corners (couples with `croprotatecrop`).
- Rotate = 90° steps (`ID_PICTURE_ROTATECLOCKWISE/COUNTERCLOCKWISE`, `Rotated 90/180/270`, `EXIF::Rotate90/180/270`), glimmer `CropImageOperation`/`RotateImageOperation`.
- Wrote `maintenance/geometry-spec.md` with domain-model mapping (NormalizedRect angle_rad quarter_turns).

M1 in progress; geometry sub-spec pass 6 CLOSED.

## M1 status (pass 7 — DB/metadata schema from real files)
- Located the live Picasa catalog on the reference machine: `%LOCALAPPDATA%\Google\Picasa2\db3\` (thumbs, bigthumbs, previews, albums, facetemplates, thumbindex + `*_index.db`) and 143 per-folder `.picasa.ini` files.
- **Corrected a key assumption:** the `db3` files are **NOT SQLite** (`file is not a database`). They use a private container format: f32 version header (`1.6` for stores, `3.1` for `thumbindex.db`) + u32 count, then records.
- Image stores (`thumbs_0.db`, `bigthumbs_0.db`, `previews_0.db`) are **concatenated raw JPEG records** with no separators (SOI/EOI back-to-back); lookup is via the `*_index.db` side files. They are pure derived cache.
- `albums_0.db` payload is compressed/obfuscated (no plaintext album names); full schema recovery needs RE of the `Picasa3.exe` reader, blocked by the Ghidra 330 s deadline + Windows rizin unavailability.
- **Authoritative layer = `.picasa.ini`** (plaintext INI, CRLF): `[Picasa]` folder keys (`name`, `date`, `P2category`), `[<filename>]` per-photo keys (`star`, `caption`, `keywords`, `rotate`, `crop`, `filters`, `albums`, `hidden`, `geotag`, `text`, `faces`, `backuphash`), and `[Contacts2]` people section. Case-insensitive keys.
- Defined the import/backward-compat strategy: read-only `.picasa.ini` parse (authoritative), ignore `db3` (derived), preserve unknown keys, non-destructive own-store model.
- Rewrote `formats/pmp-notes.md` (superseded its TODO section).

M1 in progress; DB/metadata schema pass 7 CLOSED.
Remaining for M1: final spec completeness pass (cross-cutting review + gap sweep).
