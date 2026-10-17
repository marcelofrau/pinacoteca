# M4 — Albums: completeness review

Scope per `docs/roadmap.md`: *albums — virtual SQLite albums, drag-drop, stars,
tags, smart algorithms, folder views.*

## Covered

| Area | Where | Notes |
|---|---|---|
| Album model | `pinacoteca-db::albums` | `kind` is `manual` / `folder` / `smart`; membership by photo id, never by file |
| Ordering | `album_photos.position` | dense, zero-based; reorder moves a photo **without dropping any other** |
| Detail fields | `description`, `place_taken`, `music_path`, `date` | edited from the sidebar panel (date stored, not edited) |
| Keywords | `tags` + `photo_tags` | case-insensitive via `COLLATE NOCASE` lookup; no duplicate rows |
| Smart albums | `Smart::{Recent, Starred, Year, Month}` | recomputed from the catalog every time the view opens |
| Folder views | `View::Folder` | a projection of the scan; nothing extra stored |
| Stars | `set_photo_rating` + keys `0`–`5` | one `ratings_by_path` query paints the whole grid |
| Drag & drop | grid → album row, grid → grid | add to album / reorder inside album |
| Picasa import | `pinacoteca-core::picasa_ini` | read-only; stars, captions, keywords |

## Decisions and findings

- **The schema already existed.** Migrations 0001 and 0002 already define
  `albums` (with `kind`/`param`), `album_photos`, `tags`, `photo_tags`, and the
  album dialog columns. The first draft of migration 0003 re-created them and
  every test failed with `table albums already exists`. The corrected 0003 adds
  indexes only, so M4 is almost entirely behaviour, not storage.
- **A reorder must never shrink an album.** The first `reorder` test expected
  the album's run to be *replaced*; that would silently delete photos. Reorder
  is therefore move-to-front and the test asserts the lossless behaviour.
- **Album membership cannot be imported from `.picasa.ini`.** Picasa stores
  opaque tokens in `albums=` and resolves the names in its obfuscated
  `albums_0.db`. Stars, captions and keywords *are* recovered, and the INI is
  never rewritten.
- **An album entry must reuse the scanner's thumbnail key.** `entry_for`
  re-stats the file rather than trusting the stored size/mtime, so one photo has
  exactly one cache file no matter which view shows it.

## Residual gaps (documented, not part of the M4 scope)

- Album `date` is stored but has no editor (Picasa's "Date (automatic)").
- No tag browser view; keywords are imported and searchable through
  `photos_by_tag` but not surfaced as a sidebar entry yet.
- `rotate=` from `.picasa.ini` is parsed but not applied to the viewer.
- Album music is stored; nothing plays it.

## Validation

- `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean.
- **84 tests** across the workspace (23 `pinacoteca-render`, 20
  `pinacoteca-db`, 19 `pinacoteca-core`, 12 `pinacoteca-decoders`, 10
  `pinacoteca-app`). The analysis-only `pinacoteca-bench` has no tests.

## Verdict

M4 meets its scope: albums are virtual and non-destructive, ordered, described,
tagged, starrable, viewable by folder and by smart query, reachable by
drag-and-drop, and an existing Picasa library's stars and keywords come across
without touching a single original file.

