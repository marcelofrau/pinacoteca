//! Geometry operations: rotate, crop, straighten.

use pinacoteca_core::edit::{Rect, Rotation};
use pinacoteca_decoders::RgbaImage;

use super::{get, put};

/// Rotates by a multiple of 90 degrees.
pub fn rotate(image: &RgbaImage, rotation: Rotation) -> RgbaImage {
    match rotation {
        Rotation::Cw90 => rotate_cw90(image),
        Rotation::Cw180 => rotate_cw90(&rotate_cw90(image)),
        Rotation::Cw270 => rotate_cw90(&rotate_cw90(&rotate_cw90(image))),
    }
}

/// One clockwise quarter turn: `dest(x, y) = src(y, height - 1 - x)`.
pub fn rotate_cw90(image: &RgbaImage) -> RgbaImage {
    let (width, height) = (image.width, image.height);
    let (new_width, new_height) = (height, width);
    let mut pixels = vec![0u8; new_width as usize * new_height as usize * 4];
    for y in 0..new_height {
        for x in 0..new_width {
            put(&mut pixels, new_width, x, y, get(image, y, height - 1 - x));
        }
    }
    RgbaImage::new(new_width, new_height, pixels).expect("dimensions match the buffer")
}

/// Crops to `rect`, clamped to the image so an out-of-range edit is harmless.
pub fn crop(image: &RgbaImage, rect: Rect) -> RgbaImage {
    let x = rect.0.min(image.width.saturating_sub(1));
    let y = rect.1.min(image.height.saturating_sub(1));
    let width = rect.2.clamp(1, image.width - x);
    let height = rect.3.clamp(1, image.height - y);

    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let row = width as usize * 4;
    for line in 0..height {
        let src = ((y + line) as usize * image.width as usize + x as usize) * 4;
        let dst = line as usize * row;
        pixels[dst..dst + row].copy_from_slice(&image.pixels[src..src + row]);
    }
    RgbaImage::new(width, height, pixels).expect("dimensions match the buffer")
}

/// Rotates around the centre by `hundredths` of a degree, growing the canvas so
/// no content is cut off. Positive is clockwise.
pub fn straighten(image: &RgbaImage, hundredths: i32) -> RgbaImage {
    if hundredths == 0 {
        return image.clone();
    }
    let angle = hundredths as f32 / 100.0 * std::f32::consts::PI / 180.0;
    let (sin, cos) = angle.sin_cos();
    let (width, height) = (image.width as f32, image.height as f32);
    let new_width = (width * cos.abs() + height * sin.abs()).ceil() as u32;
    let new_height = (width * sin.abs() + height * cos.abs()).ceil() as u32;

    let (cx, cy) = ((width - 1.0) / 2.0, (height - 1.0) / 2.0);
    let (ncx, ncy) = (
        (new_width as f32 - 1.0) / 2.0,
        (new_height as f32 - 1.0) / 2.0,
    );

    let mut pixels = vec![0u8; new_width as usize * new_height as usize * 4];
    for y in 0..new_height {
        for x in 0..new_width {
            let dx = x as f32 - ncx;
            let dy = y as f32 - ncy;
            let sx = dx * cos + dy * sin + cx;
            let sy = -dx * sin + dy * cos + cy;
            put(&mut pixels, new_width, x, y, sample(image, sx, sy));
        }
    }
    RgbaImage::new(new_width, new_height, pixels).expect("dimensions match the buffer")
}

/// Bilinear sample; outside the image reads as transparent.
pub(crate) fn sample(image: &RgbaImage, x: f32, y: f32) -> [u8; 4] {
    let (max_x, max_y) = ((image.width - 1) as f32, (image.height - 1) as f32);
    if x < 0.0 || y < 0.0 || x > max_x || y > max_y {
        return [0, 0, 0, 0];
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width - 1);
    let y1 = (y0 + 1).min(image.height - 1);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);

    let p00 = get(image, x0, y0);
    let p10 = get(image, x1, y0);
    let p01 = get(image, x0, y1);
    let p11 = get(image, x1, y1);

    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = mix(p00[channel], p10[channel], fx);
        let bottom = mix(p01[channel], p11[channel], fx);
        out[channel] = mix_f(top, bottom, fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn mix(a: u8, b: u8, t: f32) -> f32 {
    a as f32 + (b as f32 - a as f32) * t
}

fn mix_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
