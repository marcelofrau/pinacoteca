//! Catalog persistence: folders, photos and user state.
//!
//! A scan writes only *facts about files*; user state (rating, caption, album
//! membership, edits) is never clobbered by re-scanning. Photo facts are updated
//! in place, keyed by the unique path.

use crate::{Database, DbError};

/// A photo's catalog row as read back from the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoRow {
    /// Row id.
    pub id: i64,
    /// Absolute path.
    pub path: String,
    /// Owning folder id.
    pub folder_id: Option<i64>,
    /// File size in bytes.
    pub file_size: Option<i64>,
    /// Modification time, seconds since the Unix epoch.
    pub modified_secs: Option<i64>,
    /// Pixel width.
    pub width: Option<u32>,
    /// Pixel height.
    pub height: Option<u32>,
    /// EXIF orientation, 1..=8.
    pub orientation: u16,
    /// Date taken, seconds since the Unix epoch.
    pub date_taken: Option<i64>,
    /// Container format, e.g. `jpg`.
    pub format: Option<String>,
    /// User caption.
    pub caption: Option<String>,
    /// Whether the photo is hidden from standard views.
    pub hidden: bool,
    /// Star rating.
    pub rating: u8,
}

impl PhotoRow {
    /// Reads a [`PhotoRow`] from a statement selecting its fields in order.
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            path: row.get(1)?,
            folder_id: row.get(2)?,
            file_size: row.get(3)?,
            modified_secs: row.get(4)?,
            width: row.get(5)?,
            height: row.get(6)?,
            orientation: row.get(7)?,
            date_taken: row.get(8)?,
            format: row.get(9)?,
            caption: row.get(10)?,
            hidden: row.get(11)?,
            rating: row.get(12)?,
        })
    }
}

impl Database {
    /// Inserts a folder if needed and returns its id.
    pub fn upsert_folder(&self, path: &str) -> Result<i64, DbError> {
        self.connection().execute(
            "INSERT INTO folders (path) VALUES (?1) ON CONFLICT (path) DO NOTHING",
            [path],
        )?;
        let id = self.connection().query_row(
            "SELECT id FROM folders WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Inserts or updates a photo's file facts, keyed by path.
    ///
    /// Only facts about the file are written; user state (rating, caption,
    /// hidden, membership, edits) is preserved on update.
    pub fn upsert_photo_facts(
        &self,
        path: &str,
        folder_id: Option<i64>,
        file_size: i64,
        modified_secs: i64,
        format: Option<&str>,
    ) -> Result<i64, DbError> {
        self.connection().execute(
            "INSERT INTO photos (path, folder_id, file_size, modified_secs, format)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (path) DO UPDATE SET
                 folder_id = excluded.folder_id,
                 file_size = excluded.file_size,
                 modified_secs = excluded.modified_secs,
                 format = excluded.format",
            rusqlite::params![path, folder_id, file_size, modified_secs, format],
        )?;
        let id = self.connection().query_row(
            "SELECT id FROM photos WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Records decoded image information for a photo.
    pub fn set_photo_image_info(
        &self,
        path: &str,
        width: u32,
        height: u32,
        orientation: u16,
        date_taken: Option<i64>,
    ) -> Result<(), DbError> {
        self.connection().execute(
            "UPDATE photos
             SET width = ?2, height = ?3, orientation = ?4, date_taken = ?5
             WHERE path = ?1",
            rusqlite::params![path, width, height, orientation, date_taken],
        )?;
        Ok(())
    }

    /// Sets a photo's star rating (user state).
    pub fn set_photo_rating(&self, path: &str, rating: u8) -> Result<(), DbError> {
        self.connection().execute(
            "UPDATE photos SET rating = ?2 WHERE path = ?1",
            rusqlite::params![path, rating],
        )?;
        Ok(())
    }

    /// Sets a photo's caption (user state).
    pub fn set_photo_caption(&self, path: &str, caption: &str) -> Result<(), DbError> {
        self.connection().execute(
            "UPDATE photos SET caption = ?2 WHERE path = ?1",
            rusqlite::params![path, caption],
        )?;
        Ok(())
    }

    /// Number of photos in the catalog.
    /// Every stored rating, keyed by photo path.
    ///
    /// Used once after a scan to paint the stars on the grid without a query
    /// per cell.
    pub fn ratings_by_path(&self) -> Result<std::collections::HashMap<String, u8>, DbError> {
        let mut statement = self
            .conn
            .prepare("SELECT path, rating FROM photos WHERE rating > 0")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u8>(1)?))
        })?;
        let mut ratings = std::collections::HashMap::new();
        for row in rows {
            let (path, rating) = row?;
            ratings.insert(path, rating);
        }
        Ok(ratings)
    }

    pub fn count_photos(&self) -> Result<i64, DbError> {
        Ok(self
            .connection()
            .query_row("SELECT COUNT(*) FROM photos", [], |row| row.get(0))?)
    }

    /// Reads one photo by path.
    pub fn photo_by_path(&self, path: &str) -> Result<Option<PhotoRow>, DbError> {
        let mut statement = self.connection().prepare(
            "SELECT id, path, folder_id, file_size, modified_secs, width, height,
                    orientation, date_taken, format, caption, hidden, rating
             FROM photos WHERE path = ?1",
        )?;
        let mut rows = statement.query([path])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_photo(row)?)),
            None => Ok(None),
        }
    }

    /// Lists photos in a folder, ordered by path.
    pub fn photos_in_folder(&self, folder_id: i64) -> Result<Vec<PhotoRow>, DbError> {
        let mut statement = self.connection().prepare(
            "SELECT id, path, folder_id, file_size, modified_secs, width, height,
                    orientation, date_taken, format, caption, hidden, rating
             FROM photos WHERE folder_id = ?1 ORDER BY path",
        )?;
        let mut rows = statement.query([folder_id])?;
        let mut photos = Vec::new();
        while let Some(row) = rows.next()? {
            photos.push(row_to_photo(row)?);
        }
        Ok(photos)
    }
}

fn row_to_photo(row: &rusqlite::Row<'_>) -> Result<PhotoRow, rusqlite::Error> {
    Ok(PhotoRow {
        id: row.get(0)?,
        path: row.get(1)?,
        folder_id: row.get(2)?,
        file_size: row.get(3)?,
        modified_secs: row.get(4)?,
        width: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
        height: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
        orientation: row.get::<_, i64>(7)? as u16,
        date_taken: row.get(8)?,
        format: row.get(9)?,
        caption: row.get(10)?,
        hidden: row.get::<_, i64>(11)? != 0,
        rating: row.get::<_, i64>(12)? as u8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_upsert_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        let first = db.upsert_folder("photos/trip").unwrap();
        let second = db.upsert_folder("photos/trip").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn scanning_facts_do_not_clobber_user_state() {
        let db = Database::open_in_memory().unwrap();
        let folder = db.upsert_folder("photos").unwrap();
        db.upsert_photo_facts("photos/a.jpg", Some(folder), 100, 1000, Some("jpg"))
            .unwrap();
        db.set_photo_rating("photos/a.jpg", 5).unwrap();
        db.set_photo_caption("photos/a.jpg", "sunset").unwrap();

        // Re-scan with a new size/mtime.
        db.upsert_photo_facts("photos/a.jpg", Some(folder), 200, 2000, Some("jpg"))
            .unwrap();

        let photo = db.photo_by_path("photos/a.jpg").unwrap().unwrap();
        assert_eq!(photo.file_size, Some(200));
        assert_eq!(photo.modified_secs, Some(2000));
        assert_eq!(photo.rating, 5, "rating must survive a re-scan");
        assert_eq!(photo.caption.as_deref(), Some("sunset"));
    }

    #[test]
    fn image_info_and_queries_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let folder = db.upsert_folder("photos").unwrap();
        db.upsert_photo_facts("photos/a.jpg", Some(folder), 10, 20, Some("jpg"))
            .unwrap();
        db.upsert_photo_facts("photos/b.png", Some(folder), 11, 21, Some("png"))
            .unwrap();
        db.set_photo_image_info("photos/a.jpg", 4000, 3000, 6, Some(1699999000))
            .unwrap();

        assert_eq!(db.count_photos().unwrap(), 2);

        let a = db.photo_by_path("photos/a.jpg").unwrap().unwrap();
        assert_eq!(a.width, Some(4000));
        assert_eq!(a.height, Some(3000));
        assert_eq!(a.orientation, 6);
        assert_eq!(a.date_taken, Some(1699999000));

        let listed = db.photos_in_folder(folder).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].path, "photos/a.jpg");
        assert!(db.photo_by_path("nope.jpg").unwrap().is_none());
    }

    #[test]
    fn orientation_defaults_to_upright() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_photo_facts("photos/a.jpg", None, 1, 1, Some("jpg"))
            .unwrap();
        let photo = db.photo_by_path("photos/a.jpg").unwrap().unwrap();
        assert_eq!(photo.orientation, 1);
        assert!(!photo.hidden);
        assert_eq!(photo.rating, 0);
    }
}
