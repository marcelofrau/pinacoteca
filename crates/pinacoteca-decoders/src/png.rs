//! PNG decoding via the `png` crate (see ADR-0001).

use std::path::Path;

use crate::{DecodeError, Decoder, Dimensions, RgbaImage};

/// Decodes PNG files.
#[derive(Debug, Default)]
pub struct PngDecoder;

impl PngDecoder {
    /// Creates a PNG decoder.
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for PngDecoder {
    fn name(&self) -> &'static str {
        "png"
    }

    fn supports(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    }

    fn dimensions(&self, path: &Path) -> Result<Dimensions, DecodeError> {
        let file = std::fs::File::open(path).map_err(DecodeError::Io)?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        let info = decoder
            .read_header_info()
            .map_err(|e| DecodeError::Malformed(e.to_string()))?;
        Ok(Dimensions {
            width: info.width,
            height: info.height,
        })
    }

    fn decode(&self, path: &Path) -> Result<RgbaImage, DecodeError> {
        let file = std::fs::File::open(path).map_err(DecodeError::Io)?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder
            .read_info()
            .map_err(|e| DecodeError::Malformed(e.to_string()))?;
        let mut buffer = vec![
            0u8;
            reader
                .output_buffer_size()
                .ok_or_else(|| DecodeError::Malformed("PNG too large".into()))?
        ];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|e| DecodeError::Malformed(e.to_string()))?;
        let (width, height) = (info.width, info.height);
        let pixel_count = width as usize * height as usize;

        let rgba = match info.color_type {
            png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => {
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for chunk in buffer[..info.buffer_size()].as_chunks::<3>().0 {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                rgba
            }
            png::ColorType::Grayscale => buffer[..info.buffer_size()]
                .iter()
                .flat_map(|&v| [v, v, v, 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buffer[..info.buffer_size()]
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|c| [c[0], c[0], c[0], c[1]])
                .collect(),
            png::ColorType::Indexed => {
                return Err(DecodeError::Malformed(
                    "indexed PNG was not expanded to colour".into(),
                ));
            }
        };

        RgbaImage::new(width, height, rgba)
            .ok_or_else(|| DecodeError::Malformed("buffer does not match dimensions".into()))
    }
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
    fn supports_png_case_insensitively() {
        let decoder = PngDecoder::new();
        assert!(decoder.supports(Path::new("a.png")));
        assert!(decoder.supports(Path::new("a.PNG")));
        assert!(!decoder.supports(Path::new("a.jpg")));
    }

    #[test]
    fn reads_dimensions_and_decodes_rgba() {
        let decoder = PngDecoder::new();
        let dimensions = decoder.dimensions(&fixture("tiny.png")).unwrap();
        assert_eq!((dimensions.width, dimensions.height), (4, 3));

        let image = decoder.decode(&fixture("tiny.png")).unwrap();
        assert_eq!((image.width, image.height), (4, 3));
        assert_eq!(image.pixels.len(), 4 * 3 * 4);
        assert!(image.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }
}
