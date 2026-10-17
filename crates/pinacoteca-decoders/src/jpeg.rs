//! JPEG decoding via `zune-jpeg` (see ADR-0001).

use std::path::Path;

use crate::{DecodeError, Decoder, Dimensions, RgbaImage};

/// Decodes baseline and progressive JPEG files.
#[derive(Debug, Default)]
pub struct JpegDecoder;

impl JpegDecoder {
    /// Creates a JPEG decoder.
    pub fn new() -> Self {
        Self
    }
}

fn supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let ext = ext.to_ascii_lowercase();
            ext == "jpg" || ext == "jpeg"
        })
}

impl Decoder for JpegDecoder {
    fn name(&self) -> &'static str {
        "jpeg"
    }

    fn supports(&self, path: &Path) -> bool {
        supported_extension(path)
    }

    fn dimensions(&self, path: &Path) -> Result<Dimensions, DecodeError> {
        let file = std::fs::File::open(path).map_err(DecodeError::Io)?;
        let mut decoder = zune_jpeg::JpegDecoder::new(std::io::BufReader::new(file));
        decoder
            .decode_headers()
            .map_err(|e| DecodeError::Malformed(format!("{e:?}")))?;
        let info = decoder
            .info()
            .ok_or_else(|| DecodeError::Malformed("missing header info".into()))?;
        Ok(Dimensions {
            width: info.width as u32,
            height: info.height as u32,
        })
    }

    fn decode(&self, path: &Path) -> Result<RgbaImage, DecodeError> {
        let file = std::fs::File::open(path).map_err(DecodeError::Io)?;
        let mut decoder = zune_jpeg::JpegDecoder::new(std::io::BufReader::new(file));
        let rgb = decoder
            .decode()
            .map_err(|e| DecodeError::Malformed(format!("{e:?}")))?;
        let info = decoder
            .info()
            .ok_or_else(|| DecodeError::Malformed("missing header info".into()))?;
        let width = info.width as u32;
        let height = info.height as u32;

        to_rgba(rgb, width, height)
    }
}

/// Promotes a decoded buffer (RGB, gray, or already RGBA) to RGBA.
fn to_rgba(buffer: Vec<u8>, width: u32, height: u32) -> Result<RgbaImage, DecodeError> {
    let pixel_count = width as usize * height as usize;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    match buffer.len() {
        n if n == pixel_count * 3 => {
            for chunk in buffer.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        n if n == pixel_count => {
            for &value in &buffer {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        n if n == pixel_count * 4 => {
            return Ok(RgbaImage {
                width,
                height,
                pixels: buffer,
            });
        }
        n => {
            return Err(DecodeError::Malformed(format!(
                "unexpected buffer size {n} for {width}x{height}"
            )));
        }
    }
    RgbaImage::new(width, height, rgba)
        .ok_or_else(|| DecodeError::Malformed("buffer does not match dimensions".into()))
}

/// Reads JPEG dimensions from data already in memory.
pub fn dimensions_bytes(data: &[u8]) -> Result<Dimensions, DecodeError> {
    let mut decoder = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(data));
    decoder
        .decode_headers()
        .map_err(|e| DecodeError::Malformed(format!("{e:?}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| DecodeError::Malformed("missing header info".into()))?;
    Ok(Dimensions {
        width: info.width as u32,
        height: info.height as u32,
    })
}

/// Decodes JPEG data already in memory (e.g. an embedded EXIF thumbnail).
pub fn decode_bytes(data: &[u8]) -> Result<RgbaImage, DecodeError> {
    let mut decoder = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(data));
    let buffer = decoder
        .decode()
        .map_err(|e| DecodeError::Malformed(format!("{e:?}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| DecodeError::Malformed("missing header info".into()))?;
    to_rgba(buffer, info.width as u32, info.height as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decoder;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn supports_jpeg_extensions_case_insensitively() {
        let decoder = JpegDecoder::new();
        assert!(decoder.supports(Path::new("a.jpg")));
        assert!(decoder.supports(Path::new("a.JPEG")));
        assert!(!decoder.supports(Path::new("a.png")));
    }

    #[test]
    fn reads_dimensions_without_decoding_pixels() {
        let decoder = JpegDecoder::new();
        let dimensions = decoder.dimensions(&fixture("tiny.jpg")).unwrap();
        assert_eq!((dimensions.width, dimensions.height), (4, 3));
    }

    #[test]
    fn decodes_to_rgba() {
        let decoder = JpegDecoder::new();
        let image = decoder.decode(&fixture("tiny.jpg")).unwrap();
        assert_eq!((image.width, image.height), (4, 3));
        assert_eq!(image.pixels.len(), 4 * 3 * 4);
        assert!(image.pixels.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let decoder = JpegDecoder::new();
        let error = decoder.decode(&fixture("does-not-exist.jpg")).unwrap_err();
        assert!(matches!(error, DecodeError::Io(_)));
    }
}
