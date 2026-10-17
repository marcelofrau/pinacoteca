# PMP / DB / Metadata Stores (notes)

No `.pmp` literal exists in the shipped runtime — Picasa's local index is a set of
**SQLite-style DB files plus per-folder `.picasa.ini`**. Filenames recovered from
the `Picasa3.exe` string corpus:

| File | Purpose |
|------|---------|
| `thumbs.db` / `thumbs2.db` | primary thumbnail caches |
| `bigthumbs.db` | large thumbnails |
| `previews.db` | screen-size previews |
| `index-thumbs.db` / `thumbs_index.db` / `thumbindex.db` | thumbnail indices |
| `albums.db` | virtual album definitions |
| `profilephotos.db` | profile photos (people) |
| `facetemplatesV2.db` / `facetemplates_0.db` / `facetemplates_index.db` | face templates (out of initial scope) |
| `distims.db` | distance/hash index (duplicate detection) |
| `.picasa.ini` / `Picasa.ini` | per-folder metadata: captions, stars, album membership, rotation |

DB access classes (RTTI): `CThumbDB` (`IThumbDB`), `ThumbCache@CTransTimeline`,
`CThumbnailProvider`/`IThumbnailProvider`, `FolderCache`, `AlbumXML`/`AlbumParse`.

## Schema derivation (TODO)

- Actual table/column layouts must be read from the `.db` files themselves, not
  the string corpus. Inspect a real `thumbs.db` / `albums.db` with a SQLite reader.
- Search targets: `thumbindex`, `preview`, `albumdata`, `catalog`, `distim`.
- `.picasa.ini` key names and forward-migration rules (backward compatibility is a
  hard requirement — never break existing installs).
- Write path must stay **non-destructive**: originals never modified; metadata edits
  land in our own store (mirrors Picasa's own model).
