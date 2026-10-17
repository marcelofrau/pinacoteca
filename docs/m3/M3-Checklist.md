# M3 checklist — Viewer MVP

Goal (roadmap M3): scan a folder, grid of thumbnails in a dark UI with a bottom
bar, fullscreen GPU viewer with zoom/pan at 60 fps, EXIF orientation, keyboard
navigation.

## Tracking

- [x] Create `docs/m3/` tracking

## Foundation (from M2 decisions)

- [x] Concrete `Decoder` implementations (`zune-jpeg`, `png`) + EXIF orientation
- [ ] Thumbnail generation: EXIF-thumb-first, else decode + downscale (`ThumbSize`)
- [ ] Folder scan → catalog (incremental), persisted via `pinacoteca-db`

## UI

- [ ] Dark theme shell
- [ ] Thumbnail grid (virtualised) with bottom bar
- [ ] Keyboard navigation (arrows, enter, esc, space, f)
- [ ] Fullscreen viewer: zoom (+, -, 1), pan (drag), fit/actual size
- [ ] EXIF orientation applied in the viewer

## Closeout

- [ ] End-to-end check on a real folder; record results
- [ ] M3 completeness review + verdict
