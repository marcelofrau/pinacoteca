# M2 checklist — Architecture spike

Goal: settle the technical foundation with measurements, not assumptions
(roadmap M2). Exit criteria: documented decisions with benchmark evidence;
performance budgets recorded in `docs/architecture.md`.

## Tracking & scaffolding

- [x] Create `docs/adr/` with template + index
- [x] Create `docs/m2/` progress tracking

## Deliverables

- [x] **Decoder benchmark** on a real photo set (pure-Rust vs optional native)
      — `pinacoteca-bench` harness + `docs/m2/decoder-benchmark.md`; ADR-0001
- [x] **GPU rendering pipeline prototype**: mip/pyramid strategy for instant zoom
      — `pinacoteca-render` (pyramid + level selection, 6 tests) +
      `docs/m2/pyramid-benchmark.md`; ADR-0002
- [x] **Thumbnail cache design + background worker prototype** — `pinacoteca-render::thumbs`
      (9 tests) + ADR-0003
- [x] **Database schema v1** (catalog, albums, tags, ratings, edit stacks)
      — migration `0002` + serde edit ops + ADR-0004
- [x] **Concurrency model** (UI thread, scan, decode pool) — ADR-0005

## Closeout

- [x] Record performance budgets in `docs/architecture.md`
- [x] Update `docs/architecture.md` status (spike items settled)
- [x] M2 completeness review + verdict — `docs/m2/M2-Completeness.md`
