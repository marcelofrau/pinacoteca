-- Migration 0003: album ordering and case-insensitive lookups.
--
-- The album and tag tables arrive in 0001, and the album dialog's metadata
-- columns (description, place_taken, music_path, date) arrive in 0002. What is
-- still missing is the support for the behaviour documented in
-- docs/analysis/albums/albums-spec.md:
--
--   * an album keeps its own user-defined order, which the album editor and the
--     slideshow both read, so `position` needs its own index;
--   * "Trip" and "trip" are the same album / keyword to a person, so the names
--     are matched case-insensitively. Rather than rebuild the tables with a new
--     collation (SQLite cannot alter one in place), the indexes below let
--     `WHERE name = ?1 COLLATE NOCASE` use a scan-free path.
--
-- This migration only adds indexes: it never rewrites or drops a column.

-- Order inside an album is user state, not an accident of insertion order.
CREATE INDEX idx_album_photos_ordered ON album_photos (album_id, position);

-- Album and keyword names are user labels; case is not meaningful.
CREATE INDEX idx_albums_name_nocase  ON albums  (name COLLATE NOCASE);
CREATE INDEX idx_tags_name_nocase    ON tags    (name COLLATE NOCASE);
