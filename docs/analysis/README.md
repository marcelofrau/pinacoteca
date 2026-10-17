# Behavioral analysis (M1)

Clean-room behavioral specifications produced in M1. These describe Picasa's
observable behavior deeply enough to drive an independent reimplementation.

Rules (see [../cleanroom-protocol.md](../cleanroom-protocol.md)):

- Documents describe **observable behavior**: inputs, outputs, visible effects,
  formats, timings, edge cases.
- Documents do **not** contain decompiled or disassembled code, and do not
  describe internal implementation details of the reference application.
- Raw tool output does **not** belong here. It goes to the git-ignored
  `analysis-scratch/` directory; superseded drafts go to
  `analysis-scratch/superseded/`.

## Delivered specifications

| Spec | Area | Closed |
|------|------|--------|
| [`viewer/viewer-spec.md`](viewer/viewer-spec.md) | Viewer (`PicasaPhotoViewer.exe`): command set, filmstrip, zoom, slideshow, pan, keyboard, Fen UI framework | pass 3 |
| [`albums/albums-spec.md`](albums/albums-spec.md) | Albums, grid, thumbnails, view modes, special albums, star/keyword model | pass 5 |
| [`maintenance/maintenance-spec.md`](maintenance/maintenance-spec.md) | Maintenance & effects catalog (84 filters), modes, sliders, edit engine | pass 4 |
| [`maintenance/geometry-spec.md`](maintenance/geometry-spec.md) | Crop / straighten / rotate canvas model | pass 6 |
| [`formats/pmp-notes.md`](formats/pmp-notes.md) | DB & metadata stores (`db3` container + per-folder `.picasa.ini`) | pass 7 |

## Process & tracking

| Document | Contents |
|----------|----------|
| [`M1-REA-Plan.md`](M1-REA-Plan.md) | Analysis plan (string-driven + resource-driven REA) |
| [`M1-REA-Preliminary.md`](M1-REA-Preliminary.md) | Preliminary reconnaissance |
| [`M1-Summary-Pass1.md`](M1-Summary-Pass1.md) | Pass 1 summary |
| [`M1-Checklist.md`](M1-Checklist.md) | Milestone checklist |
| [`M1-Progress.md`](M1-Progress.md) | Running progress log (per pass) |
| [`M1-Completeness.md`](M1-Completeness.md) | Final gap review + clean-room readiness verdict |

## Status

**M1 complete.** The specs cover the initial scope — image viewing, album
organization, and photo maintenance — and are sufficient for clean-room
reimplementation. See [`M1-Completeness.md`](M1-Completeness.md) for the
residual (non-blocking) gaps and their resolution plan.
