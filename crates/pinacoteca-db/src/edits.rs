//! Persistence for the non-destructive edit stacks.
//!
//! One row per operation in `edit_ops`, keyed by `(photo_id, position)`. The
//! `kind` column keeps a stack readable in SQL; `payload` is the JSON form of
//! the whole operation, so a new variant only needs a `kind_of` arm.

use pinacoteca_core::edit::{EditOp, EditStack};
use rusqlite::params;

use crate::{Database, DbError};

/// The variant name stored in `edit_ops.kind`.
pub fn kind_of(op: &EditOp) -> &'static str {
    match op {
        EditOp::Rotate(_) => "rotate",
        EditOp::Crop(_) => "crop",
        EditOp::Straighten(_) => "straighten",
        EditOp::AutoFix => "autofix",
        EditOp::Brightness(_) => "brightness",
        EditOp::Contrast(_) => "contrast",
        EditOp::Saturation(_) => "saturation",
        EditOp::Warmth(_) => "warmth",
        EditOp::RedEye(_) => "redeye",
        EditOp::Sharpen(_) => "sharpen",
        EditOp::Effect(_) => "effect",
    }
}

impl Database {
    /// Loads a photo's edit stack, in order. An unknown path is an empty stack.
    pub fn load_edit_stack(&self, photo_path: &str) -> Result<EditStack, DbError> {
        let Some(photo_id) = self.photo_id_by_path(photo_path)? else {
            return Ok(EditStack::new());
        };
        let mut statement = self
            .connection()
            .prepare("SELECT payload FROM edit_ops WHERE photo_id = ?1 ORDER BY position")?;
        let rows = statement.query_map(params![photo_id], |row| row.get::<_, String>(0))?;

        let mut stack = EditStack::new();
        for payload in rows {
            let payload = payload?;
            let op: EditOp = serde_json::from_str(&payload)
                .map_err(|error| DbError::EditPayload(error.to_string()))?;
            stack.push(op);
        }
        Ok(stack)
    }

    /// Replaces the stored stack for a photo. An unknown path is a no-op.
    pub fn save_edit_stack(&self, photo_path: &str, stack: &EditStack) -> Result<(), DbError> {
        let Some(photo_id) = self.photo_id_by_path(photo_path)? else {
            return Ok(());
        };
        let transaction = self.connection().unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM edit_ops WHERE photo_id = ?1",
            params![photo_id],
        )?;
        for (position, op) in stack.ops().iter().enumerate() {
            let payload = serde_json::to_string(op)
                .map_err(|error| DbError::EditPayload(error.to_string()))?;
            transaction.execute(
                "INSERT INTO edit_ops (photo_id, position, kind, payload) VALUES (?1, ?2, ?3, ?4)",
                params![photo_id, position as i64, kind_of(op), payload],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Drops every stored operation for a photo.
    pub fn clear_edit_stack(&self, photo_path: &str) -> Result<(), DbError> {
        if let Some(photo_id) = self.photo_id_by_path(photo_path)? {
            self.connection().execute(
                "DELETE FROM edit_ops WHERE photo_id = ?1",
                params![photo_id],
            )?;
        }
        Ok(())
    }

    /// How many photos carry at least one edit.
    pub fn edited_photo_count(&self) -> Result<i64, DbError> {
        Ok(self.connection().query_row(
            "SELECT COUNT(DISTINCT photo_id) FROM edit_ops",
            [],
            |row| row.get(0),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinacoteca_core::edit::{Effect, Rotation};

    fn seed(db: &crate::Database, path: &str) {
        db.upsert_folder("C:/pics").expect("folder");
        db.upsert_photo_facts(path, Some(1), 10, 1, Some("jpg"))
            .expect("facts");
    }

    fn sample_stack() -> EditStack {
        let mut stack = EditStack::new();
        stack.push(EditOp::Rotate(Rotation::Cw90));
        stack.push(EditOp::Brightness(20));
        stack.push(EditOp::Effect(Effect::Sepia));
        stack
    }

    #[test]
    fn a_saved_stack_loads_back_in_order() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/a.jpg");
        db.save_edit_stack("C:/pics/a.jpg", &sample_stack())
            .expect("save");

        assert_eq!(
            db.load_edit_stack("C:/pics/a.jpg").expect("load"),
            sample_stack()
        );
        assert_eq!(db.edited_photo_count().expect("count"), 1);
    }

    #[test]
    fn an_unedited_photo_loads_an_empty_stack() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/a.jpg");
        assert!(
            db.load_edit_stack("C:/pics/a.jpg")
                .expect("load")
                .is_empty()
        );
    }

    #[test]
    fn an_unknown_path_loads_an_empty_stack() {
        let db = crate::Database::open_in_memory().expect("db");
        assert!(
            db.load_edit_stack("C:/pics/missing.jpg")
                .expect("load")
                .is_empty()
        );
    }

    #[test]
    fn saving_replaces_rather_than_appends() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/a.jpg");

        db.save_edit_stack("C:/pics/a.jpg", &sample_stack())
            .expect("save");
        let mut shorter = EditStack::new();
        shorter.push(EditOp::AutoFix);
        db.save_edit_stack("C:/pics/a.jpg", &shorter)
            .expect("save again");

        assert_eq!(db.load_edit_stack("C:/pics/a.jpg").expect("load"), shorter);
    }

    #[test]
    fn clearing_removes_every_operation() {
        let db = crate::Database::open_in_memory().expect("db");
        seed(&db, "C:/pics/a.jpg");
        db.save_edit_stack("C:/pics/a.jpg", &sample_stack())
            .expect("save");

        db.clear_edit_stack("C:/pics/a.jpg").expect("clear");
        assert!(
            db.load_edit_stack("C:/pics/a.jpg")
                .expect("load")
                .is_empty()
        );
        assert_eq!(db.edited_photo_count().expect("count"), 0);
    }

    #[test]
    fn saving_an_unknown_photo_is_a_no_op() {
        let db = crate::Database::open_in_memory().expect("db");
        db.save_edit_stack("C:/pics/missing.jpg", &sample_stack())
            .expect("no-op");
        assert_eq!(db.edited_photo_count().expect("count"), 0);
    }

    #[test]
    fn every_variant_has_a_readable_kind() {
        let ops = [
            EditOp::Rotate(Rotation::Cw180),
            EditOp::Crop((1, 2, 3, 4)),
            EditOp::Straighten(-250),
            EditOp::AutoFix,
            EditOp::Brightness(1),
            EditOp::Contrast(1),
            EditOp::Saturation(1),
            EditOp::Warmth(1),
            EditOp::RedEye((0, 0, 5, 5)),
            EditOp::Sharpen(3),
            EditOp::Effect(Effect::Grayscale),
        ];
        for op in ops {
            assert!(!kind_of(&op).is_empty());
        }
    }
}
