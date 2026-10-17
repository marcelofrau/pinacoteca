//! Domain model and logic for Pinacoteca.
//!
//! This crate is UI-agnostic and holds the concepts the application is built
//! around: the photo catalog, albums and tags, ratings, and the non-destructive
//! edit stack. It performs no rendering and does not depend on a UI framework.
//!
//! The original image files referenced by [`catalog::Photo`] are read-only
//! inputs. User state (albums, tags, ratings, edits) lives here and is persisted
//! by `pinacoteca-db`.

pub mod album;
pub mod catalog;
pub mod edit;
pub mod scan;

pub use album::{Album, AlbumId, AlbumKind};
pub use catalog::{Photo, PhotoId, Rating};
pub use edit::{EditOp, EditStack, Effect, Rotation};
pub use scan::{DiscoveredPhoto, ScanStats, scan_folder};
