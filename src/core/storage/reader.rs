//! Step 5 — single-file Reader.
//! Orchestrates `std::fs::read` → `parser::parse_bytes` → `model::AimtEntity` → `validation::validate`.
//! No workspace discovery, no graph, no reference resolution.

use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::parser::{self, ParseError};
use crate::validation::{ValidationError, validate};

// ---------------------------------------------------------------------------
// IoError — filesystem/read failure only
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct IoError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

// ---------------------------------------------------------------------------
// ReaderError — three distinct categories
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ReaderError {
    Io(IoError),
    Parse(ParseError),
    Validation(Vec<ValidationError>),
}

impl ReaderError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::Parse(_) => "Parse",
            Self::Validation(_) => "Validation",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::Parse(_) => None,
            Self::Validation(_) => None,
        }
    }
}

impl std::fmt::Display for ReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Io {:?} at {}: {}", e.kind, e.path.display(), e.message),
            Self::Parse(e) => write!(
                f,
                "Parse {} at {}:{} — {}",
                e.kind.as_str(),
                e.span.start_line,
                e.span.start_col,
                e.message
            ),
            Self::Validation(vec) => {
                write!(f, "Validation {} errors: ", vec.len())?;
                for (i, e) in vec.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(
                        f,
                        "{} field={:?} at {}:{} — {}",
                        e.kind.as_str(),
                        e.field,
                        e.span.start_line,
                        e.span.start_col,
                        e.message
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ReaderError {}

// ---------------------------------------------------------------------------
// Public API — single-file reader
// ---------------------------------------------------------------------------

/// Read exactly one `.pmap` file at `path`, parse, convert, validate.
///
/// Pipeline: `std::fs::read(path)` → `parser::parse_bytes` → `AimtEntity::from_parsed` → `validation::validate`.
/// Never uses lossy UTF-8 conversion, never normalizes or repairs file contents.
pub fn read(path: &Path) -> Result<AimtEntity, ReaderError> {
    // ① filesystem read — single boundary, preserve original ErrorKind
    let bytes = std::fs::read(path).map_err(|e| {
        ReaderError::Io(IoError {
            path: path.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })
    })?;

    // ② parser — owns UTF-8/BOM/newline/tab syntax
    let parsed = parser::parse_bytes(&bytes).map_err(ReaderError::Parse)?;

    // ③ model — preserves header/body/relations/Span, unknown fields
    let entity = AimtEntity::from_parsed(parsed);

    // ④ validation — Required/Optional/NotAllowed, UnknownField, ID shape, relation rules
    validate(&entity).map_err(ReaderError::Validation)?;

    Ok(entity)
}
