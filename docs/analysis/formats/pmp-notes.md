# PMP / DB / Metadata Stores

No `.pmp` literal exists in the shipped runtime. Picasa persists its catalog in
**two independent layers**:

1. A **global index** under `%LOCALAPPDATA%\Google\Picasa2\db3\` — a *private
   container format* (NOT SQLite), holding thumbnail/preview caches and virtual
   album/contact definitions.
2. **Per-folder `.picasa.ini`** — plaintext INI files, one per scanned folder,
   holding the authoritative per-photo metadata (stars, captions, keywords,
   rotation, crop, filters, album membership) and folder-level metadata.

> **Clean-room note:** all findings below are schema/behaviour only. Real
> databases and `.picasa.ini` files live on the user's machine and are **never**
> committed to this repo. Personal values (filenames, captions, face hashes) are
> deliberately not reproduced here.

---

## 1. Global index — `db3` container format

The `*.db` files in `%LOCALAPPDATA%\Google\Picasa2\db3\` are **not SQLite**.
`file is not a database` when handed to any SQLite reader. They use a custom
record container. Observed live files (sizes are of the reference machine, for
scale only):

| File | Size | Kind |
|------|------|------|
| `thumbs_0.db` | 272 MB | thumbnail JPEG blob store |
| `thumbs2_0.db` | 99 MB | secondary thumbnail store |
| `bigthumbs_0.db` | 171 MB | large thumbnail store |
| `previews_0.db` | 387 MB | screen-size preview store |
| `albums_0.db` | 17 MB | virtual album + contact definitions |
| `facetemplatesV2_0.db` | 23 MB | face templates (out of scope) |
| `thumbindex.db` | 1.8 MB | path → thumbnail index |
| `*_index.db` | small | per-store B-tree index (offsets/lookup) |
| `profilephotos_0.db` | 4 B | empty (header only) |

### 1.1 Container headers

Every file begins with a **little-endian IEEE-754 float version tag**, then a
32-bit count:

| File group | First 4 bytes (hex) | As f32 | Meaning |
|------------|--------------------|--------|---------|
| data stores | `cd cc cc 3f` | `1.6` | store format version |
| `thumbindex.db` | `66 66 46 40` | `3.1` | index format version |

`thumbindex.db` then has `11 b1 00 00` → `uint32 = 45329` record count.

### 1.2 Image stores = concatenated image blobs

`thumbs_0.db` / `bigthumbs_0.db` / `previews_0.db` bodies are **raw image
records laid back-to-back with no separators**. In `thumbs_0.db` the very first
record is a JPEG starting at offset 4 (`ff d8 ff e0 00 10 4a 46 49 46` = SOI +
JFIF APP0); the next record's SOI follows the previous record's EOI (`ff d9`)
directly. Record boundaries are therefore derived from the image markers
themselves, and the `*_index.db` files provide the random-access lookup
(offset/size/path) into this blob stream.

Implication: the thumbnail caches are **pure derived data** — we do not need
their internal index layout, and we can safely ignore these files because
Pinacoteca builds its own cache. (If we ever wanted a warm start we could scan
for SOI/EOI pairs, but there is no need.)

### 1.3 `thumbindex.db` = path table

Appears to be a sequence of folder/photo records:

```
[path\0][8-byte FILETIME][8-byte FILETIME][flags/word fields][0xff ff ff ff][next path\0]…
```

Paths are absolute Windows paths (e.g. `<drive>:\…\`), null-terminated.
The FILETIME values decode to plausible recent timestamps. Exact field widths
were not fully pinned down (see open questions) — this file is also **derived
index data**, not authoritative metadata.

### 1.4 `albums_0.db` = album/contact definitions

Header is the same `1.6` float + 32-bit fields, but the payload shows **no
plaintext album names** — it is compressed/obfuscated, so album titles are not
recoverable by simple string scanning. Full schema recovery would require
reverse-engineering the reader/writer in `Picasa3.exe`, which is **blocked**:
the 10 MB binary cannot complete Ghidra auto-analysis within the 330 s startup
deadline (see project constraints), and the Windows rizin provider is
unavailable.

### 1.5 Status

The `db3` format is documented here **structurally** (headers, blob
concatenation, index role) and is explicitly **out of the critical path** for
Pinacoteca because layer 2 (`.picasa.ini`) carries the authoritative metadata.
A full `db3` reader is a nice-to-have for a future one-shot migration, not a
requirement for any milestone.

---

## 2. Per-folder `.picasa.ini` — authoritative metadata

Plain text INI, CRLF line endings, UTF-8/ANSI. One file per folder that Picasa
has indexed. **Schema observed live** (143 files scanned on the reference
machine; key names only — no values reproduced):

### 2.1 Section kinds

| Section | Count seen | Purpose |
|---------|-----------|---------|
| `[<filename.ext>]` | 6977 | per-photo metadata |
| `[Picasa]` | 3 | folder-level metadata |
| `[Contacts2]` | 1 | people/contact definitions |

### 2.2 `[Picasa]` — folder-level keys

```ini
[Picasa]
name=<display name of the folder>      # e.g. a date-styled folder label
date=<folder date>                     # formatted date string
P2category=<category string>           # folder category ("P2" = Picasa 2 scheme)
```

Keys are matched case-insensitively (`P2category` and `p2category` both appear).

### 2.3 `[<filename>]` — per-photo keys

Seen live: `backuphash`, `hidden`, `faces`. The full set documented by Picasa
and present in the binary's string corpus (all lowercase keys):

| Key | Meaning |
|-----|---------|
| `star=yes` | 1-star flag (star ratings; `XMP::Rating` mirror) |
| `caption=<text>` | user caption |
| `keywords=a,b,c` | comma-separated keyword tags |
| `rotate=rotate(N)` | 90°-step rotation (`rotate(0/90/180/270)`) |
| `crop=rect64(<hex>)` | non-destructive crop rectangle (normalized 64-bit) |
| `filters=...` | applied filter/edit stack descriptor |
| `albums=<token>` | album membership (by album token) |
| `hidden=yes` | exclude from standard views |
| `geotag=lat,lon` | manual geotag |
| `faces=...` | face rectangles (out of scope) |
| `text=...` | text/annotation layer |
| `backuphash=<int>` | internal original-vs-edited bookkeeping hash |

### 2.4 `[Contacts2]` — people

```ini
[Contacts2]
<hex-token>=<value>      # one line per contact; key is a 64-bit hex id
```

Face/people data — explicitly **out of initial scope** (face recognition was
excluded from the project), but recorded so the importer can recognise and skip
it rather than choke on it.

### 2.5 Observed value shapes (patterns, not content)

- `hidden` → 3 chars (`yes`).
- `[Picasa] name` → 7 chars (matches a `YYYY-MM` style folder label).
- `[Picasa] date` → 12 chars (a formatted date).
- `P2category` → 15–19 chars (category string).
- `[Contacts2]` value → 9 chars; `faces` → 41 chars (packed binary-ish).

---

## 3. Import / backward-compatibility strategy for Pinacoteca

Hard requirement: **never break existing installs; migrate old configs forward
gracefully; originals are never modified.**

1. **Authoritative source = `.picasa.ini`.** Import is a *read-only* parse of
   each folder's `.picasa.ini`: folder metadata from `[Picasa]`, per-photo
   metadata from the `[<filename>]` sections. Unknown keys are preserved
   verbatim in our store so nothing is silently dropped.
2. **`db3` is derived cache → ignore.** No `db3` parsing is required for any
   milestone; we build our own thumbnail cache. A future one-shot "warm start"
   migration can scan the image-blob stores for SOI/EOI pairs if desired.
3. **Non-destructive, our-own-store model.** We never rewrite Picasa's files.
   The `.picasa.ini` is read as a source of truth at import time and our edits
   live in Pinacoteca's own DB (mirroring Picasa's own "originals untouched"
   model).
4. **Round-trip safety.** Because `filters=`, `crop=`, `albums=` etc. use
   Picasa-specific encodings, the first cut imports the *understood* subset
   (star, caption, keywords, rotate, hidden) and keeps the raw section text for
   anything not yet decoded.

---

## 4. Open questions / follow-ups

- Exact `thumbindex.db` record field widths (FILETIMEs + trailing word fields)
  — low priority, derived data only.
- `albums_0.db` payload encoding (appears compressed) — needs RE of the reader,
  currently blocked by the Ghidra/rizin constraints above.
- Full `filters=` edit-stack grammar (ties into the maintenance spec's
  `EditOp`/`EditStack` mapping).
- Whether `P2category` has a small fixed enumeration worth mapping to our
  folder-category model.
