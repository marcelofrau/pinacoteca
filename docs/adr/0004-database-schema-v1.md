# 4. Database schema v1

- **Status:** accepted
- **Date:** 2026-10-17
- **Milestone:** M2
- **Deciders:** Pinacoteca maintainers
- **Evidence:** M1 specs (`docs/analysis/`), `crates/pinacoteca-db/src/migrations/`

## Context

Pinacoteca must persist the catalog and all user state without ever writing to
the original image files. M1 established what has to be stored:

- a **folder tree** is a first-class view (`ID_VIEW_FOLDERS`), so folders are
  modelled, not inferred from paths each time;
- the **viewer and thumbnail pipeline** read dimensions, EXIF orientation, date
  taken and format, so those belong in the catalog rather than being re-read
  from every file on every open;
- **albums** carry the fields the album dialog exposes (name, date, place taken,
  description, music);
- the **edit stack is non-destructive** (`EditStack`), so it is stored as ordered
  operations, never as a modified file;
- M4 will import per-folder `.picasa.ini` (ADR-relevant backward compatibility),
  which needs a schema that can hold the same facts.

## Options considered

1. **SQLite via `rusqlite` (bundled)** — embedded, transactional, queryable,
   already a dependency; bundled avoids a system SQLite.
2. **A custom file format** — full control, but re-implements indexing, queries
   and crash recovery for no benefit.
3. **A document/key-value store** — poor fit for the relational facts
   (many-to-many albums/tags, ordered edit ops).

## Decision

- **SQLite through `rusqlite` with the `bundled` feature**, so the build needs no
  system SQLite library.
- **Ordered migrations** keyed by SQLite's `user_version`: `MIGRATIONS[i]` takes
  the schema to version `i + 1`; a database newer than the build is refused rather
  than silently downgraded.
- **Schema v2** (v1 = catalog skeleton, v2 = catalog metadata) is:

| Table | Purpose |
|-------|---------|
| `folders` | indexed folder tree (`path` unique) |
| `photos` | catalog facts: path (unique), rating, folder, size, mtime, width, height, orientation, date taken, format, caption, hidden |
| `albums` | virtual albums: name, kind (`manual`/`folder`/`smart`), param, description, place taken, music, date |
| `album_photos` | many-to-many membership with `position` |
| `tags` / `photo_tags` | keywords and their many-to-many link |
| `edit_ops` | one row per operation: `photo_id`, `position`, `kind`, `payload` (JSON) |

- **Edit operations are stored one row per op**, ordered by `position`, with the
  operation serialised as **JSON** in `payload` (`EditOp` derives serde; verified
  by a round-trip test). The column is deliberately `kind` + `payload` so an
  unknown/future op kind is preserved rather than dropped.
- **Original files are read-only inputs**; no table references a writable file.
  `foreign_keys` is enabled on every connection.

## Consequences

- Adding fields later is a new migration file plus one entry in `MIGRATIONS` —
  no data rewriting, and old databases upgrade in place (a hard requirement for
  existing installs).
- JSON payloads keep the edit stack forward-compatible at the cost of not being
  queryable by parameter; that is acceptable because edit ops are read as an
  ordered stack for one photo, never aggregated across photos.
- The thumbnail cache is **not** in the database (ADR-0003): it is derived,
  regenerable data that lives on disk beside the DB. This keeps library backups
  small and lets the cache be deleted freely.
- `.picasa.ini` import (M4) maps onto these tables: `[Picasa]` → `folders`
  metadata, per-photo keys → `photos` columns and `edit_ops`, `[Contacts2]` →
  out of scope (faces excluded).
