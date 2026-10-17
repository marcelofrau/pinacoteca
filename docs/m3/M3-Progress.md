# M3 progress — Viewer MVP

Running log, one section per work step.

## Step 1 — decoders (done)
- `pinacoteca-decoders` implements ADR-0001's choice: `jpeg::JpegDecoder`
  (`zune-jpeg`, header-only dimensions), `png::PngDecoder`, `exif::Orientation`
  + `read_orientation`, `default_registry()`.
- Synthetic fixtures; 10 tests.

## Step 2 — thumbnail generation (done)
- `pinacoteca-render::imageops`: `resize_to_fit` (box downscale, no upscale) +
  `apply_orientation` (8 EXIF cases).
- `pinacoteca-render::thumbs::generate`: EXIF-thumb-first, else decode → orient →
  downscale → encode JPEG (q82) into the content-keyed cache.
- `pinacoteca-decoders`: `jpeg::{dimensions_bytes, decode_bytes}`,
  `exif::{embedded_thumbnail, embedded_thumbnail_file}`.
- 23 render tests.

## Step 3 — folder scan + catalog persistence (done)
- `pinacoteca-core::scan`: `scan_folder` streams `DiscoveredPhoto` (path, size,
  mtime) recursively, skipping hidden dirs and Picasa's `Originals`; `ScanStats`
  counts photos/dirs/errors. 12 core tests.
- `pinacoteca-db::catalog`: `upsert_folder`, `upsert_photo_facts` (facts only —
  re-scan never clobbers rating/caption), `set_photo_image_info`,
  `set_photo_rating`, `set_photo_caption`, `count_photos`, `photo_by_path`,
  `photos_in_folder`. 10 db tests.
- 58 workspace tests.

Status: data layer ready; UI (grid, bottom bar, viewer) next.
