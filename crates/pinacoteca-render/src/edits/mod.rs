//! Non-destructive edit pipeline: replays an [`EditStack`] over decoded pixels.

pub mod geometry;
pub mod tone;

use pinacoteca_core::edit::{EditOp, EditStack};
use pinacoteca_decoders::RgbaImage;

/// Replays `stack` over `image`, in order.
pub fn apply(image: RgbaImage, stack: &EditStack) -> RgbaImage {
    let mut current = image;
    for op in stack.ops() {
        current = apply_op(current, *op);
    }
    current
}

/// Applies a single operation.
pub fn apply_op(image: RgbaImage, op: EditOp) -> RgbaImage {
    match op {
        EditOp::Rotate(rotation) => geometry::rotate(&image, rotation),
        EditOp::Crop(rect) => geometry::crop(&image, rect),
        EditOp::Straighten(hundredths) => geometry::straighten(&image, hundredths),
        EditOp::AutoFix => tone::auto_fix(&image),
        EditOp::Brightness(value) => tone::brightness(&image, value),
        EditOp::Contrast(value) => tone::contrast(&image, value),
        EditOp::Saturation(value) => tone::saturation(&image, value),
        EditOp::Warmth(value) => tone::warmth(&image, value),
        EditOp::RedEye(rect) => tone::red_eye(&image, rect),
        EditOp::Sharpen(strength) => tone::sharpen(&image, strength),
        EditOp::Effect(effect) => tone::effect(&image, effect),
    }
}

/// Reads a pixel.
pub(crate) fn get(image: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y as usize) * (image.width as usize) + x as usize) * 4;
    [
        image.pixels[i],
        image.pixels[i + 1],
        image.pixels[i + 2],
        image.pixels[i + 3],
    ]
}

/// Writes a pixel into a flat buffer.
pub(crate) fn put(pixels: &mut [u8], width: u32, x: u32, y: u32, pixel: [u8; 4]) {
    let i = ((y as usize) * (width as usize) + x as usize) * 4;
    pixels[i..i + 4].copy_from_slice(&pixel);
}

/// A mutable pass over every pixel, in place.
pub(crate) fn map<F: Fn([u8; 4]) -> [u8; 4]>(image: &RgbaImage, f: F) -> RgbaImage {
    let mut pixels = Vec::with_capacity(image.pixels.len());
    for chunk in image.pixels.as_chunks::<4>().0 {
        pixels.extend_from_slice(&f(*chunk));
    }
    RgbaImage::new(image.width, image.height, pixels).expect("dimensions match the buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinacoteca_core::edit::{EditOp, Effect, Rotation};

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        for pixel in pixels.as_chunks_mut::<4>().0.iter_mut() {
            pixel.copy_from_slice(&rgba);
        }
        RgbaImage::new(width, height, pixels).unwrap()
    }

    #[test]
    fn an_empty_stack_is_the_identity() {
        let image = solid(3, 2, [10, 20, 30, 255]);
        let out = apply(image.clone(), &EditStack::new());
        assert_eq!(out.pixels, image.pixels);
    }

    #[test]
    fn rotate_cw90_swaps_dimensions_and_pixels() {
        let mut left = solid(1, 1, [10, 0, 0, 255]);
        let mut right = solid(1, 1, [20, 0, 0, 255]);
        let mut pixels = Vec::new();
        pixels.append(&mut left.pixels);
        pixels.append(&mut right.pixels);
        let image = RgbaImage::new(2, 1, pixels).unwrap();

        let out = geometry::rotate(&image, Rotation::Cw90);
        assert_eq!((out.width, out.height), (1, 2));
        assert_eq!(get(&out, 0, 0), [10, 0, 0, 255]);
        assert_eq!(get(&out, 0, 1), [20, 0, 0, 255]);
    }

    #[test]
    fn crop_takes_the_requested_window() {
        let image = solid(4, 4, [5, 5, 5, 255]);
        let out = geometry::crop(&image, (1, 1, 2, 2));
        assert_eq!((out.width, out.height), (2, 2));
    }

    #[test]
    fn crop_clamps_an_out_of_range_rectangle() {
        let image = solid(4, 4, [5, 5, 5, 255]);
        let out = geometry::crop(&image, (3, 3, 99, 99));
        assert_eq!((out.width, out.height), (1, 1));
    }

    #[test]
    fn straighten_zero_is_the_identity_and_an_angle_grows_the_canvas() {
        let image = solid(4, 4, [5, 5, 5, 255]);
        let same = geometry::straighten(&image, 0);
        assert_eq!(same.pixels, image.pixels);

        let turned = geometry::straighten(&image, 1000);
        assert!(turned.width > image.width && turned.height > image.height);
    }

    #[test]
    fn grayscale_makes_every_channel_equal() {
        let image = solid(2, 2, [200, 100, 50, 255]);
        let out = tone::effect(&image, Effect::Grayscale);
        let pixel = get(&out, 0, 0);
        assert_eq!(pixel[0], pixel[1]);
        assert_eq!(pixel[1], pixel[2]);
    }

    #[test]
    fn sepia_is_warm() {
        let image = solid(2, 2, [200, 100, 50, 255]);
        let out = tone::effect(&image, Effect::Sepia);
        let pixel = get(&out, 0, 0);
        assert!(pixel[0] > pixel[1] && pixel[1] > pixel[2], "{pixel:?}");
    }

    #[test]
    fn brightness_raises_and_lowers_every_channel() {
        let image = solid(2, 2, [100, 100, 100, 255]);
        assert!(get(&tone::brightness(&image, 50), 0, 0)[0] > 100);
        assert!(get(&tone::brightness(&image, -100), 0, 0)[0] < 100);
    }

    #[test]
    fn contrast_pushes_away_from_mid_grey() {
        let image = solid(2, 2, [140, 140, 140, 255]);
        assert!(get(&tone::contrast(&image, 50), 0, 0)[0] > 140);
        assert!(
            get(
                &tone::contrast(&solid(2, 2, [100, 100, 100, 255]), 50),
                0,
                0
            )[0] < 100
        );
    }

    #[test]
    fn full_desaturation_equals_grayscale() {
        let image = solid(2, 2, [200, 100, 50, 255]);
        let out = tone::saturation(&image, -100);
        let pixel = get(&out, 0, 0);
        assert_eq!(pixel[0], pixel[1]);
        assert_eq!(pixel[1], pixel[2]);
    }

    #[test]
    fn warmth_trades_red_against_blue() {
        let image = solid(2, 2, [100, 100, 100, 255]);
        let warm = get(&tone::warmth(&image, 100), 0, 0);
        assert!(warm[0] > 100 && warm[2] < 100, "{warm:?}");
        let cool = get(&tone::warmth(&image, -100), 0, 0);
        assert!(cool[0] < 100 && cool[2] > 100, "{cool:?}");
    }

    #[test]
    fn auto_fix_stretches_a_flat_image_to_the_full_range() {
        let mut pixels = Vec::new();
        for value in [80u8, 90, 110, 120] {
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
        let image = RgbaImage::new(4, 1, pixels).unwrap();
        let out = tone::auto_fix(&image);
        assert_eq!(get(&out, 0, 0)[0], 0);
        assert_eq!(get(&out, 3, 0)[0], 255);
    }

    #[test]
    fn sharpen_leaves_flat_pixels_alone() {
        let image = solid(3, 3, [120, 120, 120, 255]);
        assert_eq!(tone::sharpen(&image, 0).pixels, image.pixels);
        assert_eq!(tone::sharpen(&image, 10).pixels, image.pixels);
    }

    #[test]
    fn red_eye_neutralises_red_and_spares_everything_else() {
        let mut pixels = Vec::new();
        pixels.extend_from_slice(&[200, 50, 50, 255]);
        pixels.extend_from_slice(&[50, 50, 50, 255]);
        let image = RgbaImage::new(2, 1, pixels).unwrap();
        let out = tone::red_eye(&image, (0, 0, 2, 1));
        assert_eq!(get(&out, 0, 0)[0], 50, "red takes the green/blue mean");
        assert_eq!(get(&out, 1, 0), [50, 50, 50, 255], "grey is untouched");
    }

    #[test]
    fn apply_replays_operations_in_order() {
        let image = solid(2, 1, [10, 0, 0, 255]);
        let mut stack = EditStack::new();
        stack.push(EditOp::Rotate(Rotation::Cw90));
        stack.push(EditOp::Crop((0, 0, 1, 1)));
        let out = apply(image, &stack);
        assert_eq!((out.width, out.height), (1, 1));
    }

    #[test]
    fn apply_op_matches_the_single_operation() {
        let image = solid(2, 2, [200, 100, 50, 255]);
        let via_op = apply_op(image.clone(), EditOp::Effect(Effect::Grayscale));
        let direct = tone::effect(&image, Effect::Grayscale);
        assert_eq!(via_op.pixels, direct.pixels);
    }
}
