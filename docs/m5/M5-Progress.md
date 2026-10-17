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

## Step 2 — the edit panel

Landed:

- `pinacoteca-app::editor` — `Editor` holds the unedited preview pixels, the
  stack, and the live texture. `refresh` replays the stack over those pixels, so
  the same `edits::apply` path serves preview and saved result.
- The panel: rotate left/right, Brightness / Contrast / Saturation / Warmth
  sliders, Sharpen, one-click Auto fix / B&W / Sepia, Undo and Reset; Save and
  Close in the panel footer. Key `E` opens it, `Esc` closes it.
- `EditStack::set` replaces every operation of the same kind, so dragging a
  slider cannot stack a hundred Brightness ops; `push` still composes, so two
  rotations really do make a half turn.
- Saving writes the stack to `edit_ops`; the decode, the preview and the file
  reads are all read-only.

Verification: `editing_never_rewrites_the_original_file` encodes a real JPEG,
opens the editor, applies rotate + brightness + sepia, saves, and asserts the
file bytes on disk are identical and the stack round-trips through the catalog.

Tests: 109 across the workspace.
