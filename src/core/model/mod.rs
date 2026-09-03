//! Step 3 — AIMT typed domain model.

pub mod convert;
pub mod entity;
pub mod relation;
pub mod value;

pub use crate::syntax::Level;
pub use entity::AimtEntity;
pub use relation::Relation;
pub use value::{EntityId, Field, ReferenceId};
