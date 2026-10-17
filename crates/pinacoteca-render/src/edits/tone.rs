//! Tone and effect operations. Values are in `-100..=100` unless stated.

use pinacoteca_core::edit::{Effect, Rect};
use pinacoteca_decoders::RgbaImage;

use super::{get, map, put};

/// Rec. 601 luma.
fn luma(pixel: [u8; 4]) -> f32 {
    0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32
}

/// Brightness: shifts every channel by up to ±127.
pub fn brightness(image: &RgbaImage, value: i32) -> RgbaImage {
    let delta = value.clamp(-100, 100) as f32 * 1.275;
    map(image, |pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] = (pixel[channel] as f32 + delta).clamp(0.0, 255.0) as u8;
        }
        out
    })
}

/// Contrast: pushes each channel away from mid-grey.
pub fn contrast(image: &RgbaImage, value: i32) -> RgbaImage {
    let factor = 1.0 + value.clamp(-100, 100) as f32 / 100.0;
    map(image, |pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] =
                ((pixel[channel] as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
        }
        out
    })
}

/// Saturation: `-100` desaturates fully.
pub fn saturation(image: &RgbaImage, value: i32) -> RgbaImage {
    let factor = 1.0 + value.clamp(-100, 100) as f32 / 100.0;
    map(image, |pixel| {
        let grey = luma(pixel);
        let mut out = pixel;
        for channel in 0..3 {
            out[channel] = (grey + (pixel[channel] as f32 - grey) * factor).clamp(0.0, 255.0) as u8;
        }
        out
    })
}

/// Warmth: `-100` is cool (blue), `100` is warm (orange).
pub fn warmth(image: &RgbaImage, value: i32) -> RgbaImage {
    let shift = value.clamp(-100, 100) as f32 * 0.5;
    map(image, |pixel| {
        let mut out = pixel;
        out[0] = (pixel[0] as f32 + shift).clamp(0.0, 255.0) as u8;
        out[2] = (pixel[2] as f32 - shift).clamp(0.0, 255.0) as u8;
        out
    })
}

/// Applies a monochrome preset.
pub fn effect(image: &RgbaImage, effect: Effect) -> RgbaImage {
    match effect {
        Effect::Grayscale => map(image, |pixel| {
            let grey = luma(pixel).clamp(0.0, 255.0) as u8;
            [grey, grey, grey, pixel[3]]
        }),
        Effect::Sepia => map(image, |pixel| {
            let grey = luma(pixel);
            let set = |value: f32| value.clamp(0.0, 255.0) as u8;
            [
                set(grey * 1.07 + 22.0),
                set(grey * 0.98 + 8.0),
                set(grey * 0.85 - 12.0),
                pixel[3],
            ]
        }),
    }
}

/// Automatic levels: stretches each channel between the 0.5% and 99.5%
/// percentiles, so a flat or hazy photo gains contrast.
pub fn auto_fix(image: &RgbaImage) -> RgbaImage {
    let mut low = [0u8; 3];
    let mut high = [255u8; 3];
    for channel in 0..3 {
        let mut histogram = [0u32; 256];
        for pixel in image.pixels.as_chunks::<4>().0 {
            histogram[pixel[channel] as usize] += 1;
        }
        let total: u32 = histogram.iter().sum();
        let cut = ((total as f32 * 0.005) as u32).max(1);
        low[channel] = percentile(&histogram, cut, false);
        high[channel] = percentile(&histogram, cut, true);
    }
    map(image, |pixel| {
        let mut out = pixel;
        for channel in 0..3 {
            let (lo, hi) = (low[channel] as f32, high[channel] as f32);
            let span = (hi - lo).max(1.0);
            out[channel] = ((pixel[channel] as f32 - lo) / span * 255.0).clamp(0.0, 255.0) as u8;
        }
        out
    })
}

/// Value at which the cumulative histogram reaches `cut`, from the bottom
/// (`from_top` false) or from the top.
fn percentile(histogram: &[u32; 256], cut: u32, from_top: bool) -> u8 {
    let mut accumulated = 0u32;
    if from_top {
        for value in (0..256).rev() {
            accumulated += histogram[value];
            if accumulated >= cut {
                return value as u8;
            }
        }
        return 0;
    }
    for (value, &count) in histogram.iter().enumerate() {
        accumulated += count;
        if accumulated >= cut {
            return value as u8;
        }
    }
    255
}
/// Unsharp mask. `strength` is roughly 0..=20.
pub fn sharpen(image: &RgbaImage, strength: u8) -> RgbaImage {
    let amount = strength.min(20) as f32 / 10.0;
    if amount == 0.0 {
        return image.clone();
    }
    let (width, height) = (image.width, image.height);
    let mut pixels = vec![0u8; image.pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let mut average = [0f32; 3];
            let mut count = 0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let sx = x as i32 + dx;
                    let sy = y as i32 + dy;
                    if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                        continue;
                    }
                    let neighbour = get(image, sx as u32, sy as u32);
                    for channel in 0..3 {
                        average[channel] += neighbour[channel] as f32;
                    }
                    count += 1.0;
                }
            }
            let original = get(image, x, y);
            let mut out = original;
            for channel in 0..3 {
                let blurred = average[channel] / count;
                out[channel] = (original[channel] as f32
                    + (original[channel] as f32 - blurred) * amount)
                    .clamp(0.0, 255.0) as u8;
            }
            put(&mut pixels, width, x, y, out);
        }
    }
    RgbaImage::new(width, height, pixels).expect("dimensions match the buffer")
}

/// Removes red-eye inside `rect`: strongly red pixels take the green/blue mean.
pub fn red_eye(image: &RgbaImage, rect: Rect) -> RgbaImage {
    let mut result = image.clone();
    let x0 = rect.0.min(image.width.saturating_sub(1));
    let y0 = rect.1.min(image.height.saturating_sub(1));
    let x1 = (rect.0 + rect.2).min(image.width);
    let y1 = (rect.1 + rect.3).min(image.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let pixel = get(image, x, y);
            let (r, g, b) = (pixel[0] as f32, pixel[1] as f32, pixel[2] as f32);
            if r > 60.0 && r > 1.5 * g && r > 1.5 * b {
                let neutral = ((g + b) / 2.0).clamp(0.0, 255.0) as u8;
                put(
                    &mut result.pixels,
                    image.width,
                    x,
                    y,
                    [neutral, g as u8, b as u8, pixel[3]],
                );
            }
        }
    }
    result
}
