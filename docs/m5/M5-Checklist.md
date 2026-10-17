# M5 — Maintenance (non-destructive edits)

Scope per `docs/roadmap.md`: *maintenance — rotate, crop, straighten, auto-fix,
red-eye, brightness/contrast/saturation/temperature, sharpen, B&W/sepia effects,
non-destructive edit stack style Picasa.*

Behaviour comes from `docs/analysis/maintenance/maintenance-spec.md` (the 84
filter catalog, slider labels/ranges) and
`docs/analysis/maintenance/geometry-spec.md` (crop / straighten / rotate).

Exit criteria:

- [x] An edit stack renders to an image: ops apply in order, one pass.
- [x] Geometry: `Rotate`, `Crop`, `Straighten`.
- [x] Tone: `Brightness`, `Contrast`, `Saturation`, `Warmth`.
- [x] `AutoFix` (automatic levels and colour).
- [x] `Sharpen`.
- [x] Effects: `Grayscale`, `Sepia` (the catalog stays reference-only, ADR-0001
      boundary in `#1204`).
- [x] `RedEye` within a rectangle.
- [x] Persistence: one `edit_ops` row per op, replayed and reorderable.
- [ ] UI: edit panel with sliders, live preview, undo/reset.
- [ ] The original file is byte-identical before and after editing (test).
- [ ] Docs: `M5-Completeness.md`, architecture update.
