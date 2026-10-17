//! Background workers.
//!
//! Scanning and thumbnail generation run off the UI thread (ADR-0005). The UI
//! polls for results each frame and never blocks. Cancellation is by generation
//! token: results carrying a stale token are discarded on arrival.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use pinacoteca_core::scan::{DiscoveredPhoto, scan_folder};
use pinacoteca_decoders::{Orientation, default_registry};
use pinacoteca_render::thumbs::{GeneratedThumb, ThumbCache, ThumbError, ThumbKey, generate};

/// Messages from a folder scan.
#[derive(Debug)]
pub enum ScanMessage {
    /// A photo was discovered.
    Photo(DiscoveredPhoto),
    /// The scan finished.
    Done {
        /// Number of photos found.
        photos: usize,
        /// Number of I/O errors encountered.
        errors: usize,
    },
}

/// A running folder scan.
pub struct ScanHandle {
    receiver: Receiver<ScanMessage>,
    generation: u64,
}

impl ScanHandle {
    /// Starts scanning `root` on a background thread.
    pub fn start(root: PathBuf, generation: u64) -> Self {
        let (sender, receiver) = channel();
        thread::spawn(move || {
            let mut send = |photo: DiscoveredPhoto| {
                let _ = sender.send(ScanMessage::Photo(photo));
            };
            match scan_folder(&root, &mut send) {
                Ok(stats) => {
                    let _ = sender.send(ScanMessage::Done {
                        photos: stats.photos,
                        errors: stats.errors,
                    });
                }
                Err(_) => {
                    let _ = sender.send(ScanMessage::Done {
                        photos: 0,
                        errors: 1,
                    });
                }
            }
        });
        Self {
            receiver,
            generation,
        }
    }

    /// Generation token this scan belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Non-blocking drain of pending messages.
    pub fn drain(&self) -> Vec<ScanMessage> {
        self.receiver.try_iter().collect()
    }
}

/// A pool of thumbnail workers.
pub struct ThumbPool {
    jobs: Sender<ThumbKey>,
    results: Receiver<(ThumbKey, Result<GeneratedThumb, ThumbError>)>,
    inflight: HashSet<ThumbKey>,
    cache: ThumbCache,
}

impl ThumbPool {
    /// Starts `threads` workers writing into `cache`.
    pub fn new(cache: ThumbCache, threads: usize) -> Self {
        let (job_tx, job_rx) = channel::<ThumbKey>();
        let (result_tx, result_rx) = channel();
        let shared_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..threads.max(1) {
            let rx = Arc::clone(&shared_rx);
            let tx = result_tx.clone();
            let cache = cache.clone();
            thread::spawn(move || {
                let registry = default_registry();
                loop {
                    let key = {
                        let guard = match rx.lock() {
                            Ok(guard) => guard,
                            Err(_) => return,
                        };
                        match guard.recv() {
                            Ok(key) => key,
                            Err(_) => return,
                        }
                    };
                    let result = generate_for(&registry, &cache, &key);
                    if tx.send((key, result)).is_err() {
                        return;
                    }
                }
            });
        }

        Self {
            jobs: job_tx,
            results: result_rx,
            inflight: HashSet::new(),
            cache,
        }
    }

    /// The backing cache.
    pub fn cache(&self) -> &ThumbCache {
        &self.cache
    }

    /// Requests a thumbnail unless it is cached or already in flight.
    ///
    /// Returns `true` if the request was queued.
    pub fn request(&mut self, key: ThumbKey) -> bool {
        if self.cache.contains(&key) || self.inflight.contains(&key) {
            return false;
        }
        if self.jobs.send(key.clone()).is_ok() {
            self.inflight.insert(key);
            true
        } else {
            false
        }
    }

    /// Non-blocking drain of finished thumbnails.
    pub fn drain(&mut self) -> Vec<(ThumbKey, Result<GeneratedThumb, ThumbError>)> {
        let mut out = Vec::new();
        while let Ok((key, result)) = self.results.try_recv() {
            self.inflight.remove(&key);
            out.push((key, result));
        }
        out
    }

    /// Number of requests currently in flight.
    pub fn inflight(&self) -> usize {
        self.inflight.len()
    }
}

fn generate_for(
    registry: &pinacoteca_decoders::DecoderRegistry,
    cache: &ThumbCache,
    key: &ThumbKey,
) -> Result<GeneratedThumb, ThumbError> {
    let decoder = registry.decoder_for(&key.source);
    match decoder {
        Some(decoder) => {
            let orientation = pinacoteca_decoders::exif::read_orientation(&key.source)
                .unwrap_or(Orientation::Upright);
            generate(cache, key, decoder, orientation)
        }
        None => Err(ThumbError::Decode(
            pinacoteca_decoders::DecodeError::Unsupported,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn scan_handle_streams_photos_and_done() {
        let root =
            std::env::temp_dir().join(format!("pinacoteca-scanhandle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.jpg"), b"x").unwrap();
        std::fs::write(root.join("b.txt"), b"x").unwrap();

        let handle = ScanHandle::start(root.clone(), 7);
        assert_eq!(handle.generation(), 7);

        let mut photos = 0;
        let mut done = false;
        for _ in 0..200 {
            for message in handle.drain() {
                match message {
                    ScanMessage::Photo(_) => photos += 1,
                    ScanMessage::Done { .. } => done = true,
                }
            }
            if done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(photos, 1);
        assert!(done);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thumb_pool_produces_a_thumbnail() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-pool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Encode a small source JPEG so the pool has something to decode.
        let pixels: Vec<u8> = (0..64 * 48 * 4).map(|i| (i % 251) as u8).collect();
        let source = pinacoteca_decoders::RgbaImage::new(64, 48, pixels).unwrap();
        let mut encoded = Vec::new();
        jpeg_encoder_encode(&source, &mut encoded);
        let source_path = dir.join("source.jpg");
        std::fs::write(&source_path, &encoded).unwrap();

        let cache = ThumbCache::new(dir.join("cache"));
        let key = ThumbKey {
            source: source_path,
            fingerprint: pinacoteca_render::thumbs::SourceFingerprint::read(
                &dir.join("source.jpg"),
            )
            .unwrap(),
            size: pinacoteca_render::thumbs::ThumbSize::Normal,
        };

        let mut pool = ThumbPool::new(cache, 2);
        assert!(pool.request(key.clone()));
        assert!(!pool.request(key.clone()), "duplicate request is dropped");
        assert_eq!(pool.inflight(), 1);

        let produced = Arc::new(AtomicUsize::new(0));
        for _ in 0..200 {
            let results = pool.drain();
            if !results.is_empty() {
                for (_, result) in results {
                    assert!(result.is_ok());
                    produced.fetch_add(1, Ordering::SeqCst);
                }
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(produced.load(Ordering::SeqCst), 1);
        assert_eq!(pool.inflight(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn end_to_end_scan_catalog_and_thumbnails() {
        let dir = std::env::temp_dir().join(format!("pinacoteca-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();

        // Two JPEGs (one nested) and one PNG.
        let jpeg_bytes = |seed: u8| {
            let pixels: Vec<u8> = (0..48 * 32 * 4)
                .map(|i| (i as u8).wrapping_add(seed))
                .collect();
            let image = pinacoteca_decoders::RgbaImage::new(48, 32, pixels).unwrap();
            let mut out = Vec::new();
            jpeg_encoder_encode(&image, &mut out);
            out
        };
        std::fs::write(dir.join("a.jpg"), jpeg_bytes(3)).unwrap();
        std::fs::write(dir.join("sub/b.jpg"), jpeg_bytes(9)).unwrap();
        std::fs::write(dir.join("c.png"), make_png(16, 16)).unwrap();

        // 1. Scan.
        let handle = ScanHandle::start(dir.clone(), 1);
        let mut photos = Vec::new();
        let mut done = false;
        for _ in 0..300 {
            for message in handle.drain() {
                match message {
                    ScanMessage::Photo(photo) => photos.push(photo),
                    ScanMessage::Done { .. } => done = true,
                }
            }
            if done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(photos.len(), 3);
        assert!(done);

        // 2. Persist to the catalog.
        let db = pinacoteca_db::Database::open_in_memory().unwrap();
        let folder = db.upsert_folder(&dir.to_string_lossy()).unwrap();
        for photo in &photos {
            db.upsert_photo_facts(
                &photo.path.to_string_lossy(),
                Some(folder),
                photo.len as i64,
                photo.modified_secs,
                photo.path.extension().and_then(|e| e.to_str()),
            )
            .unwrap();
        }
        assert_eq!(db.count_photos().unwrap(), 3);

        // 3. Generate thumbnails for every photo.
        let cache = ThumbCache::new(dir.join("cache"));
        let mut pool = ThumbPool::new(cache.clone(), 3);
        let keys: Vec<ThumbKey> = photos
            .iter()
            .map(|photo| ThumbKey {
                source: photo.path.clone(),
                fingerprint: pinacoteca_render::thumbs::SourceFingerprint::read(&photo.path)
                    .unwrap(),
                size: pinacoteca_render::thumbs::ThumbSize::Normal,
            })
            .collect();
        for key in &keys {
            pool.request(key.clone());
        }

        let mut produced = 0;
        for _ in 0..500 {
            let results = pool.drain();
            if !results.is_empty() {
                for (_, result) in results {
                    assert!(result.is_ok());
                    produced += 1;
                }
                if produced == keys.len() {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(produced, 3);
        for key in &keys {
            assert!(cache.contains(key), "thumbnail must be cached");
            let bytes = cache.read(key).unwrap().unwrap();
            let dims = pinacoteca_decoders::jpeg::dimensions_bytes(&bytes).unwrap();
            assert!(dims.width <= key.size.pixels());
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A minimal 8-bit RGBA PNG.
    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let pixels: Vec<u8> = (0..width * height * 4).map(|i| (i % 251) as u8).collect();
            writer.write_image_data(&pixels).unwrap();
        }
        out
    }

    fn jpeg_encoder_encode(image: &pinacoteca_decoders::RgbaImage, out: &mut Vec<u8>) {
        let encoder = jpeg_encoder::Encoder::new(out, 80);
        encoder
            .encode(
                &image.pixels,
                image.width as u16,
                image.height as u16,
                jpeg_encoder::ColorType::Rgba,
            )
            .unwrap();
    }
}
