//! Multi-resolution image preparation for Pinacoteca.
//!
//! The viewer must zoom and pan large photos (10+ megapixels) without a
//! perceptible hitch. Rather than re-decode or re-sample the full image on every
//! zoom change, this crate precomputes a [`Pyramid`] (see [`pyramid`]) whose
//! levels let the renderer pick a texture close to the on-screen size. Zoom then
//! becomes a level switch, not an image-wide resample.
//!
//! The GPU upload/sampling side lives in `pinacoteca-app` (it needs the window
//! and graphics device); this crate stays free of any graphics dependency so the
//! level arithmetic can be unit-tested headlessly.

pub mod pyramid;
pub mod thumbs;

pub use pyramid::Pyramid;
pub use thumbs::{RequestQueue, ThumbCache, ThumbKey, ThumbSize};
