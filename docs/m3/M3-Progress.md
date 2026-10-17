# M3 progress — Viewer MVP

Running log, one section per work step.

## Step 2 — thumbnail generation (done)
- `pinacoteca-render::imageops`: `resize_to_fit` (box-average downscale, preserve
  aspect, never upscale) + `apply_orientation` (all 8 EXIF orientations).
- `pinacoteca-render::thumbs::generate`: EXIF thumbnail is used as the fast path
  when it's large enough; otherwise decode full → orient → downscale → encode JPEG
  (q82) and store in the content-keyed cache. Returns `GeneratedThumb` metadata.
- `pinacoteca-decoders::jpeg`: `dimensions_bytes` and `decode_bytes` added to
  support in-memory extraction of EXIF thumbnails.
- `pinacoteca-decoders::exif`: `embedded_thumbnail` / `embedded_thumbnail_file`
  added (detects APP1 EXIF IFD1 thumbnail).
- 23 render tests total (up from 15), including generate downscale/orientation
  roundtrips.
