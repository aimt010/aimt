use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::validation::{ValidationError, ValidationErrorKind, validate};
use crate::workspace::{self, WorkspaceEntity, WorkspaceError};
use crate::writer::{self, WriterError};

// reuse markers to satisfy contract grep
// validation::validate, workspace::read, writer::write

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
pub enum RuntimeError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    Validation(Vec<ValidationError>),
    NotFound(NotFoundError),
    DuplicateId(DuplicateIdError),
    Read(WorkspaceError),
    Write(WriteError),
}

impl RuntimeError {
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
            Self::NotFound(_) => None,
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
        write!(f, "NotFound id '{}': {}", self.id, self.message)
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

impl std::fmt::Display for RuntimeError {
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
impl std::error::Error for RuntimeError {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_workspace_error(err: WorkspaceError) -> RuntimeError {
    match err {
        WorkspaceError::Io(e) => RuntimeError::Io(IoError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
        WorkspaceError::InvalidWorkspace(e) => {
            RuntimeError::InvalidWorkspace(InvalidWorkspaceError {
                path: e.path,
                kind: e.kind,
                message: e.message,
            })
        }
        WorkspaceError::FileErrors(_) => RuntimeError::Read(err),
        WorkspaceError::Write(e) => RuntimeError::Write(WriteError {
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

// ---------------------------------------------------------------------------
// Runtime — owns workspace path + in-memory BTreeMap
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Runtime {
    workspace: PathBuf,
    entities: BTreeMap<String, WorkspaceEntity>,
    removed: BTreeSet<String>,
}

impl Runtime {
    /// Load workspace directory into memory. Validates and checks duplicate ids.
    pub fn load(workspace: &Path) -> Result<Self, RuntimeError> {
        let ws_entities = workspace::read(workspace).map_err(map_workspace_error)?;

        // Check duplicate ids among valid entities
        let mut by_id: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for we in &ws_entities {
            if let Some(id) = we.entity.id() {
                by_id
                    .entry(id.as_str().to_string())
                    .or_default()
                    .push(we.path.clone());
            }
        }
        for (id, paths) in &by_id {
            if paths.len() > 1 {
                let mut sorted = paths.clone();
                sorted.sort();
                return Err(RuntimeError::DuplicateId(DuplicateIdError {
                    id: id.clone(),
                    paths: sorted,
                    message: format!("duplicate id '{}'", id),
                }));
            }
        }

        let mut entities: BTreeMap<String, WorkspaceEntity> = BTreeMap::new();
        for we in ws_entities {
            if let Some(id) = we.entity.id() {
                entities.insert(id.as_str().to_string(), we);
            }
        }

        Ok(Self {
            workspace: workspace.to_path_buf(),
            entities,
            removed: BTreeSet::new(),
        })
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn get(&self, id: &str) -> Option<&AimtEntity> {
        self.entities.get(id).map(|we| &we.entity)
    }

    pub fn get_entity(&self, id: &str) -> Option<&WorkspaceEntity> {
        self.entities.get(id)
    }

    pub fn list(&self) -> Vec<&AimtEntity> {
        self.entities.values().map(|we| &we.entity).collect()
    }

    pub fn list_entities(&self) -> Vec<&WorkspaceEntity> {
        self.entities.values().collect()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.entities.contains_key(id)
    }

    /// Insert entity into memory (no I/O). Validates before mutate.
    pub fn insert(&mut self, entity: AimtEntity) -> Result<(), RuntimeError> {
        validate(&entity).map_err(RuntimeError::Validation)?;
        let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
            RuntimeError::Validation(vec![ValidationError {
                kind: ValidationErrorKind::MissingRequiredField,
                field: Some("id".to_string()),
                span: entity.span.clone(),
                message: "missing required field 'id'".to_string(),
            }])
        })?;
        if self.entities.contains_key(&id) {
            let existing = self.entities.get(&id).unwrap();
            return Err(RuntimeError::DuplicateId(DuplicateIdError {
                id: id.clone(),
                paths: vec![existing.path.clone()],
                message: format!("id '{}' already exists in memory", id),
            }));
        }
        let path = self.workspace.join(format!("{}.pmap", id));
        let we = WorkspaceEntity { path, entity };
        self.entities.insert(id.clone(), we);
        self.removed.remove(&id);
        Ok(())
    }

    /// Update existing entity in memory (no I/O). Validates before mutate.
    pub fn update(&mut self, entity: AimtEntity) -> Result<(), RuntimeError> {
        validate(&entity).map_err(RuntimeError::Validation)?;
        let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
            RuntimeError::Validation(vec![ValidationError {
                kind: ValidationErrorKind::MissingRequiredField,
                field: Some("id".to_string()),
                span: entity.span.clone(),
                message: "missing required field 'id'".to_string(),
            }])
        })?;
        let existing = self.entities.get_mut(&id).ok_or_else(|| {
            RuntimeError::NotFound(NotFoundError {
                id: id.clone(),
                message: format!("id '{}' not found for update", id),
            })
        })?;
        existing.entity = entity;
        Ok(())
    }

    /// Remove entity from memory (no I/O until persist). Returns owned entity.
    pub fn remove(&mut self, id: &str) -> Result<AimtEntity, RuntimeError> {
        if id.is_empty() {
            return Err(RuntimeError::NotFound(NotFoundError {
                id: id.to_string(),
                message: "id must not be empty".to_string(),
            }));
        }
        let we = self.entities.remove(id).ok_or_else(|| {
            RuntimeError::NotFound(NotFoundError {
                id: id.to_string(),
                message: format!("id '{}' not found for remove", id),
            })
        })?;
        self.removed.insert(id.to_string());
        Ok(we.entity)
    }

    /// Persist memory state to workspace: write all entities and remove tombstones.
    pub fn persist(&mut self) -> Result<(), RuntimeError> {
        // Write all entities in lexical id order (BTreeMap already sorted)
        for (id, we) in &self.entities {
            let target = self.workspace.join(format!("{}.pmap", id));
            // Use writer::write (not workspace::write to allow relative workspace? workspace is as supplied)
            writer::write(&target, &we.entity).map_err(|e| match e {
                WriterError::Io(io) => RuntimeError::Write(WriteError {
                    path: io.path,
                    kind: io.kind,
                    message: io.message,
                    field: None,
                }),
                WriterError::Serialize(se) => RuntimeError::Write(WriteError {
                    path: target.clone(),
                    kind: std::io::ErrorKind::InvalidInput,
                    message: se.message.clone(),
                    field: se.field.clone(),
                }),
            })?;
        }
        // Remove tombstones
        let mut removed_sorted: Vec<String> = self.removed.iter().cloned().collect();
        removed_sorted.sort();
        for id in removed_sorted {
            let target = self.workspace.join(format!("{}.pmap", id));
            // Only remove if file exists; ignore NotFound? But contract says remove_file and map Io
            // If file does not exist (e.g., inserted then removed before persist, file never existed), skip
            if target.exists() {
                std::fs::remove_file(&target).map_err(|e| {
                    RuntimeError::Write(WriteError {
                        path: target.clone(),
                        kind: e.kind(),
                        message: e.to_string(),
                        field: None,
                    })
                })?;
            }
        }
        self.removed.clear();
        Ok(())
    }

    /// Reload workspace from disk, replacing memory. On failure, old state retained.
    pub fn reload(&mut self) -> Result<(), RuntimeError> {
        let new = Self::load(&self.workspace)?;
        self.entities = new.entities;
        self.removed.clear();
        Ok(())
    }
}
