//! Thumbnail cache and request scheduling.
//!
//! The grid asks for a small image for every photo in view, thousands of times
//! over. This module models the two pieces that keep that cheap:
//!
//! - a content-keyed **on-disk cache** ([`ThumbCache`]) so a thumbnail is decoded
//!   once, ever, and survives restarts; and
//! - a **deduplicating priority queue** ([`RequestQueue`]) so the same cell is
//!   never decoded twice and the most visible cells are served first.
//!
//! The threaded worker pool that drains the queue lives in `pinacoteca-app` (it
//! needs the runtime); the cache layout and scheduling rules here are pure logic
//! and unit-tested.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Target size class for a cached thumbnail.
///
/// Each class is a single "longest side" budget. A thumbnail is generated at the
/// smallest class that is at least as large as the cell that needs it, then
/// scaled by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbSize {
    /// Filmstrip and small strips.
    Small,
    /// The grid's normal cell size.
    Normal,
    /// Lightbox and large cells.
    Large,
    /// Fitted full-screen preview (not full resolution).
    Preview,
}

impl ThumbSize {
    /// Every class, smallest first.
    pub const ALL: [ThumbSize; 4] = [
        ThumbSize::Small,
        ThumbSize::Normal,
        ThumbSize::Large,
        ThumbSize::Preview,
    ];

    /// Longest-side budget in pixels.
    pub fn pixels(self) -> u32 {
        match self {
            ThumbSize::Small => 128,
            ThumbSize::Normal => 256,
            ThumbSize::Large => 512,
            ThumbSize::Preview => 1600,
        }
    }

    /// Stable subdirectory name, also the cache namespace.
    pub fn dir_name(self) -> &'static str {
        match self {
            ThumbSize::Small => "small",
            ThumbSize::Normal => "normal",
            ThumbSize::Large => "large",
            ThumbSize::Preview => "preview",
        }
    }

    /// Smallest class whose budget fits a cell of `pixels` on its longest side.
    pub fn smallest_fitting(pixels: u32) -> ThumbSize {
        *Self::ALL
            .iter()
            .find(|size| size.pixels() >= pixels)
            .unwrap_or(&ThumbSize::Preview)
    }
}

/// The parts of a source file that identify its current content, for invalidation.
///
/// Matching on size + modification time is the usual cheap proxy: an edited or
/// re-exported photo differs in at least one of these, so its old thumbnails are
/// correctly ignored without hashing the whole file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFingerprint {
    /// File size in bytes.
    pub len: u64,
    /// Modification time, seconds since the Unix epoch.
    pub modified_secs: i64,
    /// Modification time, nanosecond part.
    pub modified_nanos: u32,
}

impl SourceFingerprint {
    /// Reads the fingerprint of a file from its metadata.
    pub fn read(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .unwrap_or_default();
        Ok(Self {
            len: meta.len(),
            modified_secs: modified.as_secs() as i64,
            modified_nanos: modified.subsec_nanos(),
        })
    }
}

/// Identity of one cached thumbnail: a source photo at a given content state and
/// size class.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbKey {
    /// Path of the source photo.
    pub source: PathBuf,
    /// Content state of the source photo.
    pub fingerprint: SourceFingerprint,
    /// Size class.
    pub size: ThumbSize,
}

impl ThumbKey {
    /// Stable file name for this key, derived from a deterministic FNV-1a hash.
    ///
    /// A stable hash matters: `std`'s `DefaultHasher` is not guaranteed to be
    /// stable across Rust versions, which would silently invalidate every
    /// cached thumbnail on a toolchain upgrade.
    pub fn file_name(&self) -> String {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.source.to_string_lossy().as_bytes());
        bytes.extend_from_slice(&self.fingerprint.len.to_le_bytes());
        bytes.extend_from_slice(&self.fingerprint.modified_secs.to_le_bytes());
        bytes.extend_from_slice(&self.fingerprint.modified_nanos.to_le_bytes());
        bytes.extend_from_slice(self.size.dir_name().as_bytes());
        format!("{:016x}.jpg", fnv1a(&bytes))
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// A content-addressed on-disk thumbnail cache.
#[derive(Debug, Clone)]
pub struct ThumbCache {
    root: PathBuf,
}

impl ThumbCache {
    /// Roots the cache at `root`, e.g. `%LOCALAPPDATA%\Pinacoteca\cache\thumbs`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Where the thumbnail for `key` is stored on disk.
    pub fn path_for(&self, key: &ThumbKey) -> PathBuf {
        self.root.join(key.size.dir_name()).join(key.file_name())
    }

    /// Whether a thumbnail for `key` is already cached.
    pub fn contains(&self, key: &ThumbKey) -> bool {
        self.path_for(key).is_file()
    }

    /// Stores a thumbnail (encoded bytes) for `key`, creating directories.
    pub fn write(&self, key: &ThumbKey, bytes: &[u8]) -> io::Result<()> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)
    }

    /// Reads a cached thumbnail, or `None` if absent.
    pub fn read(&self, key: &ThumbKey) -> io::Result<Option<Vec<u8>>> {
        match fs::read(self.path_for(key)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// A scheduled piece of thumbnail work. Lower [`Request::priority`] is more
/// urgent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The thumbnail to produce.
    pub key: ThumbKey,
    /// Urgency: 0 is most urgent (e.g. the cell nearest the viewport centre).
    pub priority: u32,
}

/// A deduplicating priority queue of thumbnail work.
///
/// A cell can be re-requested many times while scrolling (same cell, different
/// rows, hover changes); the queue collapses those into one entry and keeps the
/// best priority seen, so no work is ever queued twice.
pub struct RequestQueue {
    requests: Vec<Request>,
}

impl Default for RequestQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestQueue {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Enqueues a request, or improves the priority of an identical one.
    ///
    /// Returns `true` if the request was new.
    pub fn request(&mut self, key: ThumbKey, priority: u32) -> bool {
        if let Some(existing) = self.requests.iter_mut().find(|r| r.key == key) {
            if priority < existing.priority {
                existing.priority = priority;
            }
            return false;
        }
        self.requests.push(Request { key, priority });
        true
    }

    /// Removes the most urgent request.
    pub fn pop(&mut self) -> Option<Request> {
        let best = self
            .requests
            .iter()
            .enumerate()
            .min_by_key(|(_, request)| request.priority)?;
        Some(self.requests.swap_remove(best.0))
    }

    /// Drops every request whose key is not in `keep` (the currently visible
    /// set), so work scrolled away is not performed.
    pub fn forget_missing(&mut self, keep: &[ThumbKey]) {
        self.requests.retain(|request| keep.contains(&request.key));
    }

    /// Number of pending requests.
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Whether nothing is pending.
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

/// JPEG quality used for generated thumbnails.
pub const THUMB_JPEG_QUALITY: u8 = 82;

/// A thumbnail that was produced and stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedThumb {
    /// The encoded JPEG bytes that were written to the cache.
    pub bytes: Vec<u8>,
    /// Stored width in pixels.
    pub width: u32,
    /// Stored height in pixels.
    pub height: u32,
}

/// Errors from thumbnail generation.
#[derive(Debug)]
pub enum ThumbError {
    /// The cache could not be read or written.
    Io(std::io::Error),
    /// The source image could not be decoded.
    Decode(pinacoteca_decoders::DecodeError),
    /// The thumbnail could not be encoded.
    Encode(String),
}

impl std::fmt::Display for ThumbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbError::Io(e) => write!(f, "thumbnail cache I/O error: {e}"),
            ThumbError::Decode(e) => write!(f, "thumbnail decode error: {e}"),
            ThumbError::Encode(m) => write!(f, "thumbnail encode error: {m}"),
        }
    }
}

impl std::error::Error for ThumbError {}

impl From<std::io::Error> for ThumbError {
    fn from(value: std::io::Error) -> Self {
        ThumbError::Io(value)
    }
}

/// Produces and stores the thumbnail for `key`, returning what was written.
///
/// The cheap path is tried first: when the source is a JPEG carrying an embedded
/// EXIF thumbnail that is already large enough for the requested size class, that
/// thumbnail is used instead of decoding the full image (ADR-0003). Otherwise the
/// full image is decoded, oriented and downscaled. Either way the result is
/// EXIF-oriented so the grid shows upright thumbnails.
pub fn generate(
    cache: &ThumbCache,
    key: &ThumbKey,
    decoder: &dyn pinacoteca_decoders::Decoder,
    orientation: pinacoteca_decoders::Orientation,
) -> Result<GeneratedThumb, ThumbError> {
    let target = key.size.pixels();

    let image = match embedded_source_image(&key.source, decoder, target) {
        Some(image) => image,
        None => decoder.decode(&key.source).map_err(ThumbError::Decode)?,
    };

    let image = crate::imageops::apply_orientation(&image, orientation);
    let image = crate::imageops::resize_to_fit(&image, target);
    let bytes = encode_jpeg(&image, THUMB_JPEG_QUALITY)?;
    cache.write(key, &bytes)?;

    Ok(GeneratedThumb {
        bytes,
        width: image.width,
        height: image.height,
    })
}

/// Returns the source pixels to use, preferring an embedded EXIF thumbnail when
/// it is present and at least `target` pixels on its longest side.
fn embedded_source_image(
    source: &Path,
    decoder: &dyn pinacoteca_decoders::Decoder,
    target: u32,
) -> Option<pinacoteca_decoders::RgbaImage> {
    // Only JPEGs carry an embedded JPEG thumbnail.
    if decoder.name() != "jpeg" {
        return None;
    }
    let bytes = pinacoteca_decoders::exif::embedded_thumbnail_file(source)
        .ok()
        .flatten()?;
    let dimensions = pinacoteca_decoders::jpeg::dimensions_bytes(&bytes).ok()?;
    if dimensions.width.max(dimensions.height) < target {
        return None;
    }
    pinacoteca_decoders::jpeg::decode_bytes(&bytes).ok()
}

/// Encodes an RGBA image as a baseline JPEG.
fn encode_jpeg(image: &pinacoteca_decoders::RgbaImage, quality: u8) -> Result<Vec<u8>, ThumbError> {
    let mut out = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut out, quality);
    encoder
        .encode(
            &image.pixels,
            image.width as u16,
            image.height as u16,
            jpeg_encoder::ColorType::Rgba,
        )
        .map_err(|e| ThumbError::Encode(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(size: ThumbSize) -> ThumbKey {
        ThumbKey {
            source: PathBuf::from("photos/a.jpg"),
            fingerprint: SourceFingerprint {
                len: 123,
                modified_secs: 1_700_000_000,
                modified_nanos: 0,
            },
            size,
        }
    }

    fn key_named(name: &str) -> ThumbKey {
        ThumbKey {
            source: PathBuf::from(name),
            fingerprint: SourceFingerprint {
                len: 1,
                modified_secs: 0,
                modified_nanos: 0,
            },
            size: ThumbSize::Normal,
        }
    }

    #[test]
    fn size_classes_have_distinct_budgets() {
        assert_eq!(ThumbSize::Small.pixels(), 128);
        assert_eq!(ThumbSize::Normal.pixels(), 256);
        assert_eq!(ThumbSize::Large.pixels(), 512);
        assert_eq!(ThumbSize::Preview.pixels(), 1600);
    }

    #[test]
    fn smallest_fitting_picks_the_first_sufficient_class() {
        assert_eq!(ThumbSize::smallest_fitting(96), ThumbSize::Small);
        assert_eq!(ThumbSize::smallest_fitting(256), ThumbSize::Normal);
        assert_eq!(ThumbSize::smallest_fitting(300), ThumbSize::Large);
        assert_eq!(ThumbSize::smallest_fitting(9999), ThumbSize::Preview);
    }

    #[test]
    fn key_names_are_stable_and_differ_by_state() {
        let a = key(ThumbSize::Normal);
        assert_eq!(a.file_name(), key(ThumbSize::Normal).file_name());
        assert_ne!(a.file_name(), key(ThumbSize::Large).file_name());

        let mut changed = key(ThumbSize::Normal);
        changed.fingerprint.len = 999;
        assert_ne!(a.file_name(), changed.file_name());
    }

    #[test]
    fn cache_write_read_roundtrip() {
        let root =
            std::env::temp_dir().join(format!("pinacoteca-thumbcache-test-{}", std::process::id()));
        let cache = ThumbCache::new(&root);
        let key = key(ThumbSize::Normal);
        assert!(!cache.contains(&key));
        cache.write(&key, b"encoded-thumb").unwrap();
        assert!(cache.contains(&key));
        assert_eq!(cache.read(&key).unwrap().unwrap(), b"encoded-thumb");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn different_state_gets_a_different_path() {
        let cache = ThumbCache::new(std::env::temp_dir());
        let old = key(ThumbSize::Normal);
        let mut new = key(ThumbSize::Normal);
        new.fingerprint.modified_secs += 1;
        assert_ne!(cache.path_for(&old), cache.path_for(&new));
    }

    #[test]
    fn queue_deduplicates_by_key_keeping_best_priority() {
        let mut queue = RequestQueue::new();
        assert!(queue.request(key_named("a"), 10));
        assert!(!queue.request(key_named("a"), 4));
        assert!(!queue.request(key_named("a"), 9));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap().priority, 4);
    }

    #[test]
    fn queue_serves_most_urgent_first() {
        let mut queue = RequestQueue::new();
        queue.request(key_named("far"), 50);
        queue.request(key_named("near"), 1);
        queue.request(key_named("mid"), 20);
        assert_eq!(queue.pop().unwrap().key.source, Path::new("near"));
        assert_eq!(queue.pop().unwrap().key.source, Path::new("mid"));
        assert_eq!(queue.pop().unwrap().key.source, Path::new("far"));
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_forgets_requests_that_left_the_visible_set() {
        let mut queue = RequestQueue::new();
        let visible = key_named("visible");
        let scrolled = key_named("scrolled");
        queue.request(visible.clone(), 1);
        queue.request(scrolled, 2);
        queue.forget_missing(&[visible]);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap().key.source, Path::new("visible"));
    }

    #[test]
    fn fingerprint_reads_real_file_metadata() {
        let path =
            std::env::temp_dir().join(format!("pinacoteca-fp-test-{}.jpg", std::process::id()));
        fs::write(&path, b"hello").unwrap();
        let fingerprint = SourceFingerprint::read(&path).unwrap();
        assert_eq!(fingerprint.len, 5);
        assert!(fingerprint.modified_secs > 0);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn generate_downscales_orients_and_caches() {
        // A 600x400 source, so resizing to Normal (256) is exercised.
        let mut pixels = vec![0u8; 600 * 400 * 4];
        for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel.copy_from_slice(&[(index % 251) as u8, 90, 30, 255]);
        }
        let source = pinacoteca_decoders::RgbaImage::new(600, 400, pixels).unwrap();
        let encoded = encode_jpeg(&source, THUMB_JPEG_QUALITY).unwrap();

        let dir = std::env::temp_dir().join(format!("pinacoteca-gen-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("source.jpg");
        fs::write(&source_path, &encoded).unwrap();

        let cache = ThumbCache::new(dir.join("cache"));
        let key = ThumbKey {
            source: source_path.clone(),
            fingerprint: SourceFingerprint::read(&source_path).unwrap(),
            size: ThumbSize::Normal,
        };

        let decoder = pinacoteca_decoders::JpegDecoder::new();
        let generated = generate(
            &cache,
            &key,
            &decoder,
            pinacoteca_decoders::Orientation::Upright,
        )
        .unwrap();

        assert!(cache.contains(&key));
        assert_eq!((generated.width, generated.height), (256, 171));
        assert_eq!(cache.read(&key).unwrap().unwrap(), generated.bytes);

        // The stored bytes must be a decodable JPEG at the expected size.
        let stored = pinacoteca_decoders::jpeg::dimensions_bytes(&generated.bytes).unwrap();
        assert_eq!((stored.width, stored.height), (256, 171));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn generate_applies_exif_orientation() {
        let mut pixels = vec![0u8; 40 * 20 * 4];
        pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .for_each(|p| p.copy_from_slice(&[10, 20, 30, 255]));
        let source = pinacoteca_decoders::RgbaImage::new(40, 20, pixels).unwrap();
        let encoded = encode_jpeg(&source, THUMB_JPEG_QUALITY).unwrap();

        let dir =
            std::env::temp_dir().join(format!("pinacoteca-orient-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("source.jpg");
        fs::write(&source_path, &encoded).unwrap();

        let cache = ThumbCache::new(dir.join("cache"));
        let key = ThumbKey {
            source: source_path.clone(),
            fingerprint: SourceFingerprint::read(&source_path).unwrap(),
            size: ThumbSize::Normal,
        };

        let decoder = pinacoteca_decoders::JpegDecoder::new();
        // Orientation 6 rotates 90° CW, so 40x20 becomes 20x40.
        let generated = generate(
            &cache,
            &key,
            &decoder,
            pinacoteca_decoders::Orientation::Rotate90Cw,
        )
        .unwrap();
        assert_eq!((generated.width, generated.height), (20, 40));

        let _ = fs::remove_dir_all(&dir);
    }
}
