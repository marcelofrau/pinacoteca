//! Albums and how their members are determined.

use std::path::PathBuf;

use crate::catalog::PhotoId;

/// Stable identifier for an album within the local catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AlbumId(pub u64);

/// How an album determines its members.
///
/// Albums are virtual: membership never moves, renames or rewrites files on
/// disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumKind {
    /// Members are chosen explicitly by the user.
    Manual,
    /// Members are the photos found in a folder on disk.
    Folder(PathBuf),
    /// Members are the result of a saved query (query language decided in M2).
    Smart(String),
}

/// A named collection of photos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Album {
    /// Catalog identifier.
    pub id: AlbumId,
    /// Display name.
    pub name: String,
    /// How membership is determined.
    pub kind: AlbumKind,
    /// Explicit membership. Meaningful only for [`AlbumKind::Manual`]; other
    /// kinds derive their members and keep this empty.
    pub members: Vec<PhotoId>,
}

impl Album {
    /// Creates an empty manual album.
    pub fn new_manual(id: AlbumId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: AlbumKind::Manual,
            members: Vec::new(),
        }
    }

    /// Whether this album derives its members rather than storing them.
    pub fn is_derived(&self) -> bool {
        !matches!(self.kind, AlbumKind::Manual)
    }

    /// Adds a photo to a manual album, ignoring duplicates.
    ///
    /// Returns `true` if the photo was added. Does nothing for derived albums.
    pub fn add(&mut self, photo: PhotoId) -> bool {
        if self.is_derived() || self.members.contains(&photo) {
            return false;
        }
        self.members.push(photo);
        true
    }

    /// Removes a photo from a manual album.
    ///
    /// Returns `true` if the photo was present.
    pub fn remove(&mut self, photo: PhotoId) -> bool {
        if let Some(index) = self.members.iter().position(|&m| m == photo) {
            self.members.remove(index);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_album_adds_without_duplicates() {
        let mut album = Album::new_manual(AlbumId(1), "Trip");
        assert!(album.add(PhotoId(1)));
        assert!(!album.add(PhotoId(1)));
        assert_eq!(album.members, vec![PhotoId(1)]);
    }

    #[test]
    fn manual_album_removes_member() {
        let mut album = Album::new_manual(AlbumId(1), "Trip");
        album.add(PhotoId(7));
        assert!(album.remove(PhotoId(7)));
        assert!(!album.remove(PhotoId(7)));
        assert!(album.members.is_empty());
    }

    #[test]
    fn derived_album_ignores_explicit_members() {
        let mut album = Album {
            id: AlbumId(2),
            name: "By date".into(),
            kind: AlbumKind::Smart("year:2020".into()),
            members: Vec::new(),
        };
        assert!(album.is_derived());
        assert!(!album.add(PhotoId(1)));
        assert!(album.members.is_empty());
    }
}
