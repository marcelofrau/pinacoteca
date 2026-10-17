//! Non-destructive edit model.
//!
//! Editing in Pinacoteca never rewrites the original file. Each operation is
//! recorded as an [`EditOp`] in an ordered [`EditStack`], and the final image is
//! produced by applying the stack at render time. Undo is removing the last
//! operation; the originals on disk are byte-identical before and after editing.

use serde::{Deserialize, Serialize};

/// Rotation in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    /// 90 degrees clockwise.
    Cw90,
    /// 180 degrees.
    Cw180,
    /// 270 degrees clockwise (90 counter-clockwise).
    Cw270,
}

/// A rectangle in image pixel coordinates: `(x, y, width, height)`.
pub type Rect = (u32, u32, u32, u32);

/// A simple visual effect preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Desaturate completely.
    Grayscale,
    /// Warm monochrome tone.
    Sepia,
}

/// A single non-destructive edit operation.
///
/// Adjustment values are relative integers centered on zero; the concrete range
/// and mapping to pixels are fixed when rendering is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditOp {
    /// Rotate the image.
    Rotate(Rotation),
    /// Crop to a rectangle.
    Crop(Rect),
    /// Straighten by a small angle in hundredths of a degree.
    Straighten(i32),
    /// Automatic level and color correction.
    AutoFix,
    /// Brightness adjustment.
    Brightness(i32),
    /// Contrast adjustment.
    Contrast(i32),
    /// Saturation adjustment.
    Saturation(i32),
    /// Color temperature adjustment.
    Warmth(i32),
    /// Remove red-eye within a rectangle.
    RedEye(Rect),
    /// Sharpen, by strength.
    Sharpen(u8),
    /// Apply an effect preset.
    Effect(Effect),
}

/// An ordered, reversible list of operations applied to one photo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditStack {
    ops: Vec<EditOp>,
}

impl EditStack {
    /// Creates an empty stack, meaning "no edits".
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an operation to the end of the stack.
    pub fn push(&mut self, op: EditOp) {
        self.ops.push(op);
    }

    /// Removes and returns the most recently applied operation.
    pub fn undo(&mut self) -> Option<EditOp> {
        self.ops.pop()
    }

    /// The operations in application order.
    pub fn ops(&self) -> &[EditOp] {
        &self.ops
    }

    /// Whether the photo has no edits.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Number of operations in the stack.
    pub fn len(&self) -> usize {
        self.ops.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_removes_last_operation() {
        let mut stack = EditStack::new();
        stack.push(EditOp::Rotate(Rotation::Cw90));
        stack.push(EditOp::Brightness(10));

        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo(), Some(EditOp::Brightness(10)));
        assert_eq!(stack.undo(), Some(EditOp::Rotate(Rotation::Cw90)));
        assert_eq!(stack.undo(), None);
        assert!(stack.is_empty());
    }

    #[test]
    fn stack_preserves_application_order() {
        let mut stack = EditStack::new();
        stack.push(EditOp::Crop((0, 0, 10, 10)));
        stack.push(EditOp::Effect(Effect::Grayscale));

        assert_eq!(
            stack.ops(),
            &[
                EditOp::Crop((0, 0, 10, 10)),
                EditOp::Effect(Effect::Grayscale)
            ]
        );
    }

    #[test]
    fn edit_ops_serialize_to_json_and_back() {
        // edit_ops rows store one EditOp per row as JSON in `payload`.
        let op = EditOp::Rotate(Rotation::Cw270);
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&json).unwrap(), op);

        let op = EditOp::Straighten(-312);
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&json).unwrap(), op);

        let op = EditOp::Crop((10, 20, 30, 40));
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&json).unwrap(), op);

        let op = EditOp::Sharpen(8);
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&json).unwrap(), op);
    }
}
