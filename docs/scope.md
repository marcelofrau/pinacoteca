# Scope

This document defines what Pinacoteca covers and, just as importantly, what it
deliberately leaves out. Scope changes require an explicit decision recorded here.

## In scope

### Image viewing

| Feature | Notes |
|---------|-------|
| Folder scan and library indexing | Read-only traversal; incremental indexing |
| Thumbnail grid | Picasa-like dense grid; incremental loading |
| Fullscreen viewer | GPU-rendered, zoom and pan |
| Slideshow | Timed advance, keyboard control |
| Keyboard navigation | Grid and viewer navigation, next/previous, fit/actual size |
| EXIF orientation | Honored at render time |
| Common formats | JPEG, PNG, WebP, GIF, BMP, TIFF |
| Extended formats | RAW and HEIC are candidates for an optional decoder layer (decided in M2) |

### Album organization

| Feature | Notes |
|---------|-------|
| Virtual albums | Database-backed; files are never moved |
| Folder-based views | Each folder can be browsed as an album |
| Tags | Free-form labels |
| Ratings (stars) | 0–5, used for filtering and smart albums |
| Smart albums | Saved queries, e.g. by rating, date range or tag |

### Photo maintenance

| Feature | Notes |
|---------|-------|
| Rotate | 90° increments, orientation stored in the edit stack |
| Crop and straighten | Non-destructive |
| Auto-fix | Level / contrast / color auto-correction |
| Red-eye removal | Interactive |
| Basic adjustments | Brightness, contrast, saturation, warmth |
| Sharpen | Non-destructive |
| Effects | Grayscale, sepia and similar presets |

All maintenance operations are non-destructive and reversible. Original files are
never modified.

## Out of scope (initial milestones)

These are intentionally excluded. They are considered dated, obsolete for the
project's purpose, or dependent on discontinued external services.

| Excluded feature | Reason |
|------------------|--------|
| Import from devices / cameras | Not part of the "browse and organize" core; OS file management suffices |
| Export and format conversion | Deferred; not required for viewing or organizing |
| Sharing and upload | Depends on discontinued services; not a local-first goal |
| Facial recognition / face grouping | Not a local-first goal; deferred indefinitely |
| Cloud sync, online accounts, backup | Contradicts the local and private premise |
| Geotagging / maps | Depends on external map services; deferred |

## Deferred, not forgotten

Some features are plausible later but explicitly not part of the initial
milestones. They must not leak into implementation work without an approved scope
change:

- Reading an existing third-party library format for migration.
- Print / poster layout.
- Video playback (Picasa had limited support; out of scope here).
- Plugin or scripting surface.

## Decision log

| Date | Decision |
|------|----------|
| 2026-10-15 | Initial scope set: viewing, albums, maintenance in; import, export, sharing, face recognition out. |
| 2026-10-15 | License chosen: GPL-3.0-or-later. |
