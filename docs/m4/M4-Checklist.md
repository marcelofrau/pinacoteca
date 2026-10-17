# M4 — Albums

Scope per `docs/roadmap.md`: *albums — virtual SQLite albums, drag-drop, stars,
tags, smart algorithms, folder views.*

Exit criteria:

- [ ] Album model and persistence: virtual albums referencing photos by id,
      never by file (no album edit can delete or move a photo).
- [ ] Album ordering: user-defined `position`, dense and zero-based.
- [ ] Album metadata: the fields the album dialog exposes (description, place
      taken, music, date) are stored and round-trip.
- [ ] Keywords: per photo, case-insensitive, searchable by name.
- [ ] Smart albums: recomputed from the catalog, not stored.
- [ ] UI: album list, create album, add/remove photos, stars in the grid.
- [ ] UI: folder views.
- [ ] `.picasa.ini` album import, preserving unknown keys (backward compat).
- [ ] Docs updated: architecture, completeness review.
