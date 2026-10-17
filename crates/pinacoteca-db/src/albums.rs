//! Album, keyword and membership persistence.
//!
//! Albums never reference files: membership is a join between `albums` and
//! `photos`, so an album edit can only ever change the database.
//!
//! The schema is inherited from migrations 0001 and 0002:
//! `albums (id, name, kind, param, description, place_taken, music_path, date)`,
//! `album_photos (album_id, photo_id, position)`, `tags (id, name)` and
//! `photo_tags (photo_id, tag_id)`. This module only adds behaviour on top.

use rusqlite::{OptionalExtension, params};

use crate::DbError;
use crate::catalog::PhotoRow;

/// An album plus its detail fields and how many photos it currently holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumRow {
    pub id: i64,
    pub name: String,
    /// One of `manual`, `folder` or `smart`.
    pub kind: String,
    /// Folder path for `folder` albums, saved query for `smart` ones.
    pub param: Option<String>,
    pub description: Option<String>,
    pub place_taken: Option<String>,
    pub music_path: Option<String>,
    pub date: Option<i64>,
    pub photo_count: i64,
}

/// An id-and-name pair, used for keywords.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub id: i64,
    pub name: String,
}

/// A built-in album that is recomputed from the catalog instead of stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Smart {
    /// The `limit` most recently modified photos.
    Recent { limit: i64 },
    /// Photos rated at or above `min_rating`.
    Starred { min_rating: u8 },
    /// Photos taken in a calendar year.
    Year(i32),
    /// Photos taken in a calendar month (1..=12) of any year.
    Month(u8),
}

const ALBUM_COLUMNS: &str = "a.id, a.name, a.kind, a.param, a.description, a.place_taken,
       a.music_path, a.date, COUNT(p.photo_id)";

fn album_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AlbumRow> {
    Ok(AlbumRow {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        param: row.get(3)?,
        description: row.get(4)?,
        place_taken: row.get(5)?,
        music_path: row.get(6)?,
        date: row.get(7)?,
        photo_count: row.get(8)?,
    })
}

impl crate::Database {
    /// Creates a manually curated album.
    pub fn create_album(&self, name: &str) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO albums (name, kind) VALUES (?1, 'manual')",
            params![name],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Creates an album bound to a folder on disk.
    pub fn create_folder_album(&self, name: &str, path: &str) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO albums (name, kind, param) VALUES (?1, 'folder', ?2)",
            params![name, path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every album, ordered by name.
    pub fn list_albums(&self) -> Result<Vec<AlbumRow>, DbError> {
        let sql = format!(
            "SELECT {ALBUM_COLUMNS}
               FROM albums a LEFT JOIN album_photos p ON p.album_id = a.id
              GROUP BY a.id ORDER BY a.name COLLATE NOCASE"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], album_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Renames an album. Fails if the new name is taken.
    pub fn rename_album(&self, id: i64, name: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE albums SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    /// Deletes an album. Photos on disk are untouched by design.
    pub fn delete_album(&self, id: i64) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM albums WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Writes the fields the album dialog exposes.
    pub fn set_album_detail(
        &self,
        id: i64,
        description: Option<&str>,
        place_taken: Option<&str>,
        music_path: Option<&str>,
        date: Option<i64>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE albums SET description = ?1, place_taken = ?2, music_path = ?3, date = ?4
              WHERE id = ?5",
            params![description, place_taken, music_path, date, id],
        )?;
        Ok(())
    }

    /// Appends photos (by path) to an album, keeping the existing order intact.
    ///
    /// Paths that are not in the catalog are skipped rather than failing the
    /// whole call, so a partially indexed folder still yields a usable album.
    pub fn add_photos(&self, album_id: i64, paths: &[String]) -> Result<usize, DbError> {
        let mut inserted = 0usize;
        for path in paths {
            let Some(photo_id) = self.photo_id_by_path(path)? else {
                continue;
            };
            let position: i64 = self.conn.query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM album_photos WHERE album_id = ?1",
                params![album_id],
                |r| r.get(0),
            )?;
            inserted += self.conn.execute(
                "INSERT INTO album_photos (album_id, photo_id, position) VALUES (?1, ?2, ?3)
                 ON CONFLICT (album_id, photo_id) DO NOTHING",
                params![album_id, photo_id, position],
            )?;
        }
        Ok(inserted)
    }

    /// Removes one photo from an album.
    pub fn remove_photo(&self, album_id: i64, path: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM album_photos
              WHERE album_id = ?1 AND photo_id = (SELECT id FROM photos WHERE path = ?2)",
            params![album_id, path],
        )?;
        self.compact_album(album_id)?;
        Ok(())
    }

    /// Moves `paths` to the front of the album's order, skipping unknown ones.
    ///
    /// Photos that are not listed keep their relative order behind the moved
    /// ones. Reordering may never remove a photo from an album (clean-room rule
    /// 5); [`Database::remove_photo`] is the only way membership shrinks.
    pub fn reorder(&self, album_id: i64, paths: &[String]) -> Result<(), DbError> {
        for (position, path) in paths.iter().enumerate() {
            let Some(photo_id) = self.photo_id_by_path(path)? else {
                continue;
            };
            self.conn.execute(
                "UPDATE album_photos SET position = ?1 WHERE album_id = ?2 AND photo_id = ?3",
                params![position as i64, album_id, photo_id],
            )?;
        }
        self.compact_album(album_id)?;
        Ok(())
    }

    /// Renumbers an album's positions so they are dense and zero-based.
    fn compact_album(&self, album_id: i64) -> Result<(), DbError> {
        let photos: Vec<i64> = {
            let mut stmt = self.conn.prepare(
                "SELECT photo_id FROM album_photos WHERE album_id = ?1 ORDER BY position",
            )?;
            stmt.query_map(params![album_id], |r| r.get(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (position, photo_id) in photos.iter().enumerate() {
            self.conn.execute(
                "UPDATE album_photos SET position = ?1 WHERE album_id = ?2 AND photo_id = ?3",
                params![position as i64, album_id, photo_id],
            )?;
        }
        Ok(())
    }

    /// The photos of an album, in the album's own order.
    pub fn album_photos(&self, album_id: i64) -> Result<Vec<PhotoRow>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.path, p.folder_id, p.file_size, p.modified_secs, p.width, p.height,
                    p.orientation, p.date_taken, p.format, p.caption, p.hidden, p.rating
               FROM photos p
               JOIN album_photos ap ON ap.photo_id = p.id
              WHERE ap.album_id = ?1
              ORDER BY ap.position",
        )?;
        let rows = stmt
            .query_map(params![album_id], PhotoRow::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The photos a built-in smart album resolves to, ordered by its definition.
    pub fn smart_photos(&self, smart: &Smart) -> Result<Vec<PhotoRow>, DbError> {
        let (sql, arg): (&str, String) = match smart {
            Smart::Recent { limit } => (
                "SELECT {cols} FROM photos p WHERE 1 ORDER BY p.modified_secs DESC LIMIT ?1",
                limit.to_string(),
            ),
            Smart::Starred { min_rating } => (
                "SELECT {cols} FROM photos p WHERE p.rating >= ?1 ORDER BY p.path",
                min_rating.to_string(),
            ),
            Smart::Year(year) => (
                "SELECT {cols} FROM photos p
                  WHERE p.date_taken >= ?1 AND p.date_taken < ?2
                  ORDER BY p.date_taken",
                year.to_string(),
            ),
            Smart::Month(month) => (
                "SELECT {cols} FROM photos p
                  WHERE strftime('%m', p.date_taken, 'unixepoch') = printf('%02d', ?1)
                  ORDER BY p.date_taken",
                month.to_string(),
            ),
        };
        let sql = sql.replace("{cols}", PHOTO_COLUMNS);

        // Smart::Year needs two bounds; build them here to keep the match arms
        // uniform.
        let mut stmt = self.conn.prepare(&sql)?;
        match smart {
            Smart::Year(year) => {
                let start = days_to_unix(*year, 0);
                let end = days_to_unix(*year + 1, 0);
                let rows = stmt
                    .query_map(params![start, end], PhotoRow::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            }
            _ => {
                let rows = stmt
                    .query_map(params![arg], PhotoRow::from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            }
        }
    }

    /// Creates the keyword if needed and returns its id (case-insensitive).
    pub fn ensure_tag(&self, name: &str) -> Result<i64, DbError> {
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![name],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        self.conn
            .execute("INSERT INTO tags (name) VALUES (?1)", params![name])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every keyword, ordered by name.
    pub fn list_tags(&self) -> Result<Vec<Named>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM tags ORDER BY name COLLATE NOCASE")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Named {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Attaches a keyword to a photo.
    pub fn tag_photo(&self, path: &str, tag_id: i64) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO photo_tags (photo_id, tag_id)
             SELECT id, ?1 FROM photos WHERE path = ?2
             ON CONFLICT DO NOTHING",
            params![tag_id, path],
        )?;
        Ok(())
    }

    /// The photos carrying a keyword, ordered by path.
    pub fn photos_by_tag(&self, tag_id: i64) -> Result<Vec<PhotoRow>, DbError> {
        let sql = format!(
            "SELECT {PHOTO_COLUMNS}
               FROM photos p JOIN photo_tags t ON t.photo_id = p.id
              WHERE t.tag_id = ?1 ORDER BY p.path"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![tag_id], PhotoRow::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn photo_id_by_path(&self, path: &str) -> Result<Option<i64>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM photos WHERE path = ?1",
                params![path],
                |r| r.get(0),
            )
            .optional()?)
    }
}

/// Columns [`PhotoRow::from_row`] expects, in order.
const PHOTO_COLUMNS: &str =
    "p.id, p.path, p.folder_id, p.file_size, p.modified_secs, p.width, p.height,
     p.orientation, p.date_taken, p.format, p.caption, p.hidden, p.rating";

/// Seconds since the Unix epoch for the start of a calendar year/month.
fn days_to_unix(year: i32, month: u8) -> i64 {
    const DAYS_PER_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    fn is_leap(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    // Whole years since the epoch, counting each year's real length.
    let mut days = 0i64;
    for y in 1970..year as i64 {
        days += if is_leap(y) { 366 } else { 365 };
    }

    // Then the months already elapsed in the target year.
    let leap = is_leap(year as i64);
    let elapsed = month.min(12) as usize;
    for (month_index, days_in_month) in DAYS_PER_MONTH.iter().take(elapsed).enumerate() {
        days += days_in_month + i64::from(month_index == 1 && leap);
    }

    days * 86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seeds a photo so membership, ratings and dates have something to read.
    fn seed(db: &crate::Database, path: &str, modified: i64, rating: u8, taken: Option<i64>) {
        db.upsert_folder("C:/pics").expect("folder");
        db.upsert_photo_facts(path, Some(1), 10, modified, Some("jpg"))
            .expect("facts");
        db.set_photo_rating(path, rating).expect("rating");
        if let Some(taken) = taken {
            db.set_photo_image_info(path, 1920, 1080, 1, Some(taken))
                .expect("info");
        }
    }

    fn paths(photos: &[PhotoRow]) -> Vec<&str> {
        photos.iter().map(|p| p.path.as_str()).collect()
    }

    #[test]
    fn a_manual_album_reports_its_kind() {
        let db = crate::Database::open_in_memory().expect("db");
        db.create_album("Wedding").expect("create");
        let albums = db.list_albums().expect("list");
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].name, "Wedding");
        assert_eq!(albums[0].kind, "manual");
        assert_eq!(albums[0].photo_count, 0);
    }

    #[test]
    fn a_folder_album_remembers_the_folder_path() {
        let db = crate::Database::open_in_memory().expect("db");
        db.create_folder_album("Camera", "C:/pics").expect("create");
        let albums = db.list_albums().expect("list");
        assert_eq!(albums[0].kind, "folder");
        assert_eq!(albums[0].param.as_deref(), Some("C:/pics"));
    }

    #[test]
    fn adding_photos_keeps_order_and_skips_unknown_paths() {
        let db = crate::Database::open_in_memory().expect("db");
        let album = db.create_album("Trip").expect("album");
        for name in ["a.jpg", "b.jpg"] {
            seed(&db, &format!("C:/pics/{name}"), 0, 0, None);
        }
        let added = db
            .add_photos(
                album,
                &[
                    "C:/pics/b.jpg".into(),
                    "C:/pics/missing.jpg".into(),
                    "C:/pics/a.jpg".into(),
                ],
            )
            .expect("add");
        assert_eq!(added, 2, "the unknown path is skipped, not an error");
        assert_eq!(
            paths(&db.album_photos(album).expect("photos")),
            ["C:/pics/b.jpg", "C:/pics/a.jpg"]
        );
    }

    #[test]
    fn reorder_moves_photos_forward_without_dropping_any() {
        let db = crate::Database::open_in_memory().expect("db");
        let album = db.create_album("Trip").expect("album");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            seed(&db, &format!("C:/pics/{name}"), 0, 0, None);
        }
        let all: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|n| format!("C:/pics/{n}.jpg"))
            .collect();
        db.add_photos(album, &all).expect("add");

        // Moving c and a to the front must not lose b: an album edit may only
        // reorder membership, never remove a photo.
        db.reorder(album, &[all[2].clone(), all[0].clone()])
            .expect("reorder");
        assert_eq!(
            paths(&db.album_photos(album).expect("photos")),
            ["C:/pics/c.jpg", "C:/pics/a.jpg", "C:/pics/b.jpg"]
        );
    }

    #[test]
    fn an_album_edit_never_touches_a_photo_row() {
        let db = crate::Database::open_in_memory().expect("db");
        let album = db.create_album("Trip").expect("album");
        for name in ["a.jpg", "b.jpg"] {
            seed(&db, &format!("C:/pics/{name}"), 0, 0, None);
        }
        db.add_photos(album, &["C:/pics/a.jpg".into(), "C:/pics/b.jpg".into()])
            .expect("add");

        db.remove_photo(album, "C:/pics/a.jpg").expect("remove");
        assert_eq!(
            db.count_photos().expect("count"),
            2,
            "photo rows survive an edit"
        );

        db.delete_album(album).expect("delete");
        assert_eq!(
            db.count_photos().expect("count"),
            2,
            "deleting an album keeps photos too"
        );
    }

    #[test]
    fn smart_recent_follows_modification_time() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/old.jpg", 100, 0, None);
        seed(&db, "C:/pics/new.jpg", 900, 0, None);
        let photos = db
            .smart_photos(&Smart::Recent { limit: 1 })
            .expect("recent");
        assert_eq!(paths(&photos), ["C:/pics/new.jpg"]);
    }

    #[test]
    fn smart_starred_respects_the_minimum_rating() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/dull.jpg", 0, 1, None);
        seed(&db, "C:/pics/bright.jpg", 0, 4, None);
        let photos = db
            .smart_photos(&Smart::Starred { min_rating: 3 })
            .expect("starred");
        assert_eq!(paths(&photos), ["C:/pics/bright.jpg"]);
    }

    #[test]
    fn smart_year_and_month_read_the_date_taken() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(
            &db,
            "C:/pics/summer.jpg",
            0,
            0,
            Some(days_to_unix(2024, 6) + 86_400),
        );
        seed(
            &db,
            "C:/pics/winter.jpg",
            0,
            0,
            Some(days_to_unix(2025, 0) + 86_400),
        );

        assert_eq!(
            paths(&db.smart_photos(&Smart::Year(2024)).expect("year")),
            ["C:/pics/summer.jpg"]
        );
        let months = db.smart_photos(&Smart::Month(7)).expect("month");
        assert_eq!(paths(&months), ["C:/pics/summer.jpg"]);
        assert!(
            db.smart_photos(&Smart::Month(2)).expect("feb").is_empty(),
            "no February photos"
        );
    }

    #[test]
    fn keywords_are_case_insensitive_and_filter_photos() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/a.jpg", 0, 0, None);
        seed(&db, "C:/pics/b.jpg", 0, 0, None);

        let tag = db.ensure_tag("holiday").expect("tag");
        assert_eq!(
            db.ensure_tag("HOLIDAY").expect("folded"),
            tag,
            "one tag, any case"
        );
        assert_eq!(db.list_tags().expect("tags").len(), 1, "no duplicate row");

        db.tag_photo("C:/pics/a.jpg", tag).expect("tag a");
        db.tag_photo("C:/pics/b.jpg", tag).expect("tag b");
        assert_eq!(
            paths(&db.photos_by_tag(tag).expect("photos")),
            ["C:/pics/a.jpg", "C:/pics/b.jpg"]
        );
    }

    #[test]
    fn album_details_round_trip_and_clearing_removes_them() {
        let db = crate::Database::open_in_memory().expect("db");
        let album = db.create_album("Trip").expect("album");

        db.set_album_detail(
            album,
            Some("Summer"),
            Some("Lisbon"),
            Some("C:/m.mp3"),
            Some(42),
        )
        .expect("save");
        let saved = db.list_albums().expect("albums").remove(0);
        assert_eq!(saved.description.as_deref(), Some("Summer"));
        assert_eq!(saved.place_taken.as_deref(), Some("Lisbon"));
        assert_eq!(saved.music_path.as_deref(), Some("C:/m.mp3"));
        assert_eq!(saved.date, Some(42));

        // Clearing a field must clear the column, not leave the old value.
        db.set_album_detail(album, None, Some("Lisbon"), None, None)
            .expect("clear");
        let cleared = db.list_albums().expect("albums").remove(0);
        assert_eq!(cleared.description, None);
        assert_eq!(cleared.music_path, None);
        assert_eq!(cleared.date, None);
        assert_eq!(cleared.place_taken.as_deref(), Some("Lisbon"));
    }
}
