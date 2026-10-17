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
- [x] Album metadata: `description`, `place_taken` and `music_path` round-trip
      from the sidebar detail panel; clearing a field clears the column.
      (`date` is stored but not edited yet.)
- [x] UI: remove a photo from an album — `Delete` (file and catalog row untouched).
- [x] UI: drag-and-drop — dragging a grid cell onto an album adds the photo;
      dragging onto another cell reorders the album.
- [x] UI: folder views — `View::Folder(dir)`, listed in the sidebar.
- [x] `.picasa.ini` album import: stars, captions and keywords restored
      read-only; album membership deliberately skipped (Picasa stores opaque
      tokens there and the names live in its obfuscated `albums_0.db`).
- [x] Docs updated: `M4-Completeness.md`, `architecture.md` (schema v3, Picasa
      library import).
