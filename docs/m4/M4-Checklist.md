# M4 — Albums

Scope per `docs/roadmap.md`: *albums — virtual SQLite albums, drag-drop, stars,
tags, smart algorithms, folder views.*

Exit criteria:

- [x] Album model and persistence: virtual albums referencing photos by id,
      never by file (no album edit can delete or move a photo).
- [x] Album ordering: user-defined `position`, dense and zero-based. Reordering
      moves photos to the front and never drops one.
- [x] Keywords: per photo, case-insensitive, searchable by name.
- [x] Smart albums: recomputed from the catalog, not stored (`Smart::Recent`,
      `Starred`, `Year`, `Month`).
- [x] UI: album list with photo counts, create album, add selected photo.
- [x] UI: stars — keys `0`–`5`, persisted, painted on the grid.
- [ ] Album metadata: columns exist (`description`, `place_taken`,
      `music_path`, `date`) but the dialog fields are not wired yet.
- [ ] UI: remove a photo from an album; reorder by drag-and-drop.
- [ ] UI: folder views.
- [ ] `.picasa.ini` album import, preserving unknown keys (backward compat).
- [ ] Docs updated: architecture, completeness review.
