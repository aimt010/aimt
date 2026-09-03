//! Step 1 syntax contract — constants and helpers derived from
//! `docs/specification/pmap.md`, `levels.md`, and `fields.md`.

pub mod field;
pub mod level;
pub mod span;

pub use field::{
    BODY_MARKER, HEADER_MARKER, INDENT_WIDTH, RELATION_MARKER, RELATIONS_FIELD, SECTION_MARKERS,
    indent_for_depth, is_lowercase_name, is_supported_section, is_tab_present, is_valid_indent,
};
pub use level::{
    Level, SUPPORTED_LEVEL_MARKERS, SUPPORTED_LEVELS, is_supported_level, is_supported_level_marker,
};
pub use span::Span;
