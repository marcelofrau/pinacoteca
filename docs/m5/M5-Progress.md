# M5 progress

Running log for the maintenance milestone. Updated as each piece lands.

- **M5 plan**: tracking opened.

## Step 1 — edit pipeline and persistence

Landed:

- `pinacoteca-render::edits` — `apply` / `apply_op` replay a stack in one pass.
  `geometry::{rotate, crop, straighten}`; `tone::{brightness, contrast,
  saturation, warmth, auto_fix, sharpen, effect, red_eye}`.
- `pinacoteca-db::edits` — `load_edit_stack`, `save_edit_stack`,
  `clear_edit_stack`, `edited_photo_count`. One `edit_ops` row per op, written in
  a transaction so a failed save cannot leave a half stack.
- `pinacoteca-db` now depends on `pinacoteca-core`; `DbError::EditPayload` for a
  corrupted stored row.

Choices worth recording:

- Adjustment values are `-100..=100`, our own mapping. The analysis has only the
  catalog (labels and ranges), and the pixel maths is re-derived here.
- `auto_fix` is a per-channel 0.5%–99.5% percentile stretch.
- `sharpen` is a 3x3 unsharp mask, `strength` roughly 0..=20.
- `red_eye` neutralises strongly red pixels inside the rectangle only.
- `straighten` grows the canvas so no content is lost; positive is clockwise.

Tests: 107 across the workspace (39 in `pinacoteca-render`, 27 in
`pinacoteca-db`).
