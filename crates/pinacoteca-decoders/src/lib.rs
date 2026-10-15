//! Pluggable image decoding for Pinacoteca.
//!
//! Decoding sits behind the [`Decoder`] trait so implementations can be swapped
//! or added without touching the rest of the application. Concrete decoders
//! (pure-Rust, and optional RAW / HEIC packs) are selected in milestone M2 based
//! on benchmarks; this crate currently defines only the interface and the
//! registry that resolves a decoder for a given file.

use std::path::Path;

/// Pixel dimensions of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// A decoded image in 8-bit RGBA, row-major, tightly packed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixel data: `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// Creates an image, checking that the buffer length matches the dimensions.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        (pixels.len() == expected).then_some(Self {
            width,
            height,
            pixels,
        })
    }
}

/// Reasons a decoder can fail.
#[derive(Debug)]
pub enum DecodeError {
    /// No registered decoder claims the file.
    Unsupported,
    /// The file could not be read.
    Io(std::io::Error),
    /// The file is claimed by the decoder but is not valid for it.
    Malformed(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Unsupported => write!(f, "no decoder supports this file"),
            DecodeError::Io(e) => write!(f, "failed to read file: {e}"),
            DecodeError::Malformed(m) => write!(f, "malformed image: {m}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// A source of decoded image data for one or more formats.
pub trait Decoder: Send + Sync {
    /// Stable name of the decoder, for diagnostics.
    fn name(&self) -> &'static str;

    /// Whether this decoder can handle the file at `path`.
    ///
    /// Intended to be a cheap check (typically the file extension); actual
    /// validity is determined during decoding.
    fn supports(&self, path: &Path) -> bool;

    /// Reads the pixel dimensions without decoding the full image if possible.
    fn dimensions(&self, path: &Path) -> Result<Dimensions, DecodeError>;

    /// Decodes the full image.
    fn decode(&self, path: &Path) -> Result<RgbaImage, DecodeError>;
}

/// An ordered set of decoders used to resolve a decoder for a file.
///
/// The first registered decoder that [`Decoder::supports`] a path wins.
#[derive(Default)]
pub struct DecoderRegistry {
    decoders: Vec<Box<dyn Decoder>>,
}

impl DecoderRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a decoder at the end of the priority order.
    pub fn register(&mut self, decoder: Box<dyn Decoder>) {
        self.decoders.push(decoder);
    }

    /// Returns the first decoder that supports `path`, if any.
    pub fn decoder_for(&self, path: &Path) -> Option<&dyn Decoder> {
        self.decoders
            .iter()
            .find(|d| d.supports(path))
            .map(|d| d.as_ref())
    }

    /// Number of registered decoders.
    pub fn len(&self) -> usize {
        self.decoders.len()
    }

    /// Whether no decoders are registered.
    pub fn is_empty(&self) -> bool {
        self.decoders.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy(&'static str, &'static str);

    impl Decoder for Dummy {
        fn name(&self) -> &'static str {
            self.0
        }
        fn supports(&self, path: &Path) -> bool {
            path.extension().is_some_and(|e| e == self.1)
        }
        fn dimensions(&self, _path: &Path) -> Result<Dimensions, DecodeError> {
            Ok(Dimensions {
                width: 1,
                height: 1,
            })
        }
        fn decode(&self, _path: &Path) -> Result<RgbaImage, DecodeError> {
            RgbaImage::new(1, 1, vec![0, 0, 0, 255]).ok_or(DecodeError::Unsupported)
        }
    }

    #[test]
    fn registry_resolves_first_matching_decoder() {
        let mut registry = DecoderRegistry::new();
        registry.register(Box::new(Dummy("png", "png")));
        registry.register(Box::new(Dummy("jpg", "jpg")));

        assert_eq!(
            registry
                .decoder_for(Path::new("a/b.png"))
                .map(Decoder::name),
            Some("png")
        );
        assert!(registry.decoder_for(Path::new("a/b.gif")).is_none());
    }

    #[test]
    fn rgba_image_checks_buffer_length() {
        assert!(RgbaImage::new(2, 2, vec![0; 16]).is_some());
        assert!(RgbaImage::new(2, 2, vec![0; 15]).is_none());
    }
}
