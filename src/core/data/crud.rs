use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::validation::{ValidationError, ValidationErrorKind, validate};
use crate::workspace::{self, WorkspaceEntity, WorkspaceError};
use crate::writer::{self, WriterError};
// reuse: reader::read via workspace, writer::write, validation::validate

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
pub struct NotFoundError {
    pub id: String,
    pub workspace: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct DuplicateIdError {
    pub id: String,
    pub paths: Vec<PathBuf>,
    pub message: String,
}

#[derive(Debug)]
pub struct WriteError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug)]
pub enum CrudError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    Validation(Vec<ValidationError>),
    NotFound(NotFoundError),
    DuplicateId(DuplicateIdError),
    Read(WorkspaceError),
    Write(WriteError),
}

impl CrudError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::Validation(_) => "Validation",
            Self::NotFound(_) => "NotFound",
            Self::DuplicateId(_) => "DuplicateId",
            Self::Read(_) => "Read",
            Self::Write(_) => "Write",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::Validation(_) => None,
            Self::NotFound(e) => Some(&e.workspace),
            Self::DuplicateId(e) => e.paths.first().map(|p| p.as_path()),
            Self::Read(e) => e.path(),
            Self::Write(e) => Some(&e.path),
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::NotFound(e) => Some(&e.id),
            Self::DuplicateId(e) => Some(&e.id),
            _ => None,
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

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NotFound id '{}' in {}: {}",
            self.id,
            self.workspace.display(),
            self.message
        )
    }
}
impl std::error::Error for NotFoundError {}

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

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Write {:?} at {} field={:?}: {}",
            self.kind,
            self.path.display(),
            self.field,
            self.message
        )
    }
}
impl std::error::Error for WriteError {}

impl std::fmt::Display for CrudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::Validation(vec) => {
                write!(f, "Validation {} errors: ", vec.len())?;
                for (i, e) in vec.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{}", e)?;
                }
                Ok(())
            }
            Self::NotFound(e) => write!(f, "{}", e),
            Self::DuplicateId(e) => write!(f, "{}", e),
            Self::Read(e) => write!(f, "Read {}", e),
            Self::Write(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for CrudError {}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn map_workspace_error(err: WorkspaceError) -> CrudError {
    match err {
        WorkspaceError::Io(e) => CrudError::Io(IoError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
        WorkspaceError::InvalidWorkspace(e) => CrudError::InvalidWorkspace(InvalidWorkspaceError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
        WorkspaceError::FileErrors(_) => CrudError::Read(err),
        WorkspaceError::Write(e) => CrudError::Write(WriteError {
            path: e.path,
            kind: match &e.error {
                WriterError::Io(io) => io.kind,
                WriterError::Serialize(_) => std::io::ErrorKind::InvalidInput,
            },
            message: e.error.to_string(),
            field: match &e.error {
                WriterError::Serialize(se) => se.field.clone(),
                _ => None,
            },
        }),
    }
}

fn find_matches(workspace: &Path, id: &str) -> Result<Vec<WorkspaceEntity>, CrudError> {
    let entities = workspace::read(workspace).map_err(map_workspace_error)?;
    let mut matches: Vec<WorkspaceEntity> = entities
        .into_iter()
        .filter(|we| we.entity.id().map(|e| e.as_str() == id).unwrap_or(false))
        .collect();
    matches.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(matches)
}

fn validate_entity(entity: &AimtEntity) -> Result<(), CrudError> {
    if let Err(errs) = validate(entity) {
        return Err(CrudError::Validation(errs));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

/// Create a new entity in `workspace`. Validates, checks duplicate, writes `<id>.pmap`.
pub fn create(workspace: &Path, entity: &AimtEntity) -> Result<PathBuf, CrudError> {
    validate_entity(entity)?;

    let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
        CrudError::Validation(vec![ValidationError {
            kind: ValidationErrorKind::MissingRequiredField,
            field: Some("id".to_string()),
            span: entity.span.clone(),
            message: "missing required field 'id'".to_string(),
        }])
    })?;

    // Check workspace valid and collect duplicates / read errors
    let matches = find_matches(workspace, &id)?;
    if !matches.is_empty() {
        let paths = matches.into_iter().map(|m| m.path).collect::<Vec<_>>();
        return Err(CrudError::DuplicateId(DuplicateIdError {
            id: id.clone(),
            paths,
            message: format!("id '{}' already exists", id),
        }));
    }

    let target = workspace.join(format!("{}.pmap", id));
    if target.exists() {
        return Err(CrudError::DuplicateId(DuplicateIdError {
            id: id.clone(),
            paths: vec![target.clone()],
            message: format!("file '{}' already exists", target.display()),
        }));
    }

    writer::write(&target, entity).map_err(|e| match e {
        WriterError::Io(io) => CrudError::Write(WriteError {
            path: io.path,
            kind: io.kind,
            message: io.message,
            field: None,
        }),
        WriterError::Serialize(se) => CrudError::Write(WriteError {
            path: target.clone(),
            kind: std::io::ErrorKind::InvalidInput,
            message: se.message.clone(),
            field: se.field.clone(),
        }),
    })?;

    Ok(target)
}

/// Get entity by `id` from `workspace`.
pub fn get(workspace: &Path, id: &str) -> Result<WorkspaceEntity, CrudError> {
    if id.is_empty() {
        return Err(CrudError::NotFound(NotFoundError {
            id: id.to_string(),
            workspace: workspace.to_path_buf(),
            message: "id must not be empty".to_string(),
        }));
    }
    let matches = find_matches(workspace, id)?;
    match matches.len() {
        0 => Err(CrudError::NotFound(NotFoundError {
            id: id.to_string(),
            workspace: workspace.to_path_buf(),
            message: format!("id '{}' not found", id),
        })),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(CrudError::DuplicateId(DuplicateIdError {
            id: id.to_string(),
            paths: matches.into_iter().map(|m| m.path).collect(),
            message: format!("duplicate id '{}'", id),
        })),
    }
}

/// List all entities in `workspace`, sorted lexically by path.
pub fn list(workspace: &Path) -> Result<Vec<WorkspaceEntity>, CrudError> {
    workspace::read(workspace).map_err(map_workspace_error)
}

/// Update existing entity in `workspace`. Validates, finds existing by id, overwrites that file.
pub fn update(workspace: &Path, entity: &AimtEntity) -> Result<PathBuf, CrudError> {
    validate_entity(entity)?;

    let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
        CrudError::Validation(vec![ValidationError {
            kind: ValidationErrorKind::MissingRequiredField,
            field: Some("id".to_string()),
            span: entity.span.clone(),
            message: "missing required field 'id'".to_string(),
        }])
    })?;

    let matches = find_matches(workspace, &id)?;
    match matches.len() {
        0 => Err(CrudError::NotFound(NotFoundError {
            id: id.clone(),
            workspace: workspace.to_path_buf(),
            message: format!("id '{}' not found for update", id),
        })),
        1 => {
            let existing_path = matches.into_iter().next().unwrap().path;
            writer::write(&existing_path, entity).map_err(|e| match e {
                WriterError::Io(io) => CrudError::Write(WriteError {
                    path: io.path,
                    kind: io.kind,
                    message: io.message,
                    field: None,
                }),
                WriterError::Serialize(se) => CrudError::Write(WriteError {
                    path: existing_path.clone(),
                    kind: std::io::ErrorKind::InvalidInput,
                    message: se.message.clone(),
                    field: se.field.clone(),
                }),
            })?;
            Ok(existing_path)
        }
        _ => Err(CrudError::DuplicateId(DuplicateIdError {
            id: id.clone(),
            paths: matches.into_iter().map(|m| m.path).collect(),
            message: format!("duplicate id '{}' cannot update", id),
        })),
    }
}

/// Delete entity by `id` from `workspace`.
pub fn delete(workspace: &Path, id: &str) -> Result<PathBuf, CrudError> {
    if id.is_empty() {
        return Err(CrudError::NotFound(NotFoundError {
            id: id.to_string(),
            workspace: workspace.to_path_buf(),
            message: "id must not be empty".to_string(),
        }));
    }
    let matches = find_matches(workspace, id)?;
    match matches.len() {
        0 => Err(CrudError::NotFound(NotFoundError {
            id: id.to_string(),
            workspace: workspace.to_path_buf(),
            message: format!("id '{}' not found for delete", id),
        })),
        1 => {
            let path = matches.into_iter().next().unwrap().path;
            std::fs::remove_file(&path).map_err(|e| {
                CrudError::Io(IoError {
                    path: path.clone(),
                    kind: e.kind(),
                    message: e.to_string(),
                })
            })?;
            Ok(path)
        }
        _ => Err(CrudError::DuplicateId(DuplicateIdError {
            id: id.to_string(),
            paths: matches.into_iter().map(|m| m.path).collect(),
            message: format!("duplicate id '{}' cannot delete", id),
        })),
    }
}
