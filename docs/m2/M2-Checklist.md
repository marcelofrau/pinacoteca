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
- [ ] **GPU rendering pipeline prototype**: mip/pyramid strategy for instant zoom
      — ADR-0002
- [ ] **Thumbnail cache design + background worker prototype** — ADR-0003
- [ ] **Database schema v1** (catalog, albums, tags, ratings, edit stacks)
      — migration + ADR-0004
- [ ] **Concurrency model** (UI thread, scan, decode pool) — ADR-0005

## Closeout

- [ ] Record performance budgets in `docs/architecture.md`
- [ ] Update `docs/architecture.md` status (spike items settled)
- [ ] M2 completeness review + verdict
