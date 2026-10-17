//! M2 decoder benchmark harness.
//!
//! Measures candidate image decoders against a real photo set so the decoder
//! strategy (ADR-0001) is decided on evidence, not assumption. This is an
//! analysis tool: it is not shipped and takes the photo folder as a CLI
//! argument, so no photos are read from or written to the repository.
//!
//! Usage:
//!   pinacoteca-bench [--limit N] [--reps N] <folder>
//!
//! Decode timing measures CPU work only: each file is read fully into memory
//! once, before timing, so filesystem cache/IO does not pollute the numbers.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Config {
    folder: PathBuf,
    limit: usize,
    reps: u32,
}

fn parse_args() -> Result<Config, String> {
    let mut folder: Option<PathBuf> = None;
    let mut limit = usize::MAX;
    let mut reps = 3u32;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--limit" => {
                limit = args
                    .next()
                    .ok_or("--limit needs a value")?
                    .parse()
                    .map_err(|_| "--limit must be an integer".to_string())?;
            }
            "--reps" => {
                reps = args
                    .next()
                    .ok_or("--reps needs a value")?
                    .parse()
                    .map_err(|_| "--reps must be an integer".to_string())?;
            }
            "-h" | "--help" => return Err(usage().to_string()),
            other => {
                if other.starts_with("--") {
                    return Err(format!("unknown option {other}\n\n{}", usage()));
                }
                folder = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Config {
        folder: folder.ok_or_else(|| usage().to_string())?,
        limit,
        reps: reps.max(1),
    })
}

fn usage() -> &'static str {
    "usage: pinacoteca-bench [--limit N] [--reps N] <folder>"
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Jpeg,
    Png,
}

impl Kind {
    fn from_path(path: &Path) -> Option<Kind> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "jpg" | "jpeg" => Some(Kind::Jpeg),
            "png" => Some(Kind::Png),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Jpeg => "JPEG",
            Kind::Png => "PNG",
        }
    }
}

struct Sample {
    kind: Kind,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct Decoded {
    width: u32,
    height: u32,
    len: usize,
}

impl Decoded {
    fn megapixels(&self) -> f64 {
        (self.width as f64 * self.height as f64) / 1_000_000.0
    }
}

type DecodeFn = fn(&[u8]) -> Result<Decoded, String>;

struct Candidate {
    name: &'static str,
    kind: Kind,
    decode: DecodeFn,
}

fn decode_zune_jpeg(data: &[u8]) -> Result<Decoded, String> {
    let mut decoder = zune_jpeg::JpegDecoder::new(std::io::Cursor::new(data));
    let pixels = decoder.decode().map_err(|e| format!("{e:?}"))?;
    let info = decoder.info().ok_or("no image info")?;
    Ok(Decoded {
        width: info.width as u32,
        height: info.height as u32,
        len: pixels.len(),
    })
}

fn decode_jpeg_decoder(data: &[u8]) -> Result<Decoded, String> {
    let mut decoder = jpeg_decoder::Decoder::new(data);
    let pixels = decoder.decode().map_err(|e| format!("{e}"))?;
    let info = decoder.info().ok_or("no image info")?;
    Ok(Decoded {
        width: info.width as u32,
        height: info.height as u32,
        len: pixels.len(),
    })
}

fn decode_image_crate_jpeg(data: &[u8]) -> Result<Decoded, String> {
    let image = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;
    let rgb = image.to_rgb8();
    Ok(Decoded {
        width: rgb.width(),
        height: rgb.height(),
        len: rgb.as_raw().len(),
    })
}

fn decode_png_crate(data: &[u8]) -> Result<Decoded, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size().ok_or("buffer size overflow")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    Ok(Decoded {
        width: info.width,
        height: info.height,
        len: info.buffer_size(),
    })
}

fn decode_image_crate_png(data: &[u8]) -> Result<Decoded, String> {
    let image = image::load_from_memory_with_format(data, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let rgba = image.to_rgba8();
    Ok(Decoded {
        width: rgba.width(),
        height: rgba.height(),
        len: rgba.as_raw().len(),
    })
}

fn candidates() -> Vec<Candidate> {
    #[allow(unused_mut)]
    let mut list = vec![
        Candidate {
            name: "zune-jpeg",
            kind: Kind::Jpeg,
            decode: decode_zune_jpeg,
        },
        Candidate {
            name: "jpeg-decoder",
            kind: Kind::Jpeg,
            decode: decode_jpeg_decoder,
        },
        Candidate {
            name: "image(jpeg)",
            kind: Kind::Jpeg,
            decode: decode_image_crate_jpeg,
        },
        Candidate {
            name: "png",
            kind: Kind::Png,
            decode: decode_png_crate,
        },
        Candidate {
            name: "image(png)",
            kind: Kind::Png,
            decode: decode_image_crate_png,
        },
    ];

    #[cfg(feature = "native-mozjpeg")]
    list.push(Candidate {
        name: "mozjpeg(native)",
        kind: Kind::Jpeg,
        decode: decode_mozjpeg,
    });

    list
}

/// Native libjpeg-turbo (mozjpeg) decode, compiled only with the feature.
#[cfg(feature = "native-mozjpeg")]
fn decode_mozjpeg(data: &[u8]) -> Result<Decoded, String> {
    let decompress = mozjpeg::Decompress::new_mem(data).map_err(|e| e.to_string())?;
    let width = decompress.width() as u32;
    let height = decompress.height() as u32;
    let mut started = decompress.rgb().map_err(|e| e.to_string())?;
    let pixels = started.read_scanlines::<u8>().map_err(|e| e.to_string())?;
    Ok(Decoded {
        width,
        height,
        len: pixels.len(),
    })
}

/// Reads the EXIF Orientation (1..=8) from a JPEG/TIFF container, if present.
fn exif_orientation(data: &[u8]) -> Option<u16> {
    let mut cursor = std::io::Cursor::new(data);
    let exif = exif::Reader::new().read_from_container(&mut cursor).ok()?;
    let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    field.value.get_uint(0).map(|v| v as u16)
}

/// Extracts the JPEG thumbnail embedded in the EXIF APP1 segment, if present.
///
/// This is the "reuse the EXIF thumbnail" fast path a thumbnail pipeline wants:
/// it avoids a full decode when a sufficiently large thumbnail already exists.
fn embedded_exif_thumbnail(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xDA {
            break; // start of scan: metadata ends
        }
        let seg_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if seg_len < 2 || i + 2 + seg_len > data.len() {
            return None;
        }
        if marker == 0xE1 {
            let seg = &data[i + 4..i + 2 + seg_len];
            if seg.len() >= 6 && &seg[..6] == b"Exif\0\0" {
                return parse_tiff_thumbnail(&seg[6..]);
            }
        }
        i += 2 + seg_len;
    }
    None
}

fn parse_tiff_thumbnail(tiff: &[u8]) -> Option<Vec<u8>> {
    if tiff.len() < 8 {
        return None;
    }
    let little = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let rd16 = |o: usize| -> Option<u16> {
        let b = tiff.get(o..o + 2)?;
        Some(if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let rd32 = |o: usize| -> Option<u32> {
        let b = tiff.get(o..o + 4)?;
        Some(if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    };

    let ifd0 = rd32(4)? as usize;
    let entries0 = rd16(ifd0)? as usize;
    let ifd1 = rd32(ifd0 + 2 + entries0 * 12)? as usize;
    if ifd1 == 0 {
        return None;
    }
    let entries1 = rd16(ifd1)? as usize;

    let mut offset = None;
    let mut length = None;
    for e in 0..entries1 {
        let ep = ifd1 + 2 + e * 12;
        let tag = rd16(ep)?;
        let field_type = rd16(ep + 2)?;
        let value = if field_type == 3 {
            rd16(ep + 8)? as u32
        } else {
            rd32(ep + 8)?
        };
        match tag {
            0x0201 => offset = Some(value as usize),
            0x0202 => length = Some(value as usize),
            _ => {}
        }
    }

    let end = offset?.checked_add(length?)?;
    let thumb = tiff.get(offset?..end)?;
    (thumb.len() >= 2 && thumb[0] == 0xFF && thumb[1] == 0xD8).then(|| thumb.to_vec())
}

fn collect(config: &Config) -> Vec<Sample> {
    let mut paths = Vec::new();
    let mut stack = vec![config.folder.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                paths.push(path);
            }
        }
    }
    paths.sort();

    let mut samples = Vec::new();
    for path in paths {
        let Some(kind) = Kind::from_path(&path) else {
            continue;
        };
        if samples.len() >= config.limit {
            break;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            samples.push(Sample { kind, bytes });
        }
    }
    samples
}

fn megabytes(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn main() {
    let config = match parse_args() {
        Ok(c) => c,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    println!("Pinacoteca M2 decoder benchmark");
    println!("folder: {}", config.folder.display());
    println!("reps:   {}", config.reps);

    let read_start = Instant::now();
    let samples = collect(&config);
    let read_elapsed = read_start.elapsed();
    let total_bytes: usize = samples.iter().map(|s| s.bytes.len()).sum();
    let jpeg_count = samples.iter().filter(|s| s.kind == Kind::Jpeg).count();
    let png_count = samples.iter().filter(|s| s.kind == Kind::Png).count();

    println!(
        "samples: {} ({} JPEG, {} PNG), {:.1} MiB read in {:.1} ms ({:.1} MiB/s, once, untimed)",
        samples.len(),
        jpeg_count,
        png_count,
        megabytes(total_bytes),
        read_elapsed.as_secs_f64() * 1000.0,
        megabytes(total_bytes) / read_elapsed.as_secs_f64().max(f64::EPSILON)
    );

    bench_decoders(&samples, config.reps);
    bench_thumbnails(&samples, config.reps);
    bench_exif(&samples);
}

/// Decodes a JPEG at a reduced IDCT scale (1/1, 1/2, 1/4 or 1/8) chosen so the
/// result still covers `target`. This is the thumbnail fast path: no full-size
/// decode, no separate downscale step.
fn decode_jpeg_decoder_scaled(data: &[u8], target: u16) -> Result<Decoded, String> {
    let mut decoder = jpeg_decoder::Decoder::new(data);
    let (width, height) = decoder.scale(target, target).map_err(|e| e.to_string())?;
    let pixels = decoder.decode().map_err(|e| format!("{e}"))?;
    Ok(Decoded {
        width: width as u32,
        height: height as u32,
        len: pixels.len(),
    })
}

fn bench_thumbnails(samples: &[Sample], reps: u32) {
    let jpegs: Vec<&Sample> = samples.iter().filter(|s| s.kind == Kind::Jpeg).collect();
    if jpegs.is_empty() {
        return;
    }

    // Baseline: full decode with the fastest full-size decoder.
    let full_ms = time_decode(&jpegs, reps, decode_zune_jpeg);

    println!("\n== thumbnail fast path (jpeg-decoder scaled IDCT) ==");
    println!(
        "{:<22} {:>10} {:>14} {:>14}",
        "path", "ms/img", "speedup vs full", "avg output"
    );
    let (full_w, full_h) = avg_dims(&jpegs, decode_zune_jpeg);
    println!(
        "{:<22} {:>10.2} {:>14} {:>13.0}x{:<0.0}",
        "zune-jpeg full", full_ms, "1.0x", full_w, full_h
    );

    for target in [256u16, 1280] {
        let name = format!("jpeg-decoder ->{target}");
        let (scaled_ms, w, h) = time_decode_scaled(&jpegs, reps, target);
        println!(
            "{:<22} {:>10.2} {:>13.1}x {:>13.0}x{:<0.0}",
            name,
            scaled_ms,
            full_ms / scaled_ms.max(f64::EPSILON),
            w,
            h
        );
    }
}

/// Average output dimensions of a decoder over samples.
fn avg_dims(samples: &[&Sample], decode: DecodeFn) -> (f64, f64) {
    let mut w = 0f64;
    let mut h = 0f64;
    let mut n = 0u64;
    for sample in samples {
        if let Ok(d) = decode(&sample.bytes) {
            w += d.width as f64;
            h += d.height as f64;
            n += 1;
        }
    }
    (w / n.max(1) as f64, h / n.max(1) as f64)
}

/// Times a decoder over JPEG samples; returns milliseconds per image.
fn time_decode(samples: &[&Sample], reps: u32, decode: DecodeFn) -> f64 {
    for sample in samples {
        let _ = decode(&sample.bytes);
    }
    let mut total = Duration::ZERO;
    let mut ok = 0u64;
    let mut checksum = 0u64;
    for _ in 0..reps {
        for sample in samples {
            let start = Instant::now();
            if let Ok(decoded) = decode(&sample.bytes) {
                total += start.elapsed();
                checksum ^= decoded.len as u64;
                ok += 1;
            }
        }
    }
    black_box(checksum);
    total.as_secs_f64() * 1000.0 / ok.max(1) as f64
}

/// Times a scaled decode over JPEG samples; returns (ms per image, avg w, avg h).
fn time_decode_scaled(samples: &[&Sample], reps: u32, target: u16) -> (f64, f64, f64) {
    for sample in samples {
        let _ = decode_jpeg_decoder_scaled(&sample.bytes, target);
    }
    let mut total = Duration::ZERO;
    let mut ok = 0u64;
    let mut checksum = 0u64;
    let mut wsum = 0f64;
    let mut hsum = 0f64;
    for _ in 0..reps {
        for sample in samples {
            let start = Instant::now();
            if let Ok(decoded) = decode_jpeg_decoder_scaled(&sample.bytes, target) {
                total += start.elapsed();
                checksum ^= decoded.len as u64;
                wsum += decoded.width as f64;
                hsum += decoded.height as f64;
                ok += 1;
            }
        }
    }
    black_box(checksum);
    let n = ok.max(1) as f64;
    (total.as_secs_f64() * 1000.0 / n, wsum / n, hsum / n)
}

fn bench_decoders(samples: &[Sample], reps: u32) {
    println!("\n== full decode (CPU only, warm) ==");
    println!(
        "{:<14} {:>5} {:>6} {:>7} {:>10} {:>10} {:>10} {:>10}",
        "decoder", "kind", "imgs", "errors", "total ms", "ms/img", "img/s", "MP/s"
    );

    for candidate in candidates() {
        let files: Vec<&Sample> = samples
            .iter()
            .filter(|s| s.kind == candidate.kind)
            .collect();
        if files.is_empty() {
            continue;
        }

        // Warm up: first decode populates any internal caches.
        for sample in &files {
            let _ = (candidate.decode)(&sample.bytes);
        }

        let mut total = Duration::ZERO;
        let mut megapixels = 0.0f64;
        let mut ok = 0u64;
        let mut errors = 0u64;
        let mut checksum = 0u64;

        for _ in 0..reps {
            for sample in &files {
                let start = Instant::now();
                match (candidate.decode)(&sample.bytes) {
                    Ok(decoded) => {
                        total += start.elapsed();
                        megapixels += decoded.megapixels();
                        ok += 1;
                        checksum ^= decoded.len as u64;
                    }
                    Err(_) => errors += 1,
                }
            }
        }
        black_box(checksum);

        let secs = total.as_secs_f64().max(f64::EPSILON);
        println!(
            "{:<14} {:>5} {:>6} {:>7} {:>10.1} {:>10.2} {:>10.1} {:>10.1}",
            candidate.name,
            candidate.kind.label(),
            ok,
            errors,
            total.as_secs_f64() * 1000.0,
            total.as_secs_f64() * 1000.0 / ok.max(1) as f64,
            ok as f64 / secs,
            megapixels / secs
        );
    }
}

fn bench_exif(samples: &[Sample]) {
    let jpegs: Vec<&Sample> = samples.iter().filter(|s| s.kind == Kind::Jpeg).collect();
    if jpegs.is_empty() {
        return;
    }

    println!("\n== EXIF (JPEG) ==");

    let start = Instant::now();
    let mut orientations = [0u32; 9];
    let mut exif_present = 0u32;
    for sample in &jpegs {
        if let Some(o) = exif_orientation(&sample.bytes) {
            exif_present += 1;
            if (1..=8).contains(&o) {
                orientations[o as usize] += 1;
            }
        }
    }
    let exif_elapsed = start.elapsed();

    println!(
        "orientation parse: {} / {} JPEGs with EXIF, {:.2} ms total ({:.3} ms/img)",
        exif_present,
        jpegs.len(),
        exif_elapsed.as_secs_f64() * 1000.0,
        exif_elapsed.as_secs_f64() * 1000.0 / jpegs.len() as f64
    );
    print!("orientation values: ");
    for (value, count) in orientations.iter().enumerate().skip(1) {
        if *count > 0 {
            print!("{value}={count}  ");
        }
    }
    println!();

    let start = Instant::now();
    let mut with_thumb = 0u32;
    let mut thumb_bytes = 0usize;
    let mut max_thumb = 0usize;
    let mut tw = 0u64;
    let mut th = 0u64;
    let mut timed = 0u64;
    let mut min_side = u32::MAX;
    for sample in &jpegs {
        if let Some(thumb) = embedded_exif_thumbnail(&sample.bytes) {
            with_thumb += 1;
            thumb_bytes += thumb.len();
            max_thumb = max_thumb.max(thumb.len());
            if let Ok(d) = decode_zune_jpeg(&thumb) {
                tw += d.width as u64;
                th += d.height as u64;
                timed += 1;
                min_side = min_side.min(d.width.min(d.height));
            }
        }
    }
    let thumb_elapsed = start.elapsed();

    let n = timed.max(1) as f64;
    println!(
        "embedded thumbnail: {} / {} JPEGs, avg {:.1} KiB (max {:.1}), avg size {:.0}x{:.0} (min side {}), {:.2} ms total ({:.3} ms/img)",
        with_thumb,
        jpegs.len(),
        if with_thumb > 0 {
            thumb_bytes as f64 / with_thumb as f64 / 1024.0
        } else {
            0.0
        },
        max_thumb as f64 / 1024.0,
        tw as f64 / n,
        th as f64 / n,
        if min_side == u32::MAX { 0 } else { min_side },
        thumb_elapsed.as_secs_f64() * 1000.0,
        thumb_elapsed.as_secs_f64() * 1000.0 / jpegs.len() as f64
    );
}
