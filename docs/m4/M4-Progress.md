# M4 Progress

Running log of the album milestone. One section per step, newest last.

## Step 1 — album, keyword and smart-album persistence

**Started from a wrong assumption, corrected against the real schema.** I first
wrote `0003_albums_tags.sql` as a `CREATE TABLE albums / tags / album_photos`
migration. Every database test then failed with `table albums already exists`.

That was the schema telling me what I had not checked: migrations **0001 and
0002 already create the album layer**.

- `0001` defines `albums (id, name, kind, param)` where `kind` is `manual`,
  `folder` or `smart` and `param` holds the folder path or the saved query;
  it also defines `album_photos (album_id, photo_id, position)`, `tags` and
  `photo_tags`.
- `0002` then adds the album dialog's columns: `description`, `place_taken`,
  `music_path` and `date`.

So there was no schema to add — only behaviour. The rewritten `0003` is
index-only:

| Index | Why |
|---|---|
| `idx_album_photos_ordered` | the order inside an album is user state, and the album editor and slideshow read it |
| `idx_albums_name_nocase`, `idx_tags_name_nocase` | "Trip" and "trip" are the same album or keyword; SQLite cannot alter a column's collation in place, so a `COLLATE NOCASE` index lets the lookup stay index-backed |

**Added on top of the existing schema** (`crates/pinacoteca-db/src/albums.rs`):

- `create_album` (manual), `create_folder_album`, `list_albums`, `rename_album`,
  `delete_album`, `set_album_detail`.
- Membership: `add_photos`, `remove_photo`, `reorder`, `album_photos`.
  `add_photos` skips paths that are not in the catalog instead of failing the
  whole call, and `reorder` moves photos to the front **without ever dropping
  one** — only `remove_photo` may shrink membership (clean-room rule 5).
- Keywords: `ensure_tag`, `list_tags`, `tag_photo`, `photos_by_tag`.
- Smart albums as `Smart::{Recent, Starred, Year, Month}` with `smart_photos`,
  recomputed from the catalog rather than stored.

`SCHEMA_VERSION` is now 3. The version test asserts it equals `MIGRATIONS.len()`
instead of a literal, so it cannot silently drift again.

**Worth writing down:** my first test expected `reorder` to *replace* the album's
run and it failed with `["c","a","b"]`. The implementation was right and the
test was wrong — rewriting the run would silently delete photos from an album.
The test is now `reorder_moves_photos_forward_without_dropping_any` and asserts
the non-destructive behaviour explicitly.

Validation: fmt clean, clippy clean (`-D warnings`), **73 tests** passing
(19 in `pinacoteca-db`).

## Step 2 — album sidebar, smart-album views and stars

The grid used to show only the scan result. It now renders a [`View`], and the
album sidebar drives which one:

- `View::All` — the last scan;
- `View::Album(id)` — `album_photos` in user order;
- `View::Smart(smart)` — a saved query, recomputed every time it is opened.

Album entries are built by `entry_for`, which **re-stats the file** instead of
trusting the row's stored size and mtime. That is deliberate: the scanner builds
its `ThumbKey` from the live file metadata, so re-reading it makes an album entry
produce the *same* key and share one cache file per photo. A test
(`album_entries_share_the_scanner_thumbnail_key`) pins that invariant, because
getting it wrong would silently double every decode and every cache entry.

Stars are stored through `set_photo_rating`, painted on the grid cell as a small
bar of yellow marks, and bound to keys `0`–`5` (0 clears). `ratings_by_path`
loads every stored rating in one query after a scan, so painting the grid does
not query per cell.

Sidebar actions: create album, add the selected photo, delete album. Deleting an
album removes rows from `albums` only — the `ON DELETE CASCADE` clears
membership, and the photos stay in the catalog.

Validation: fmt clean, clippy clean (`-D warnings`), **74 tests**, 8 in
`pinacoteca-app`.

Still open in M4: remove-from-album and drag-and-drop reordering, folder views,
the album dialog fields (the columns exist), and the `.picasa.ini` album import.
