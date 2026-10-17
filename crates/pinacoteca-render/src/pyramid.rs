//! Multi-resolution image pyramid.
//!
//! A [`Pyramid`] holds a base image plus successive half-resolution levels. The
//! renderer chooses the level whose resolution is closest to (but not smaller
//! than) the on-screen size, so the GPU samples a texture that is at most ~2×
//! larger than the displayed image. Switching level is O(1), which is what makes
//! zoom feel instant: no re-decode and no full-image resample on zoom.

use pinacoteca_decoders::RgbaImage;

/// Smallest side, in pixels, at which pyramid construction stops by default.
pub const DEFAULT_MIN_SIDE: u32 = 64;

/// A quad-tree-like set of half-resolution levels of one image.
///
/// Level 0 is the full-resolution base; each following level has (approximately)
/// half the width and height of the previous one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pyramid {
    levels: Vec<RgbaImage>,
}

impl Pyramid {
    /// Builds a pyramid from `base`, halving until the smallest side is at or
    /// below `min_side`.
    ///
    /// The base is always present, so the pyramid has at least one level.
    pub fn build(base: RgbaImage, min_side: u32) -> Self {
        let min_side = min_side.max(1);
        let mut levels = vec![base];
        loop {
            let last = levels.last().expect("base level is always present");
            if last.width.min(last.height) <= min_side {
                break;
            }
            levels.push(downsample_half(last));
        }
        Self { levels }
    }

    /// Builds a pyramid with [`DEFAULT_MIN_SIDE`].
    pub fn from_base(base: RgbaImage) -> Self {
        Self::build(base, DEFAULT_MIN_SIDE)
    }

    /// All levels, base first, smallest last.
    pub fn levels(&self) -> &[RgbaImage] {
        &self.levels
    }

    /// The full-resolution base level.
    pub fn base(&self) -> &RgbaImage {
        &self.levels[0]
    }

    /// Number of levels.
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Whether the pyramid has no levels (never true in practice).
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// The level to sample when the image is displayed at `scale`, where `scale`
    /// is device pixels per image pixel.
    ///
    /// - `scale >= 1.0` (100% or closer) → level 0 (full resolution).
    /// - otherwise the largest level that is still not smaller than the display,
    ///   giving at most ~2× GPU minification and therefore a sharp result.
    pub fn level_for_scale(&self, scale: f32) -> usize {
        let last = self.levels.len() - 1;
        if !scale.is_finite() || scale >= 1.0 {
            return 0;
        }
        if scale <= 0.0 {
            return last;
        }
        let level = (-scale.log2()).floor() as i64;
        level.clamp(0, last as i64) as usize
    }

    /// Bytes held by every level together.
    pub fn bytes(&self) -> usize {
        self.levels.iter().map(|level| level.pixels.len()).sum()
    }

    /// Total pyramid memory as a multiple of the base level.
    ///
    /// For a full halving series this approaches 4/3.
    pub fn memory_overhead(&self) -> f32 {
        self.bytes() as f32 / self.base().pixels.len().max(1) as f32
    }
}

/// Halves an image with a 2×2 box filter, rounding odd dimensions up by clamping
/// the edge sample.
fn downsample_half(src: &RgbaImage) -> RgbaImage {
    let width = (src.width / 2).max(1);
    let height = (src.height / 2).max(1);
    let mut pixels = vec![0u8; width as usize * height as usize * 4];

    for y in 0..height {
        for x in 0..width {
            let mut acc = [0u32; 4];
            let mut count = 0u32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = (x * 2 + dx).min(src.width - 1);
                    let sy = (y * 2 + dy).min(src.height - 1);
                    let i = (sy as usize * src.width as usize + sx as usize) * 4;
                    for (channel, value) in acc.iter_mut().enumerate() {
                        *value += src.pixels[i + channel] as u32;
                    }
                    count += 1;
                }
            }
            let o = (y as usize * width as usize + x as usize) * 4;
            for (channel, value) in acc.iter().enumerate() {
                pixels[o + channel] = (value / count) as u8;
            }
        }
    }

    RgbaImage::new(width, height, pixels).expect("buffer matches dimensions")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
        RgbaImage::new(width, height, pixels).unwrap()
    }

    #[test]
    fn build_halves_until_min_side() {
        let pyramid = Pyramid::build(solid(512, 256, [10, 20, 30, 255]), 64);
        let dims: Vec<(u32, u32)> = pyramid
            .levels()
            .iter()
            .map(|l| (l.width, l.height))
            .collect();
        // 512x256 -> 256x128 -> 128x64 (stop: min side 64 <= 64)
        assert_eq!(dims, vec![(512, 256), (256, 128), (128, 64)]);
    }

    #[test]
    fn base_is_level_zero_and_never_empty() {
        let pyramid = Pyramid::from_base(solid(32, 32, [1, 2, 3, 255]));
        assert_eq!(pyramid.len(), 1);
        assert!(!pyramid.is_empty());
        assert_eq!((pyramid.base().width, pyramid.base().height), (32, 32));
    }

    #[test]
    fn downsampling_averages_the_2x2_block() {
        let mut pixels = Vec::new();
        for value in [0u8, 40, 80, 120] {
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
        let src = RgbaImage::new(2, 2, pixels).unwrap();
        let pyramid = Pyramid::build(src, 1);
        let level1 = &pyramid.levels()[1];
        assert_eq!((level1.width, level1.height), (1, 1));
        // average of 0, 40, 80, 120 = 60
        assert_eq!(level1.pixels[0], 60);
        assert_eq!(level1.pixels[3], 255);
    }

    #[test]
    fn level_selection_matches_zoom() {
        let pyramid = Pyramid::build(solid(1024, 1024, [0, 0, 0, 255]), 64);
        assert_eq!(pyramid.len(), 5); // 1024, 512, 256, 128, 64
        assert_eq!(pyramid.level_for_scale(2.0), 0); // zoomed in
        assert_eq!(pyramid.level_for_scale(1.0), 0); // 100%
        assert_eq!(pyramid.level_for_scale(0.5), 1);
        assert_eq!(pyramid.level_for_scale(0.25), 2);
        assert_eq!(pyramid.level_for_scale(0.3), 1); // floor(-log2 0.3) = 1
        assert_eq!(pyramid.level_for_scale(0.001), 4); // clamped to smallest
        assert_eq!(pyramid.level_for_scale(f32::NAN), 0);
    }

    #[test]
    fn memory_overhead_approaches_four_thirds() {
        let pyramid = Pyramid::build(solid(512, 512, [0, 0, 0, 255]), 64);
        assert!(pyramid.memory_overhead() < 4.0 / 3.0 + 0.02);
        assert!(pyramid.memory_overhead() >= 1.0);
    }

    #[test]
    fn odd_dimensions_round_up_without_panicking() {
        let pyramid = Pyramid::build(solid(101, 77, [5, 5, 5, 255]), 16);
        assert!(pyramid.len() > 1);
        for level in pyramid.levels() {
            assert!(level.width >= 1 && level.height >= 1);
            assert_eq!(
                level.pixels.len(),
                level.width as usize * level.height as usize * 4
            );
        }
    }
}
