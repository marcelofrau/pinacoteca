# Decoder benchmark (M2)

Evidence for [ADR-0001](../adr/0001-decoder-strategy.md). Produced by the
`pinacoteca-bench` harness (workspace member, `publish = false`).

## Method

- Harness: `crates/pinacoteca-bench`, `cargo run --release -p pinacoteca-bench
  -- --reps 4 <folder>`.
- All files are read fully into memory **once, before timing**, so filesystem
  I/O does not pollute decode numbers.
- Each decoder is warmed up once, then timed over the whole sample set × `reps`.
- A checksum of the output is consumed to stop the optimizer discarding work.

### Environment

| | |
|---|---|
| CPU | AMD Ryzen 9 5900X, 12 cores / 24 threads |
| RAM | 32 GiB |
| OS | Windows 11 Pro (10.0.26200), x86_64 |
| Toolchain | cargo/rustc 1.98.0, `x86_64-pc-windows-msvc`, edition 2024 |
| Sample set | 35 files, 256 MiB — 29 JPEG + 6 PNG, real photos (12 MP phone camera + PNG screenshots) |
| JPEG size | avg ~3600×2900 (≈10.5 MP) |

## Results

### Full decode (CPU only, warm, 4 reps)

| decoder | kind | imgs | errors | ms/img | img/s | MP/s |
|---------|------|------|--------|--------|-------|------|
| `zune-jpeg` | JPEG | 116 | 0 | 70.91 | 14.1 | **185.1** |
| `jpeg-decoder` | JPEG | 116 | 0 | 79.65 | 12.6 | 164.8 |
| `image` (jpeg feature) | JPEG | 116 | 0 | 80.24 | 12.5 | 163.6 |
| `mozjpeg` (native libjpeg-turbo, feature) | JPEG | 116 | 0 | **60.23** | 16.6 | **218.0** |
| `png` | PNG | 24 | 0 | 16.19 | 61.8 | **217.7** |
| `image` (png feature) | PNG | 24 | 0 | 17.80 | 56.2 | 198.0 |

### Thumbnail fast path — scaled IDCT

| path | ms/img | speedup vs full | avg output |
|------|--------|-----------------|-----------|
| `zune-jpeg` full | 69.93 | 1.0× | 3624×2910 |
| `jpeg-decoder` → 256 (picks 1/8) | 53.35 | 1.3× | 508×404 |
| `jpeg-decoder` → 1280 (picks 1/4) | 60.33 | 1.2× | 1364×1325 |

### EXIF (JPEG)

```
orientation parse:   21 / 29 JPEGs with EXIF, 0.44 ms total (0.015 ms/img)
orientation values:  1=21
embedded thumbnail:  19 / 29 JPEGs, avg 34.1 KiB (max 54.2),
                     avg size 350x410 (min side 38),
                     0.001 ms/img to locate/extract
```

## Findings

1. **`zune-jpeg` is the fastest pure-Rust JPEG decoder** here — 185 MP/s, ~1.13×
   `jpeg-decoder`.
2. **`image` delegates JPEG to `jpeg-decoder`.** Their full-decode timings are
   within noise of each other (80.2 vs 79.7 ms/img), so choosing `image` for
   JPEG buys convenience, not speed. For PNG, `image` wraps the `png` crate and
   adds ~10% for format negotiation + conversion.
3. **Native `mozjpeg` is only ~18% faster than `zune-jpeg`** (60.2 vs 70.9
   ms/img). That is a modest gain for a mandatory C toolchain (cmake + MSVC/gcc).
4. **Scaled IDCT is not a large win** (1.2–1.3×). For these high-detail JPEGs,
   entropy (Huffman) decoding dominates; scaling only removes part of the IDCT,
   upsampling and colour conversion. Do **not** build the thumbnail pipeline
   around scaled decode.
5. **Reusing the embedded EXIF thumbnail is essentially free** (≈0.001 ms to
   locate/extract vs ~70 ms to decode) and most photos have one (19/29), at a
   usable 350×410 average — but the minimum side seen was 38 px, so a size check
   and decode fallback are required.
6. **EXIF orientation parsing is negligible** (0.015 ms/img). This sample
   contained only `Orientation = 1`, so the *transform* itself is not exercised
   here and must be validated in M3 with rotated samples.
7. Decode is **CPU-bound and parallelizable**: ~70 ms/img single-threaded on a
   12-core machine → a full folder's thumbnails converge in roughly
   `photos × 70 ms / cores` for photos without an EXIF thumbnail.

## Raw output

```
Pinacoteca M2 decoder benchmark
folder: analysis-scratch\photos-bench
reps:   4
samples: 35 (29 JPEG, 6 PNG), 256.3 MiB read in 182.4 ms (1404.6 MiB/s, once, untimed)

== full decode (CPU only, warm) ==
decoder         kind   imgs  errors   total ms     ms/img      img/s       MP/s
zune-jpeg       JPEG    116       0     8225.1      70.91       14.1      185.1
jpeg-decoder    JPEG    116       0     9239.1      79.65       12.6      164.8
image(jpeg)     JPEG    116       0     9308.1      80.24       12.5      163.6
png             PNG      24       0      388.6      16.19       61.8      217.7
image(png)      PNG      24       0      427.2      17.80       56.2      198.0
mozjpeg(native) JPEG    116       0     6986.8      60.23       16.6      218.0

== thumbnail fast path (jpeg-decoder scaled IDCT) ==
path                       ms/img speedup vs full     avg output
zune-jpeg full              69.93           1.0x          3624x2910
jpeg-decoder ->256          53.35           1.3x           508x404
jpeg-decoder ->1280         60.33           1.2x          1364x1325

== EXIF (JPEG) ==
orientation parse: 21 / 29 JPEGs with EXIF, 0.44 ms total (0.015 ms/img)
orientation values: 1=21
embedded thumbnail: 19 / 29 JPEGs, avg 34.1 KiB (max 54.2), avg size 350x410 (min side 38), 10.23 ms total (0.353 ms/img)
```

> The embedded-thumbnail line's 0.353 ms/img is the cost of *decoding* each
> extracted thumbnail to measure its size; the locate/extract step alone is
> ~0.001 ms/img.

## Caveats

- Single machine, single run, 4 reps; treat the numbers as order-of-magnitude,
  not a microbenchmark. Relative ordering was stable across runs.
- Sample is phone-camera JPEGs (high detail) + screenshots; a different mix
  (e.g. progressive JPEG, low-detail) could shift the scaled-decode result.
- `mozjpeg` was built from source by `mozjpeg-sys` with the local MSVC + nasm
  toolchain; the default build never compiles it (behind a feature).
