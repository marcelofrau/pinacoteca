//! The photo catalog: identifiers, photos and ratings.

use std::path::PathBuf;

/// Stable identifier for a photo within the local catalog.
///
/// Identifiers are assigned when a photo is first indexed and are unaffected by
/// the file being moved or renamed externally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhotoId(pub u64);

/// A single photo in the local library.
///
/// The referenced file is treated as a read-only input. Pinacoteca never writes
/// to it; all user state is stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Photo {
    /// Catalog identifier.
    pub id: PhotoId,
    /// Location of the image file on disk.
    pub path: PathBuf,
    /// User rating, zero to five stars.
    pub rating: Rating,
}

impl Photo {
    /// Creates a photo with the given identity and path, unrated.
    pub fn new(id: PhotoId, path: impl Into<PathBuf>) -> Self {
        Self {
            id,
            path: path.into(),
            rating: Rating::default(),
        }
    }
}

/// A star rating from zero to five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Rating(u8);

impl Rating {
    /// Highest allowed rating, in stars.
    pub const MAX: u8 = 5;

    /// Creates a rating, returning `None` if `stars` is above [`Rating::MAX`].
    pub fn new(stars: u8) -> Option<Self> {
        (stars <= Self::MAX).then_some(Self(stars))
    }

    /// The rating in stars.
    pub fn stars(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_rejects_values_above_max() {
        assert!(Rating::new(Rating::MAX).is_some());
        assert!(Rating::new(Rating::MAX + 1).is_none());
    }

    #[test]
    fn rating_orders_by_stars() {
        assert!(Rating::new(1).unwrap() < Rating::new(4).unwrap());
    }

    #[test]
    fn new_photo_starts_unrated() {
        let photo = Photo::new(PhotoId(1), "photos/a.jpg");
        assert_eq!(photo.rating, Rating::default());
        assert_eq!(photo.rating.stars(), 0);
    }
}
