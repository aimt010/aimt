//! Step 2 parser — deterministic `.pmap` parser.
//! `src/syntax.rs` is the single source of syntax truth.

pub mod error;
pub mod line;
pub mod parse;
pub mod types;

pub use crate::syntax::{Level as ParsedLevel, Span};
pub use error::{ErrorKind, ParseError};
pub use types::{ParsedField, ParsedPmap, ParsedRelation};

pub use parse::{parse, parse_bytes};
