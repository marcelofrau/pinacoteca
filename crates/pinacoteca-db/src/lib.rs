//! SQLite persistence for Pinacoteca.
//!
//! The database holds the catalog and all user state. Original image files are
//! never referenced as writable and are never modified. Schema changes are
//! applied as ordered migrations; see [`migrate`].

use std::path::Path;

/// Errors produced by the persistence layer.
#[derive(Debug)]
pub enum DbError {
    /// The underlying SQLite operation failed.
    Sqlite(rusqlite::Error),
    /// A migration failed.
    Migration(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Sqlite(e) => write!(f, "database error: {e}"),
            DbError::Migration(m) => write!(f, "migration error: {m}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::Sqlite(e) => Some(e),
            DbError::Migration(_) => None,
        }
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(value: rusqlite::Error) -> Self {
        DbError::Sqlite(value)
    }
}

/// The highest schema version this build understands.
pub const SCHEMA_VERSION: u32 = 2;

/// Ordered schema migrations. Each entry moves the schema to version `index + 1`.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_catalog_metadata.sql"),
];

/// An open connection to a Pinacoteca database.
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    /// Opens (creating if needed) a database at `path` and applies migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let conn = rusqlite::Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// Opens a private in-memory database and applies migrations.
    ///
    /// Useful for tests and for ephemeral sessions.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: rusqlite::Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self { conn };
        migrate(&db.conn)?;
        Ok(db)
    }

    /// The current schema version recorded in the database.
    pub fn schema_version(&self) -> Result<u32, DbError> {
        let version: u32 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        Ok(version)
    }

    /// Borrows the underlying connection.
    pub fn connection(&self) -> &rusqlite::Connection {
        &self.conn
    }
}

/// Applies any pending migrations, advancing `user_version` to [`SCHEMA_VERSION`].
fn migrate(conn: &rusqlite::Connection) -> Result<(), DbError> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current > SCHEMA_VERSION {
        return Err(DbError::Migration(format!(
            "database schema version {current} is newer than supported version {SCHEMA_VERSION}"
        )));
    }

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let target = index as u32 + 1;
        if current < target {
            conn.execute_batch(sql).map_err(|e| {
                DbError::Migration(format!("applying migration {target} failed: {e}"))
            })?;
            conn.pragma_update(None, "user_version", target)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_is_migrated_to_current_version() {
        let db = Database::open_in_memory().expect("open in-memory database");
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn migration_creates_core_tables() {
        let db = Database::open_in_memory().unwrap();
        let count: i64 = db
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name IN
                 ('photos', 'albums', 'album_photos', 'tags', 'photo_tags',
                  'edit_ops', 'folders')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn photos_carry_catalog_metadata() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.connection();
        conn.execute("INSERT INTO folders (path) VALUES ('photos/trip')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO photos
             (path, rating, folder_id, file_size, modified_secs, width, height,
              orientation, date_taken, format, caption, hidden)
             VALUES ('photos/trip/a.jpg', 4, 1, 123456, 1700000000, 3624, 2910,
                     6, 1699999000, 'jpg', 'a caption', 0)",
            [],
        )
        .unwrap();

        let (rating, width, height, orientation, format, caption): (
            u8,
            u32,
            u32,
            u8,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT rating, width, height, orientation, format, caption
                 FROM photos WHERE path = 'photos/trip/a.jpg'",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as u8,
                        row.get::<_, i64>(1)? as u32,
                        row.get::<_, i64>(2)? as u32,
                        row.get::<_, i64>(3)? as u8,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(rating, 4);
        assert_eq!((width, height), (3624, 2910));
        assert_eq!(orientation, 6);
        assert_eq!(format, "jpg");
        assert_eq!(caption, "a caption");
    }

    #[test]
    fn albums_carry_detail_fields() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.connection();
        conn.execute(
            "INSERT INTO albums (name, kind, param, description, place_taken,
                                 music_path, date)
             VALUES ('Trip', 'manual', NULL, 'desc', 'Lisbon', 'music.mp3', 1700000000)",
            [],
        )
        .unwrap();

        let (description, place, music): (String, String, String) = conn
            .query_row(
                "SELECT description, place_taken, music_path FROM albums",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (description.as_str(), place.as_str(), music.as_str()),
            ("desc", "Lisbon", "music.mp3")
        );
    }

    #[test]
    fn schema_version_tracks_migrations() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(SCHEMA_VERSION, 2);
    }

    #[test]
    fn migration_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        migrate(db.connection()).expect("second migration run is a no-op");
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }
}
