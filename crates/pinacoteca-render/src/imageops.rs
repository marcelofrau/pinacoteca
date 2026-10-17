//! Small image operations used when preparing thumbnails and previews.
//!
//! These are deliberately simple, dependency-free CPU routines: a box-average
//! downscale and the eight EXIF orientation transforms. They run on worker
//! threads (ADR-0005), never on the UI thread.

use pinacoteca_decoders::{Orientation, RgbaImage};

/// Scales `image` down so its longest side is at most `max_side`, preserving
/// aspect ratio. Images already within the limit are returned unchanged; this
/// never upscales.
pub fn resize_to_fit(image: &RgbaImage, max_side: u32) -> RgbaImage {
    let max_side = max_side.max(1);
    let longest = image.width.max(image.height);
    if longest <= max_side {
        return image.clone();
    }

    let scale = max_side as f32 / longest as f32;
    let width = ((image.width as f32 * scale).round() as u32).max(1);
    let height = ((image.height as f32 * scale).round() as u32).max(1);
    box_resample(image, width, height)
}

/// Area-average (box) resample to exactly `width` × `height`.
fn box_resample(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let src_w = image.width as u64;
    let src_h = image.height as u64;
    let dst_w = width as u64;
    let dst_h = height as u64;

    for dy in 0..dst_h {
        let y0 = dy * src_h / dst_h;
        let y1 = (((dy + 1) * src_h + dst_h - 1) / dst_h).max(y0 + 1).min(src_h);
        for dx in 0..dst_w {
            let x0 = dx * src_w / dst_w;
            let x1 = (((dx + 1) * src_w + dst_w - 1) / dst_w).max(x0 + 1).min(src_w);

            let mut sums = [0u64; 4];
            let mut count = 0u64;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = (sy * src_w + sx) as usize * 4;
                    for (channel, sum) in sums.iter_mut().enumerate() {
                        *sum += image.pixels[i + channel] as u64;
                    }
                    count += 1;
                }
            }
            let o = (dy * dst_w + dx) as usize * 4;
            let count = count.max(1);
            for (channel, sum) in sums.iter().enumerate() {
                pixels[o + channel] = (sum / count) as u8;
            }
        }
    }

    RgbaImage::new(width, height, pixels).expect("buffer matches dimensions")
}

/// Applies an EXIF orientation transform, returning a new upright image.
///
/// Dimensions are swapped for the four 90°/270° transforms.
pub fn apply_orientation(image: &RgbaImage, orientation: Orientation) -> RgbaImage {
    if orientation == Orientation::Upright {
        return image.clone();
    }

    let (src_w, src_h) = (image.width, image.height);
    let (dst_w, dst_h) = orientation.oriented_dimensions(src_w, src_h);
    let mut pixels = vec![0u8; dst_w as usize * dst_h as usize * 4];

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            // Inverse mapping: where in the source does dest (dx, dy) come from.
            let (sx, sy) = match orientation {
                Orientation::Upright => (dx, dy),
                Orientation::MirrorH => (src_w - 1 - dx, dy),
                Orientation::Rotate180 => (src_w - 1 - dx, src_h - 1 - dy),
                Orientation::MirrorV => (dx, src_h - 1 - dy),
                Orientation::Transpose => (dy, dx),
                Orientation::Rotate90Cw => (dy, src_h - 1 - dx),
                Orientation::Transverse => (src_w - 1 - dy, src_h - 1 - dx),
                Orientation::Rotate270Cw => (src_w - 1 - dy, dx),
            };
            let src = (sy as usize * src_w as usize + sx as usize) * 4;
            let dst = (dy as usize * dst_w as usize + dx as usize) * 4;
            pixels[dst..dst + 4].copy_from_slice(&image.pixels[src..src + 4]);
        }
    }

    RgbaImage::new(dst_w, dst_h, pixels).expect("buffer matches dimensions")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32) -> RgbaImage {
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[(index % 256) as u8, 10, 20, 255]);
        }
        RgbaImage::new(width, height, pixels).unwrap()
    }

    #[test]
    fn resize_preserves_aspect_and_caps_longest_side() {
        let resized = resize_to_fit(&image(4000, 3000), 256);
        assert_eq!(resized.width, 256);
        assert_eq!(resized.height, 192);
    }

    #[test]
    fn resize_never_upscales() {
        let source = image(100, 50);
        let resized = resize_to_fit(&source, 256);
        assert_eq!((resized.width, resized.height), (100, 50));
    }

    #[test]
    fn rotate_90_swaps_dimensions_and_moves_pixels() {
        // 2x1 image: pixel 0 = value 1, pixel 1 = value 2.
        let src = RgbaImage::new(2, 1, vec![1, 0, 0, 255, 2, 0, 0, 255]).unwrap();
        let rotated = apply_orientation(&src, Orientation::Rotate90Cw);
        assert_eq!((rotated.width, rotated.height), (1, 2));
        // Clockwise: left pixel goes to top, right pixel to bottom.
        assert_eq!(rotated.pixels[0], 1);
        assert_eq!(rotated.pixels[4], 2);
    }

    #[test]
    fn rotate_180_reverses_and_keeps_dimensions() {
        let src = RgbaImage::new(2, 1, vec![1, 0, 0, 255, 2, 0, 0, 255]).unwrap();
        let rotated = apply_orientation(&src, Orientation::Rotate180);
        assert_eq!((rotated.width, rotated.height), (2, 1));
        assert_eq!(rotated.pixels[0], 2);
        assert_eq!(rotated.pixels[4], 1);
    }

    #[test]
    fn mirror_h_flips_left_right() {
        let src = RgbaImage::new(2, 1, vec![1, 0, 0, 255, 2, 0, 0, 255]).unwrap();
        let mirrored = apply_orientation(&src, Orientation::MirrorH);
        assert_eq!(mirrored.pixels[0], 2);
        assert_eq!(mirrored.pixels[4], 1);
    }

    #[test]
    fn upright_is_identity() {
        let src = image(3, 2);
        assert_eq!(apply_orientation(&src, Orientation::Upright), src);
    }
}
