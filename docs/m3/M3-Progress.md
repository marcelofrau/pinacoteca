# M3 progress — Viewer MVP

Running log, one section per work step.

## Step 1 — decoders (done)
- `pinacoteca-decoders` now implements ADR-0001's choice:
  - `jpeg::JpegDecoder` — `zune-jpeg`; `dimensions()` via `decode_headers()`
    (no full decode); `decode()` promotes RGB/gray to RGBA.
  - `png::PngDecoder` — `png` crate; header-only dimensions; RGB/gray/alpha
    promoted to RGBA via `normalize_to_color8`.
  - `exif::Orientation` + `read_orientation()` — EXIF 1..=8, with
    `oriented_dimensions()` for the viewer transform.
  - `default_registry()` builds the standard set (JPEG + PNG).
- Synthetic fixtures committed: `crates/pinacoteca-decoders/tests/fixtures/tiny.{jpg,png}`.
- 10 tests (registry, orientation mapping, real JPEG/PNG decode, IO error).

Status: decoder layer ready for the scan + thumbnail work.
