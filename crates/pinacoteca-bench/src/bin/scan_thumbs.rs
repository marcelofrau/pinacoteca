//! Analysis binary: scan a real folder and time thumbnail generation.
//!
//! Usage: `cargo run -p pinacoteca-bench --bin scan-thumbs -- <folder> [limit]`

use std::path::PathBuf;
use std::time::Instant;

use pinacoteca_core::scan::{DiscoveredPhoto, scan_folder};
use pinacoteca_decoders::{Orientation, default_registry};
use pinacoteca_render::thumbs::{SourceFingerprint, ThumbCache, ThumbKey, ThumbSize, generate};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(folder) = args.next() else {
        eprintln!("usage: scan-thumbs <folder> [limit]");
        std::process::exit(2);
    };
    let limit: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    run(PathBuf::from(folder), limit);
}

fn run(root: PathBuf, limit: usize) {
    let scan_start = Instant::now();
    let mut photos: Vec<DiscoveredPhoto> = Vec::new();
    let stats = scan_folder(&root, &mut |photo| photos.push(photo)).expect("scan folder");
    let scan_time = scan_start.elapsed();

    photos.sort_by(|a, b| a.path.cmp(&b.path));
    photos.truncate(limit);

    println!("folder: {}", root.display());
    println!(
        "scan: {} photos in {} dirs ({} errors) in {:.1} ms",
        stats.photos,
        stats.directories,
        stats.errors,
        scan_time.as_secs_f64() * 1000.0
    );
    println!("measuring first {} photos", photos.len());

    let cache_root = std::env::temp_dir().join("pinacoteca-scan-thumbs-cache");
    let _ = std::fs::remove_dir_all(&cache_root);
    let cache = ThumbCache::new(cache_root.clone());
    let registry = default_registry();

    let mut exif_hits = 0usize;
    let mut failures = 0usize;
    let mut total = std::time::Duration::ZERO;
    let mut first_100 = None;

    for (index, photo) in photos.iter().enumerate() {
        let Some(decoder) = registry.decoder_for(&photo.path) else {
            failures += 1;
            continue;
        };
        if pinacoteca_decoders::exif::embedded_thumbnail_file(&photo.path)
            .ok()
            .flatten()
            .is_some()
        {
            exif_hits += 1;
        }
        let key = ThumbKey {
            source: photo.path.clone(),
            fingerprint: SourceFingerprint::read(&photo.path).unwrap_or(SourceFingerprint {
                len: photo.len,
                modified_secs: photo.modified_secs,
                modified_nanos: photo.modified_nanos,
            }),
            size: ThumbSize::Normal,
        };
        let orientation = pinacoteca_decoders::exif::read_orientation(&photo.path)
            .unwrap_or(Orientation::Upright);
        let started = Instant::now();
        match generate(&cache, &key, decoder, orientation) {
            Ok(_) => total += started.elapsed(),
            Err(_) => failures += 1,
        }
        if index + 1 == 100.min(photos.len()) {
            first_100 = Some(total);
        }
    }

    let produced = photos.len().saturating_sub(failures);
    println!(
        "thumbnails: {produced} produced, {failures} failed, {exif_hits} with an embedded EXIF thumbnail"
    );
    if produced > 0 {
        println!(
            "time: {:.1} ms total, {:.1} ms/photo (single thread)",
            total.as_secs_f64() * 1000.0,
            total.as_secs_f64() * 1000.0 / produced as f64
        );
    }
    if let Some(first) = first_100 {
        println!(
            "first {} thumbnails: {:.1} ms",
            100.min(photos.len()),
            first.as_secs_f64() * 1000.0
        );
    }
    println!("cache: {}", cache_root.display());
}
