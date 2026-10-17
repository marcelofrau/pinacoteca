# M2 Completeness Review — Architecture Spike (final)

Gate for M2: the technical foundation is settled **with measurements**, and the
performances budgets are recorded in [`../architecture.md`](../architecture.md).

## Deliverables vs roadmap

| Roadmap deliverable | Result | Evidence |
|---------------------|--------|----------|
| Decoder benchmark (pure-Rust vs optional native) | Done | `m2/decoder-benchmark.md`, ADR-0001 |
| GPU pipeline prototype: mip/pyramid for instant zoom | Done | `m2/pyramid-benchmark.md`, ADR-0002, `pinacoteca-render::pyramid` (6 tests) |
| Thumbnail cache design + background worker prototype | Done | ADR-0003, `pinacoteca-render::thumbs` (9 tests) |
| Database schema v1 (catalog, albums, tags, ratings, edit stacks) | Done | ADR-0004, migration `0002`, serde edit ops |
| ADRs under `docs/adr/` | Done | ADR-0001…0005, all accepted |

Exit criteria: **met** — decisions carry benchmark evidence and the budgets are
in `architecture.md`.

## What was decided

- **Decoders:** pure-Rust default (`zune-jpeg` + `png`); native `mozjpeg` optional
  (~18% faster, needs a C toolchain). ADR-0001.
- **Zoom/render:** lazy mip pyramid for the current photo; level chosen by zoom
  scale (≤ ~2× minification). ADR-0002.
- **Thumbnails:** content-keyed on-disk cache with size classes + EXIF-thumb-first
  generation; deduplicating priority request queue. ADR-0003.
- **Storage:** SQLite schema v2 with ordered migrations; edit ops one JSON row
  each. ADR-0004.
- **Concurrency:** OS threads + channels, no async runtime; generation-token
  cancellation, bounded-channel backpressure. ADR-0005.

## New code landed in M2

- `pinacoteca-render` (new crate): `pyramid`, `thumbs` — 15 tests.
- `pinacoteca-bench` (new, analysis-only): decoder + pyramid + EXIF harness.
- `pinacoteca-db`: migration `0002`, 3 new tests (6 total).
- `pinacoteca-core`: serde on the edit model, 1 new test (9 total).

## Residual items (all non-blocking, scheduled)

| Item | Why open | Where |
|------|----------|-------|
| GPU upload/sampling itself | Needs a window/device; logic is settled and tested headlessly | M3 |
| Tiled upload of pyramid level 0 | Only matters above ~24 MP or with several full-res views | documented follow-up |
| Worker pool wiring, priority formula, generation tokens | Runtime concern | M3 |
| Concrete `Decoder` implementations | Interface + choice settled; implementation is viewer work | M3 |
| Real end-to-end budget verification | Needs the app running | M3 |

## Verdict

**M2 is complete.** The foundation is decided and recorded with evidence; the
prototypes that could be validated headlessly are implemented and tested. Nothing
here blocks M3.

**Next milestone: M3 — Viewer MVP** (folder scan + incremental indexing, thumbnail
grid with keyboard navigation, fullscreen GPU viewer with zoom/pan and EXIF
orientation, slideshow).
