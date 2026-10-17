# M2 progress — Architecture spike

Running log, one section per work step.

## Step 0 — recon (done)
- Workspace confirmed: `pinacoteca-app` (eframe 0.33.3), `pinacoteca-core`
  (album/catalog/edit), `pinacoteca-db` (rusqlite bundled, schema v1 = 6 tables),
  `pinacoteca-decoders` (`Decoder` trait + `DecoderRegistry`, no concrete decoder yet).
- Toolchain: cargo/rustc 1.98.0, edition 2024. crates.io reachable.
- Domain model present: `Photo`/`PhotoId`/`Rating`, `Album`/`AlbumKind`,
  `EditOp`/`EditStack`/`Rotation`/`Rect`/`Effect`.
- DB schema v1 already has: photos, albums, album_photos, tags, photo_tags, edit_ops.

Status: recon complete; ADR scaffold created.

## Step 1 — decoder benchmark (done)
- Added workspace member `pinacoteca-bench` (`publish = false`): decodes a real
  photo folder across candidate decoders, plus a scaled-IDCT (thumbnail) section
  and an EXIF section (orientation parse + embedded-thumbnail extraction).
- Candidates: `zune-jpeg`, `jpeg-decoder`, `image` (jpeg + png), `png`, and
  native `mozjpeg` behind an off-by-default `native-mozjpeg` feature.
- Measured on 35 real photos (29 JPEG + 6 PNG, 256 MiB) on a Ryzen 9 5900X:
  - `zune-jpeg` 185 MP/s (fastest pure Rust) > `jpeg-decoder` 165 > `mozjpeg` 218 (native).
  - `image` delegates JPEG to `jpeg-decoder` (identical timings) — no speed win.
  - Scaled IDCT only 1.2–1.3× (entropy decode dominates) → not a thumbnail strategy.
  - Embedded EXIF thumbnail extract ≈ 0.001 ms vs ~70 ms decode; 19/29 present, 350×410 avg (min side 38 → needs size check).
  - EXIF orientation parse negligible (0.015 ms/img); sample had only orientation 1.
- Evidence: `docs/m2/decoder-benchmark.md`. Decision: `docs/adr/0001-decoder-strategy.md`
  (pure-Rust default = `zune-jpeg` + `png`; `mozjpeg` optional feature).
- CI-equivalent green: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (14 tests).

Status: decoder strategy decided and recorded.
