# M5 — maintenance: completeness review

Scope per `docs/roadmap.md`: *rotate, crop, straighten, auto-fix, red-eye,
brightness/contrast/saturation/temperature, sharpen, B&W/sepia effects,
non-destructive edit stack style Picasa.*

## Coverage

| Roadmap item | Status | Where |
| --- | --- | --- |
| Rotate | done | `render::edits::geometry::rotate` |
| Crop | done (model + renderer) | `geometry::crop` |
| Straighten | done | `geometry::straighten` |
| Auto-fix | done | `tone::auto_fix` |
| Red-eye | done (renderer) | `tone::red_eye` |
| Brightness / Contrast / Saturation / Temperature | done | `tone::{brightness, contrast, saturation, warmth}` |
| Sharpen | done | `tone::sharpen` |
| B&W / sepia | done | `tone::effect` |
| Non-destructive edit stack | done | `core::edit`, `db::edits`, `app::editor` |

## The guarantee

`editing_never_rewrites_the_original_file` encodes a real JPEG, opens the
editor, applies rotate + brightness + sepia, saves, and asserts the bytes on
disk are unchanged and the stack round-trips through the catalog. The editor
only ever reads the file: the preview comes from a decode of the pixels, and
the stack goes to `edit_ops`.

## Decisions

- Adjustment values are `-100..=100` and the pixel mappings are **our own**
  (`tone.rs`). The M1 analysis has only the catalog — labels, ranges,
  conceptual behaviour — and `filterdesc.xml` effect scripts are reference-only
  (`#1204`), so the maths is re-derived here.
- `EditStack::set` replaces every operation of the same kind (sliders);
  `push` composes (rotate). This is what keeps a dragged slider from stacking
  a hundred operations.
- One `edit_ops` row per operation, written in a transaction, `kind` readable
  in SQL and `payload` the JSON op.

## Residual gaps (deliberate, not blocking)

- **Crop and red-eye have no rectangle UI yet.** Both are implemented and
  tested in the renderer and persist as ops, but the panel has no drag-rectangle
  or slider control for them; a crop can only be applied by writing the op.
- **Two of the catalog's 84 filters.** Only `Grayscale` and `Sepia` exist;
  every other effect in the catalog is a reference-only description.
- **No reorder/replay of individual ops** in the panel — Undo and Reset only.
- **Straighten grows the canvas.** Picasa auto-crops to the largest inscribed
  rectangle; we keep the corners and expose them as transparent.
- **Preview is a separate decode.** Editing re-decodes the file at preview
  resolution instead of reusing the thumbnail cache.

## Validation

- 109 tests across the workspace, `cargo fmt` and `cargo clippy -D warnings`
  clean, CI green on ubuntu and windows.
- Non-destructive property covered by an end-to-end test.

## Verdict

M5 meets its roadmap scope: every listed operation exists, applies through one
non-destructive pipeline, persists per photo, and provably leaves the original
untouched. The gaps above are UI surface and catalog breadth, both additive.
