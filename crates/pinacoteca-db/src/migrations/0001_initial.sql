-- Schema v1: catalog, albums, tags and non-destructive edit stacks.
--
-- Original image files are never modified. Every table here holds user state
-- or read-only catalog facts.

CREATE TABLE photos (
    id     INTEGER PRIMARY KEY,
    path   TEXT    NOT NULL UNIQUE,
    rating INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5)
);

CREATE TABLE albums (
    id   INTEGER PRIMARY KEY,
    name TEXT    NOT NULL,
    kind TEXT    NOT NULL CHECK (kind IN ('manual', 'folder', 'smart')),
    -- Folder path for 'folder' albums, saved query for 'smart' albums.
    param TEXT
);

CREATE TABLE album_photos (
    album_id INTEGER NOT NULL REFERENCES albums (id) ON DELETE CASCADE,
    photo_id INTEGER NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (album_id, photo_id)
);

CREATE INDEX idx_album_photos_photo ON album_photos (photo_id);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT    NOT NULL UNIQUE
);

CREATE TABLE photo_tags (
    photo_id INTEGER NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (photo_id, tag_id)
);

CREATE INDEX idx_photo_tags_tag ON photo_tags (tag_id);

-- Non-destructive edit operations. `position` orders them within a photo;
-- `kind` names the operation and `payload` holds its parameters as JSON.
-- The original file is never rewritten.
CREATE TABLE edit_ops (
    photo_id INTEGER NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    kind     TEXT    NOT NULL,
    payload  TEXT    NOT NULL,
    PRIMARY KEY (photo_id, position)
);
