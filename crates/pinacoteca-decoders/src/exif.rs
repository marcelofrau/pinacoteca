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

/// Extracts the JPEG thumbnail embedded in a JPEG's EXIF APP1 segment, if any.
///
/// Reusing this thumbnail is the cheap path a grid wants (~0.001 ms to locate vs
/// ~70 ms to decode; see ADR-0003). The caller is responsible for checking that
/// the result is large enough for its target size.
pub fn embedded_thumbnail(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }

    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        // Markers without a length payload.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        // Start of scan: metadata is over.
        if marker == 0xDA {
            break;
        }
        let segment_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if segment_len < 2 || i + 2 + segment_len > data.len() {
            return None;
        }
        if marker == 0xE1 {
            let segment = &data[i + 4..i + 2 + segment_len];
            if segment.len() >= 6 && &segment[..6] == b"Exif\0\0" {
                return parse_tiff_thumbnail(&segment[6..]);
            }
        }
        i += 2 + segment_len;
    }
    None
}

/// Reads a file and returns its embedded EXIF thumbnail, if any.
pub fn embedded_thumbnail_file(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    let data = std::fs::read(path)?;
    Ok(embedded_thumbnail(&data))
}

fn parse_tiff_thumbnail(tiff: &[u8]) -> Option<Vec<u8>> {
    if tiff.len() < 8 {
        return None;
    }
    let little_endian = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let read16 = |offset: usize| -> Option<u16> {
        let bytes = tiff.get(offset..offset + 2)?;
        Some(if little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        })
    };
    let read32 = |offset: usize| -> Option<u32> {
        let bytes = tiff.get(offset..offset + 4)?;
        Some(if little_endian {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        })
    };

    // IFD0 -> next-IFD offset -> IFD1 holds the thumbnail image.
    let ifd0 = read32(4)? as usize;
    let entries0 = read16(ifd0)? as usize;
    let ifd1 = read32(ifd0 + 2 + entries0 * 12)? as usize;
    if ifd1 == 0 {
        return None;
    }
    let entries1 = read16(ifd1)? as usize;

    let mut offset = None;
    let mut length = None;
    for entry in 0..entries1 {
        let entry_pos = ifd1 + 2 + entry * 12;
        let tag = read16(entry_pos)?;
        let field_type = read16(entry_pos + 2)?;
        // SHORT (3) value sits in the first two bytes of the value field.
        let value = if field_type == 3 {
            read16(entry_pos + 8)? as u32
        } else {
            read32(entry_pos + 8)?
        };
        match tag {
            0x0201 => offset = Some(value as usize),
            0x0202 => length = Some(value as usize),
            _ => {}
        }
    }

    let start = offset?;
    let end = start.checked_add(length?)?;
    let thumbnail = tiff.get(start..end)?;
    (thumbnail.len() >= 2 && thumbnail[0] == 0xFF && thumbnail[1] == 0xD8)
        .then(|| thumbnail.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn orientation_defaults_to_upright_without_exif() {
        let orientation = read_orientation(&fixture("tiny.jpg")).unwrap();
        assert_eq!(orientation, Orientation::Upright);
    }

    #[test]
    fn embedded_thumbnail_absent_in_plain_jpeg() {
        assert!(embedded_thumbnail_file(&fixture("tiny.jpg"))
            .unwrap()
            .is_none());
    }
}
