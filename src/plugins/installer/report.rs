use std::path::PathBuf;

#[derive(Debug)]
pub struct IoErrorDetails {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

#[derive(Debug)]
pub enum InstallError {
    UnknownTool(String),
    Io(IoErrorDetails),
    InvalidProject(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(t) => write!(f, "unknown tool '{}'", t),
            Self::Io(e) => write!(f, "io error at {}: {}", e.path.display(), e.message),
            Self::InvalidProject(m) => write!(f, "invalid project: {}", m),
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Debug, Default)]
pub struct InstallReport {
    pub tool: String,
    pub created: Vec<PathBuf>,
    pub updated: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}
