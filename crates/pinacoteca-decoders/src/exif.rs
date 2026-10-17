//! EXIF orientation.
//!
//! Camera JPEGs carry an EXIF `Orientation` tag describing how the stored scan
//! data must be transformed before display. The viewer must honour it, otherwise
//! photos taken in portrait appear sideways. This module reads that tag; the
//! viewer applies the transform.

use std::path::Path;

/// EXIF orientation, values 1–8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// 1 — upright, no transform.
    Upright,
    /// 2 — mirrored horizontally.
    MirrorH,
    /// 3 — rotated 180°.
    Rotate180,
    /// 4 — mirrored vertically.
    MirrorV,
    /// 5 — transposed (mirror then rotate 90° CCW).
    Transpose,
    /// 6 — rotated 90° clockwise.
    Rotate90Cw,
    /// 6 — transverse (mirror then rotate 90° CW).
    Transverse,
    /// 8 — rotated 90° counter-clockwise (270° clockwise).
    Rotate270Cw,
}

impl Orientation {
    /// Maps a raw EXIF orientation value to an [`Orientation`], defaulting to
    /// upright for anything out of range.
    pub fn from_exif(value: u16) -> Orientation {
        match value {
            2 => Orientation::MirrorH,
            3 => Orientation::Rotate180,
            4 => Orientation::MirrorV,
            5 => Orientation::Transpose,
            6 => Orientation::Rotate90Cw,
            7 => Orientation::Transverse,
            8 => Orientation::Rotate270Cw,
            _ => Orientation::Upright,
        }
    }

    /// Whether the transform swaps width and height.
    pub fn swaps_dimensions(self) -> bool {
        matches!(
            self,
            Orientation::Transpose
                | Orientation::Rotate90Cw
                | Orientation::Transverse
                | Orientation::Rotate270Cw
        )
    }

    /// The dimensions the image has *after* this orientation is applied.
    pub fn oriented_dimensions(self, width: u32, height: u32) -> (u32, u32) {
        if self.swaps_dimensions() {
            (height, width)
        } else {
            (width, height)
        }
    }
}

/// Reads the EXIF orientation of a JPEG file.
///
/// Returns upright when the file carries no EXIF orientation tag.
pub fn read_orientation(path: &Path) -> std::io::Result<Orientation> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let exif = match exif::Reader::new().read_from_container(&mut reader) {
        Ok(exif) => exif,
        // No EXIF, or not a container we understand: treat as upright.
        Err(_) => return Ok(Orientation::Upright),
    };
    let value = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|field| field.value.get_uint(0));
    Ok(value
        .map(|v| Orientation::from_exif(v as u16))
        .unwrap_or(Orientation::Upright))
}
