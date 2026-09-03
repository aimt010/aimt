use std::path::{Path, PathBuf};

use crate::model::AimtEntity;
use crate::runtime::{Runtime, RuntimeError};
use crate::validation::ValidationError;
use crate::workspace::WorkspaceError;

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Unopened,
    Opened,
    Closed,
    Exited,
}

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
pub struct NotOpenedError {
    pub workspace: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug)]
pub struct AlreadyOpenedError {
    pub workspace: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct AlreadyClosedError {
    pub message: String,
}

#[derive(Debug)]
pub struct ExitedError {
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
}

#[derive(Debug)]
pub enum SessionError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    NotOpened(NotOpenedError),
    AlreadyOpened(AlreadyOpenedError),
    AlreadyClosed(AlreadyClosedError),
    Exited(ExitedError),
    Validation(Vec<ValidationError>),
    DuplicateId(DuplicateIdError),
    NotFound(NotFoundError),
    Read(WorkspaceError),
    Write(WriteError),
}

impl SessionError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::NotOpened(_) => "NotOpened",
            Self::AlreadyOpened(_) => "AlreadyOpened",
            Self::AlreadyClosed(_) => "AlreadyClosed",
            Self::Exited(_) => "Exited",
            Self::Validation(_) => "Validation",
            Self::DuplicateId(_) => "DuplicateId",
            Self::NotFound(_) => "NotFound",
            Self::Read(_) => "Read",
            Self::Write(_) => "Write",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::NotOpened(e) => e.workspace.as_deref(),
            Self::AlreadyOpened(e) => Some(&e.workspace),
            Self::AlreadyClosed(_) => None,
            Self::Exited(_) => None,
            Self::Validation(_) => None,
            Self::DuplicateId(e) => e.paths.first().map(|p| p.as_path()),
            Self::NotFound(_) => None,
            Self::Read(e) => e.path(),
            Self::Write(e) => Some(&e.path),
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::DuplicateId(e) => Some(&e.id),
            Self::NotFound(e) => Some(&e.id),
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

impl std::fmt::Display for NotOpenedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NotOpened workspace={:?}: {}",
            self.workspace, self.message
        )
    }
}
impl std::error::Error for NotOpenedError {}

impl std::fmt::Display for AlreadyOpenedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AlreadyOpened at {}: {}",
            self.workspace.display(),
            self.message
        )
    }
}
impl std::error::Error for AlreadyOpenedError {}

impl std::fmt::Display for AlreadyClosedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AlreadyClosed: {}", self.message)
    }
}
impl std::error::Error for AlreadyClosedError {}

impl std::fmt::Display for ExitedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Exited: {}", self.message)
    }
}
impl std::error::Error for ExitedError {}

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
            "Write {:?} at {}: {}",
            self.kind,
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for WriteError {}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::NotOpened(e) => write!(f, "{}", e),
            Self::AlreadyOpened(e) => write!(f, "{}", e),
            Self::AlreadyClosed(e) => write!(f, "{}", e),
            Self::Exited(e) => write!(f, "{}", e),
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
            Self::DuplicateId(e) => write!(f, "{}", e),
            Self::NotFound(e) => write!(f, "{}", e),
            Self::Read(e) => write!(f, "Read {}", e),
            Self::Write(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for SessionError {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn map_runtime_error(err: RuntimeError) -> SessionError {
    match err {
        RuntimeError::Io(e) => SessionError::Io(IoError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
        RuntimeError::InvalidWorkspace(e) => {
            SessionError::InvalidWorkspace(InvalidWorkspaceError {
                path: e.path,
                kind: e.kind,
                message: e.message,
            })
        }
        RuntimeError::Validation(v) => SessionError::Validation(v),
        RuntimeError::NotFound(e) => SessionError::NotFound(NotFoundError {
            id: e.id,
            message: e.message,
        }),
        RuntimeError::DuplicateId(e) => SessionError::DuplicateId(DuplicateIdError {
            id: e.id,
            paths: e.paths,
            message: e.message,
        }),
        RuntimeError::Read(e) => SessionError::Read(e),
        RuntimeError::Write(e) => SessionError::Write(WriteError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
    }
}

// ---------------------------------------------------------------------------
// Session — owns Runtime + state machine
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Session {
    state: SessionState,
    workspace: Option<PathBuf>,
    runtime: Option<Runtime>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: SessionState::Unopened,
            workspace: None,
            runtime: None,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn workspace(&self) -> Option<&Path> {
        self.workspace.as_deref()
    }

    pub fn is_open(&self) -> bool {
        self.state == SessionState::Opened
    }

    pub fn is_exited(&self) -> bool {
        self.state == SessionState::Exited
    }

    /// OPEN workspace — load Runtime, transition Unopened/Closed -> Opened
    pub fn open(&mut self, workspace: &Path) -> Result<(), SessionError> {
        match self.state {
            SessionState::Opened => {
                return Err(SessionError::AlreadyOpened(AlreadyOpenedError {
                    workspace: workspace.to_path_buf(),
                    message: "session already opened".to_string(),
                }));
            }
            SessionState::Exited => {
                return Err(SessionError::Exited(ExitedError {
                    message: "session already exited".to_string(),
                }));
            }
            SessionState::Unopened | SessionState::Closed => {}
        }

        let runtime = Runtime::load(workspace).map_err(map_runtime_error)?;
        self.runtime = Some(runtime);
        self.workspace = Some(workspace.to_path_buf());
        self.state = SessionState::Opened;
        Ok(())
    }

    /// READ id -> &AimtEntity from memory, no I/O
    pub fn read(&self, id: &str) -> Result<&AimtEntity, SessionError> {
        match self.state {
            SessionState::Exited => {
                return Err(SessionError::Exited(ExitedError {
                    message: "session exited".to_string(),
                }));
            }
            SessionState::Unopened | SessionState::Closed => {
                return Err(SessionError::NotOpened(NotOpenedError {
                    workspace: self.workspace.clone(),
                    message: "session not opened".to_string(),
                }));
            }
            SessionState::Opened => {}
        }
        if id.is_empty() {
            return Err(SessionError::NotFound(NotFoundError {
                id: id.to_string(),
                message: "id must not be empty".to_string(),
            }));
        }
        let rt = self.runtime.as_ref().unwrap();
        rt.get(id).ok_or_else(|| {
            SessionError::NotFound(NotFoundError {
                id: id.to_string(),
                message: format!("id '{}' not found", id),
            })
        })
    }

    /// WRITE entity — validate, insert/update in memory, persist that file
    pub fn write(&mut self, entity: AimtEntity) -> Result<(), SessionError> {
        match self.state {
            SessionState::Exited => {
                return Err(SessionError::Exited(ExitedError {
                    message: "session exited".to_string(),
                }));
            }
            SessionState::Unopened | SessionState::Closed => {
                return Err(SessionError::NotOpened(NotOpenedError {
                    workspace: self.workspace.clone(),
                    message: "session not opened".to_string(),
                }));
            }
            SessionState::Opened => {}
        }

        let id = entity
            .id()
            .map(|e| e.as_str().to_string())
            .unwrap_or_default();

        let rt = self.runtime.as_mut().unwrap();
        let is_update = rt.contains(&id);

        let op_res = if is_update {
            rt.update(entity)
        } else {
            rt.insert(entity)
        };

        if let Err(e) = op_res {
            return Err(map_runtime_error(e));
        }

        // Persist exactly that file via Runtime::persist (writes all + tombstones)
        // Using persist ensures deterministic file creation via writer::write
        let rt_mut = self.runtime.as_mut().unwrap();
        rt_mut.persist().map_err(map_runtime_error)?;

        Ok(())
    }

    /// CLOSE — release memory, transition Opened -> Closed
    pub fn close(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Opened => {
                self.runtime = None;
                self.state = SessionState::Closed;
                Ok(())
            }
            SessionState::Closed => Err(SessionError::AlreadyClosed(AlreadyClosedError {
                message: "session already closed".to_string(),
            })),
            SessionState::Unopened => Err(SessionError::NotOpened(NotOpenedError {
                workspace: self.workspace.clone(),
                message: "session not opened".to_string(),
            })),
            SessionState::Exited => Err(SessionError::Exited(ExitedError {
                message: "session exited".to_string(),
            })),
        }
    }

    /// EXIT — terminal, drop runtime, forbid further operations
    pub fn exit(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Exited => Err(SessionError::Exited(ExitedError {
                message: "session already exited".to_string(),
            })),
            SessionState::Unopened | SessionState::Opened | SessionState::Closed => {
                self.runtime = None;
                self.workspace = None;
                self.state = SessionState::Exited;
                Ok(())
            }
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
