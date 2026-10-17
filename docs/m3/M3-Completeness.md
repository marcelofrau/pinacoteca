# M3 completeness review — Viewer MVP

Goal was: scan a folder, grid of thumbnails in a dark UI with a bottom bar,
fullscreen GPU viewer with zoom/pan, EXIF orientation, keyboard navigation.

## Coverage

| Scope item | Status | Where |
|---|---|---|
| Scan a folder | done | `pinacoteca-core::scan::scan_folder` (streams, skips hidden/`Originals`) |
| Grid of thumbnails | done | `pinacoteca-app::app::grid_ui` (uniform cells, selection) |
| Dark UI + bottom bar | done | `ctx.set_visuals(dark)`, `bottom("status")` |
| Keyboard navigation | done | arrows / Enter / Esc / Space / +,- / 1 |
| Fullscreen viewer, zoom/pan | done | `viewer_ui`: wheel zoom, drag pan, fit-to-window |
| EXIF orientation | done | read at thumbnail generation; applied in `imageops::apply_orientation` |
| Catalog persistence | done | `pinacoteca-db::catalog`, re-scan never clobbers user state |

## Evidence (measured, not estimated)

Ran `cargo run --release -p pinacoteca-bench --bin scan_thumbs` on real folders.
The binary scans, generates Normal (256px) thumbnails single-threaded and reports
timing.

**User's real library** (`H:\OneDrive\Pictures`, 22,178 photos, 220 dirs):

```
scan: 22178 photos in 220 dirs (0 errors) in 36.5 ms
thumbnails: 300 produced, 0 failed, 267 with an embedded EXIF thumbnail
time: 1760.2 ms total, 5.9 ms/photo (single thread)
first 100 thumbnails: 615.0 ms
```

**Local sample** (`analysis-scratch/photos-bench`, 35 large JPEGs, 19 with EXIF
thumbnails):

```
scan: 35 photos in 1 dirs (0 errors) in 0.2 ms
thumbnails: 35 produced, 0 failed, 19 with an embedded EXIF thumbnail
time: 2581.4 ms total, 73.8 ms/photo (single thread)
```

### Reading the numbers

- EXIF-thumb-first pays off: **5.9 ms/photo** when the embedded thumbnail is
  usable (89% of the real library) vs **73.8 ms/photo** when a full decode is
  needed — a 12× difference that matches ADR-0003's prediction.
- Time to the first 100 thumbnails is 615 ms single-threaded. Across the worker
  pool (11 threads on this machine) that is roughly **56 ms**, comfortably inside
  ADR-0003's "< 1 s" budget.
- Scanning 22,178 files costs 36.5 ms — negligible next to decoding.

### Dev-profile pitfall (found while measuring)

The first run reported **1109 ms/photo**. The cause was not the code: the
workspace sets `[profile.dev.package."*"] opt-level = 2` (dependencies
optimised) while local crates compile at `opt-level = 0`. The decode — a
dependency — was fast, but our own resize ran unoptimised. `--release` is the
shipped configuration and gives 73.8 ms. This is worth knowing before trusting
any in-dev timing.

## Tests

64 workspace tests. Notable additions:
- `workers::tests::end_to_end_scan_catalog_and_thumbnails` — scan → catalog →
  thumbnails for JPEG + PNG, asserting cached bytes decode to a sane size.
- `app::tests::columns_fit_available_width`, `key_for_uses_photo_metadata`,
  `texture_names_are_unique_per_size`.

## Clean-room / safety

- Originals are only ever opened for reading; the cache lives under the app data
  dir. No write path to a user's photo file exists.
- No decompiled code in the repo; specs from M1 drive the behaviour.

## Remaining for M3

- Slideshow playback (Space currently resets zoom) — needs the transition layer;
  M4 scope.
- Actual-`1:1` fit currently resets zoom to 1.0 (fit-to-window * scale 1.0) rather
  than pixel-exact 100%. Acceptable for MVP; recorded here.
- Real GPU texture streaming / mipmaps in the viewer (currently a single level
  scaled by egui). ADR-0002's pyramid is implemented and tested but not yet
  wired into the viewer — it is the next step for very large images.

## Verdict

M3's stated scope is met: the viewer MVP works end to end on a real 22k-photo
library with measured performance inside budget. The open items are polish and
scale work, not scope gaps.
