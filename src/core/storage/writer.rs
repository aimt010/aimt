//! Step 5B — single-file Writer.
//! Converts `AimtEntity` → canonical `.pmap` bytes and writes to one path.
//! No workspace discovery, no graph, no validation duplication.

use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::syntax::{
    BODY_MARKER, HEADER_MARKER, INDENT_WIDTH, RELATION_MARKER, indent_for_depth, is_lowercase_name,
};

// ---------------------------------------------------------------------------
// Errors — structured, not String
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct IoError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializeErrorKind {
    InvalidFieldName,
    InvalidLevel,
}

impl SerializeErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidFieldName => "InvalidFieldName",
            Self::InvalidLevel => "InvalidLevel",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SerializeError {
    pub kind: SerializeErrorKind,
    pub field: Option<String>,
    pub message: String,
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} field={:?} — {}",
            self.kind.as_str(),
            self.field,
            self.message
        )
    }
}
impl std::error::Error for SerializeError {}

#[derive(Debug)]
pub enum WriterError {
    Io(IoError),
    Serialize(SerializeError),
}

impl WriterError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::Serialize(_) => "Serialize",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::Serialize(_) => None,
        }
    }
}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Io {:?} at {}: {}", e.kind, e.path.display(), e.message),
            Self::Serialize(e) => write!(f, "Serialize {} — {}", e.kind.as_str(), e.message),
        }
    }
}
impl std::error::Error for WriterError {}

// ---------------------------------------------------------------------------
// Serialization — pure, deterministic, no I/O
// ---------------------------------------------------------------------------

fn serialize_field(
    out: &mut String,
    name: &str,
    value: &str,
    base_depth: usize,
) -> Result<(), WriterError> {
    if !is_lowercase_name(name) {
        return Err(WriterError::Serialize(SerializeError {
            kind: SerializeErrorKind::InvalidFieldName,
            field: Some(name.to_string()),
            message: format!("invalid field name '{}'", name),
        }));
    }
    let field_indent = " ".repeat(indent_for_depth(base_depth));
    let value_indent = " ".repeat(indent_for_depth(base_depth + 1));
    out.push_str(&format!("{}{}:\n", field_indent, name));
    if value.is_empty() {
        return Ok(());
    }
    for segment in value.split('\n') {
        if segment.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&format!("{}{}\n", value_indent, segment));
        }
    }
    Ok(())
}

/// Serialize `entity` to canonical `.pmap` `String` (UTF-8, LF, single trailing LF).
///
/// Deterministic: same `AimtEntity` → same `String` (header/body/relations order preserved, no sorting).
pub fn serialize(entity: &AimtEntity) -> Result<String, WriterError> {
    // Validate field names eagerly (defensive)
    for field in entity.header.iter().chain(entity.body.iter()) {
        if !is_lowercase_name(&field.name) {
            return Err(WriterError::Serialize(SerializeError {
                kind: SerializeErrorKind::InvalidFieldName,
                field: Some(field.name.clone()),
                message: format!("invalid field name '{}'", field.name),
            }));
        }
    }
    for rel in &entity.relations {
        for field in &rel.fields {
            if !is_lowercase_name(&field.name) {
                return Err(WriterError::Serialize(SerializeError {
                    kind: SerializeErrorKind::InvalidFieldName,
                    field: Some(field.name.clone()),
                    message: format!("invalid field name '{}' inside relation", field.name),
                }));
            }
        }
    }

    let mut out = String::new();

    // @level at column 0
    out.push_str(&format!("{}\n", entity.level.marker()));
    out.push('\n');
    // #header
    out.push_str(&format!("{}\n", HEADER_MARKER));
    for field in &entity.header {
        serialize_field(&mut out, &field.name, &field.value, 1)?;
    }
    out.push('\n');
    // #body
    out.push_str(&format!("{}\n", BODY_MARKER));
    for field in &entity.body {
        serialize_field(&mut out, &field.name, &field.value, 1)?;
    }
    // relations — single field containing @relation blocks (omit if zero)
    if !entity.relations.is_empty() {
        let rel_field_indent = " ".repeat(indent_for_depth(1));
        let at_relation_indent = " ".repeat(indent_for_depth(2));
        out.push_str(&format!("{}relations:\n", rel_field_indent));
        for rel in &entity.relations {
            out.push_str(&format!("{}{}\n", at_relation_indent, RELATION_MARKER));
            for field in &rel.fields {
                // relation fields at depth 3 (indent 6), values at depth 4 (indent 8)
                if !is_lowercase_name(&field.name) {
                    return Err(WriterError::Serialize(SerializeError {
                        kind: SerializeErrorKind::InvalidFieldName,
                        field: Some(field.name.clone()),
                        message: format!("invalid field name '{}' inside relation", field.name),
                    }));
                }
                let f_indent = " ".repeat(indent_for_depth(3));
                let v_indent = " ".repeat(indent_for_depth(4));
                out.push_str(&format!("{}{}:\n", f_indent, field.name));
                if field.value.is_empty() {
                    continue;
                }
                for seg in field.value.split('\n') {
                    if seg.is_empty() {
                        out.push('\n');
                    } else {
                        out.push_str(&format!("{}{}\n", v_indent, seg));
                    }
                }
            }
        }
    }

    // Ensure exactly one trailing LF, no CRLF, no BOM
    if !out.ends_with('\n') {
        out.push('\n');
    }
    // Remove any extra trailing blank lines beyond the single final LF
    // Our construction never produces "\n\n" at EOF except for blank lines inside values,
    // but the last line is always a value or field at indent 2, so it ends with single LF.
    // Defensive: trim extra trailing empty lines beyond one, but keep single LF
    while out.ends_with("\n\n") {
        // Check if last two chars are \n\n and the preceding content ends with a value line
        // Canonical output should end with single LF, not blank line, so pop one
        // However we must not remove the blank line that is part of a multiline value's trailing? No, multiline trailing is trimmed.
        // So we can safely ensure single LF: if ends with "\n\n", pop one
        out.pop();
    }
    // Final invariant: exactly one LF at EOF, no CRLF
    debug_assert!(out.ends_with('\n') && !out.ends_with("\r\n") && !out.ends_with("\n\n"));
    // No tabs, use only 2-space indents (by construction)
    debug_assert!(!out.contains('\t'));
    // Verify indentation is multiple of INDENT_WIDTH for non-blank lines
    for (idx, line) in out.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        debug_assert!(
            indent % INDENT_WIDTH == 0,
            "line {} indent {} not multiple of {}",
            idx + 1,
            indent,
            INDENT_WIDTH
        );
    }

    Ok(out)
}

/// Serialize to `Vec<u8>` (UTF-8).
pub fn serialize_to_bytes(entity: &AimtEntity) -> Result<Vec<u8>, WriterError> {
    Ok(serialize(entity)?.into_bytes())
}

/// Write exactly one `.pmap` file at `path`.
///
/// Creates or truncates the file, writes canonical bytes, preserves `path` for diagnostics.
/// Does **not** create parent directories, does not walk directories, does not infer `id` from filename.
pub fn write(path: &Path, entity: &AimtEntity) -> Result<(), WriterError> {
    let bytes = serialize_to_bytes(entity)?;
    std::fs::write(path, bytes).map_err(|e| {
        WriterError::Io(IoError {
            path: path.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })
    })
}
