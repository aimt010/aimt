use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::reader::{self, ReaderError};
use crate::writer::{self, WriterError};

// ---------------------------------------------------------------------------
// Error types — structured, not String
// ---------------------------------------------------------------------------

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
pub struct WorkspaceFileError {
    pub path: PathBuf,
    pub error: ReaderError,
}

#[derive(Debug)]
pub struct WriterFileError {
    pub path: PathBuf,
    pub error: WriterError,
}

#[derive(Debug)]
pub enum WorkspaceError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    FileErrors(Vec<WorkspaceFileError>),
    Write(WriterFileError),
}

impl WorkspaceError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::FileErrors(_) => "FileErrors",
            Self::Write(_) => "Write",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::FileErrors(_) => None,
            Self::Write(e) => Some(&e.path),
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

impl std::fmt::Display for WorkspaceFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileError at {}: {}", self.path.display(), self.error)
    }
}
impl std::error::Error for WorkspaceFileError {}

impl std::fmt::Display for WriterFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WriteError at {}: {}", self.path.display(), self.error)
    }
}
impl std::error::Error for WriterFileError {}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::FileErrors(vec) => {
                write!(f, "FileErrors {} errors: ", vec.len())?;
                for (i, e) in vec.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{}", e)?;
                }
                Ok(())
            }
            Self::Write(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for WorkspaceError {}

// ---------------------------------------------------------------------------
// Workspace entity with provenance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEntity {
    pub path: PathBuf,
    pub entity: AimtEntity,
}

// ---------------------------------------------------------------------------
// Internal helper — validate workspace is existing directory
// ---------------------------------------------------------------------------

fn validate_workspace(workspace: &Path) -> Result<(), WorkspaceError> {
    match std::fs::metadata(workspace) {
        Ok(md) => {
            if md.is_dir() {
                Ok(())
            } else {
                Err(WorkspaceError::InvalidWorkspace(InvalidWorkspaceError {
                    path: workspace.to_path_buf(),
                    kind: std::io::ErrorKind::Other,
                    message: "workspace path is not a directory".to_string(),
                }))
            }
        }
        Err(e) => Err(WorkspaceError::Io(IoError {
            path: workspace.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })),
    }
}

// ---------------------------------------------------------------------------
// Discovery — immediate .pmap files only, sorted lexically
// ---------------------------------------------------------------------------

/// Discover immediate `.pmap` files in `workspace` directory, sorted lexically.
/// Does NOT recurse, does NOT follow symlinks, does NOT infer IDs.
pub fn discover(workspace: &Path) -> Result<Vec<PathBuf>, WorkspaceError> {
    validate_workspace(workspace)?;

    let entries = std::fs::read_dir(workspace).map_err(|e| {
        WorkspaceError::Io(IoError {
            path: workspace.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })
    })?;

    let mut candidates: Vec<PathBuf> = Vec::new();

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(e) => {
                return Err(WorkspaceError::Io(IoError {
                    path: workspace.to_path_buf(),
                    kind: e.kind(),
                    message: e.to_string(),
                }));
            }
        };

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                return Err(WorkspaceError::Io(IoError {
                    path: workspace.to_path_buf(),
                    kind: e.kind(),
                    message: e.to_string(),
                }));
            }
        };

        if ft.is_symlink() {
            continue;
        }

        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with('.') {
            continue;
        }

        if ft.is_dir() {
            continue;
        }

        if !ft.is_file() {
            continue;
        }

        let path = entry.path();
        match path.extension() {
            Some(ext) if ext == "pmap" => candidates.push(path),
            _ => continue,
        }
    }

    candidates.sort();
    Ok(candidates)
}

// ---------------------------------------------------------------------------
// Read — orchestrate discover + reader::read, collect errors (policy B)
// ---------------------------------------------------------------------------

/// Read all immediate `.pmap` files in `workspace`, returning validated entities
/// with provenance. Collects all per-file errors (policy B) sorted lexically.
pub fn read(workspace: &Path) -> Result<Vec<WorkspaceEntity>, WorkspaceError> {
    let paths = discover(workspace)?;

    let mut oks: Vec<WorkspaceEntity> = Vec::new();
    let mut errs: Vec<WorkspaceFileError> = Vec::new();

    for p in &paths {
        match reader::read(p) {
            Ok(entity) => oks.push(WorkspaceEntity {
                path: p.clone(),
                entity,
            }),
            Err(e) => errs.push(WorkspaceFileError {
                path: p.clone(),
                error: e,
            }),
        }
    }

    if errs.is_empty() {
        Ok(oks)
    } else {
        errs.sort_by(|a, b| a.path.cmp(&b.path));
        Err(WorkspaceError::FileErrors(errs))
    }
}

// ---------------------------------------------------------------------------
// Write — absolute-path-only thin wrapper around writer::write
// ---------------------------------------------------------------------------

/// Write `entity` to exactly one absolute `.pmap` file at `path`.
/// Delegates to `writer::write`. Does not create parent directories,
/// does not infer ID from filename.
pub fn write(path: &Path, entity: &AimtEntity) -> Result<(), WorkspaceError> {
    if !path.is_absolute() {
        return Err(WorkspaceError::InvalidWorkspace(InvalidWorkspaceError {
            path: path.to_path_buf(),
            kind: std::io::ErrorKind::InvalidInput,
            message: format!("path must be absolute: {}", path.display()),
        }));
    }

    writer::write(path, entity).map_err(|e| {
        WorkspaceError::Write(WriterFileError {
            path: path.to_path_buf(),
            error: e,
        })
    })
}
