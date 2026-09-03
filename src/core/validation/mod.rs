//! Step 4 — semantic validation for `AimtEntity`.

pub mod error;
pub mod rules;
pub mod validate;

pub use error::{ValidationError, ValidationErrorKind};
pub use validate::validate;
