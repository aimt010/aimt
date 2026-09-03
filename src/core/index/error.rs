use std::path::PathBuf;

use crate::syntax::{Level, Span};
use crate::workspace::WorkspaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityErrorKind {
    UnknownReference,
    InvalidParentTarget,
    InvalidFileTarget,
}

impl IntegrityErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnknownReference => "UnknownReference",
            Self::InvalidParentTarget => "InvalidParentTarget",
            Self::InvalidFileTarget => "InvalidFileTarget",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegrityError {
    pub kind: IntegrityErrorKind,
    pub field: Option<String>,
    pub id: String,
    pub target: String,
    pub level: Level,
    pub target_level: Option<Level>,
    pub span: Span,
    pub message: String,
}

impl IntegrityError {
    pub fn kind_str(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} field={:?} id='{}' target='{}' level={} target_level={:?} at {}:{} — {}",
            self.kind.as_str(),
            self.field,
            self.id,
            self.target,
            self.level.as_str(),
            self.target_level.map(|l| l.as_str()),
            self.span.start_line,
            self.span.start_col,
            self.message
        )
    }
}
impl std::error::Error for IntegrityError {}

#[derive(Debug, Clone)]
pub struct IoError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

#[derive(Debug)]
pub struct InvalidWorkspaceError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

#[derive(Debug)]
pub struct DuplicateIdError {
    pub id: String,
    pub paths: Vec<PathBuf>,
    pub message: String,
}

#[derive(Debug)]
pub enum IndexError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    DuplicateId(DuplicateIdError),
    Read(WorkspaceError),
}

impl IndexError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::DuplicateId(_) => "DuplicateId",
            Self::Read(_) => "Read",
        }
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::DuplicateId(e) => e.paths.first().map(|p| p.as_path()),
            Self::Read(e) => e.path(),
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Io {:?} at {}: {}",
            self.kind,
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for IoError {}

impl std::fmt::Display for InvalidWorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InvalidWorkspace {:?} at {}: {}",
            self.kind,
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for InvalidWorkspaceError {}

impl std::fmt::Display for DuplicateIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DuplicateId '{}' at [{}]: {}",
            self.id,
            self.paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.message
        )
    }
}
impl std::error::Error for DuplicateIdError {}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::DuplicateId(e) => write!(f, "{}", e),
            Self::Read(e) => write!(f, "Read {}", e),
        }
    }
}
impl std::error::Error for IndexError {}
