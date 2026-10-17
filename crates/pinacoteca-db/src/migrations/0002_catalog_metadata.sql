-- Migration 0002: catalog metadata, folders and album detail.
--
-- M1 showed the viewer/library needs more than "a photo has a path and a rating":
-- the folder tree is a first-class view, photos need the metadata the viewer and
-- the thumbnail pipeline read (dimensions, orientation, date taken, format), and
-- albums carry the fields the album dialog exposes (description, place taken,
-- music, date).
--
-- Original image files are still read-only inputs; every column here is either a
-- read-only fact about a file or user state.

CREATE TABLE folders (
    id   INTEGER PRIMARY KEY,
    path TEXT    NOT NULL UNIQUE
);

ALTER TABLE photos ADD COLUMN folder_id INTEGER REFERENCES folders (id);
ALTER TABLE photos ADD COLUMN file_size INTEGER;
ALTER TABLE photos ADD COLUMN modified_secs INTEGER;
ALTER TABLE photos ADD COLUMN width INTEGER;
ALTER TABLE photos ADD COLUMN height INTEGER;
-- EXIF orientation (1..=8). 1 = upright.
ALTER TABLE photos ADD COLUMN orientation INTEGER NOT NULL DEFAULT 1;
-- Date taken, seconds since the Unix epoch, when known.
ALTER TABLE photos ADD COLUMN date_taken INTEGER;
-- Lowercase container format, e.g. 'jpg', 'png'.
ALTER TABLE photos ADD COLUMN format TEXT;
-- User caption.
ALTER TABLE photos ADD COLUMN caption TEXT;
-- Hidden photos are excluded from standard views.
ALTER TABLE photos ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_photos_folder ON photos (folder_id);

-- Album detail, mirroring the album metadata dialog.
ALTER TABLE albums ADD COLUMN description TEXT;
ALTER TABLE albums ADD COLUMN place_taken TEXT;
ALTER TABLE albums ADD COLUMN music_path TEXT;
-- Album display date, seconds since the Unix epoch, when set.
ALTER TABLE albums ADD COLUMN date INTEGER;
