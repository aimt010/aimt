use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::workspace;
use crate::workspace::WorkspaceError;
use crate::writer::WriterError;

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
pub struct InvalidPackageError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct InvalidEntryError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct DuplicateEntryError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub enum PackageError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    InvalidPackage(InvalidPackageError),
    InvalidEntry(InvalidEntryError),
    DuplicateEntry(DuplicateEntryError),
    Read(WorkspaceError),
    ExtractionFailed(IoError),
}

impl PackageError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::InvalidPackage(_) => "InvalidPackage",
            Self::InvalidEntry(_) => "InvalidEntry",
            Self::DuplicateEntry(_) => "DuplicateEntry",
            Self::Read(_) => "Read",
            Self::ExtractionFailed(_) => "ExtractionFailed",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::InvalidPackage(e) => Some(&e.path),
            Self::InvalidEntry(e) => Some(&e.path),
            Self::DuplicateEntry(e) => Some(&e.path),
            Self::Read(e) => e.path(),
            Self::ExtractionFailed(e) => Some(&e.path),
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

impl std::fmt::Display for InvalidPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InvalidPackage at {}: {}",
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for InvalidPackageError {}

impl std::fmt::Display for InvalidEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InvalidEntry at {}: {}",
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for InvalidEntryError {}

impl std::fmt::Display for DuplicateEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DuplicateEntry at {}: {}",
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for DuplicateEntryError {}

impl std::fmt::Display for PackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::InvalidPackage(e) => write!(f, "{}", e),
            Self::InvalidEntry(e) => write!(f, "{}", e),
            Self::DuplicateEntry(e) => write!(f, "{}", e),
            Self::Read(e) => write!(f, "Read {}", e),
            Self::ExtractionFailed(e) => write!(f, "ExtractionFailed {}", e),
        }
    }
}
impl std::error::Error for PackageError {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_valid_entry_path(path_str: &str) -> bool {
    if path_str.is_empty() {
        return false;
    }
    if path_str.starts_with('.') {
        return false;
    }
    if path_str.contains('/') || path_str.contains('\\') {
        return false;
    }
    let p = Path::new(path_str);
    if p.is_absolute() {
        return false;
    }
    // No .. or . components
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => return false,
            std::path::Component::CurDir => return false,
            _ => {}
        }
    }
    // Must end with .pmap case-sensitive
    if !path_str.ends_with(".pmap") {
        return false;
    }
    // Must have exactly one component (no slash already checked)
    if p.components().count() != 1 {
        return false;
    }
    // Length check
    if path_str.len() > 4096 {
        return false;
    }
    true
}

fn map_workspace_error(err: WorkspaceError) -> PackageError {
    match err {
        WorkspaceError::Io(e) => PackageError::Io(IoError {
            path: e.path,
            kind: e.kind,
            message: e.message,
        }),
        WorkspaceError::InvalidWorkspace(e) => {
            PackageError::InvalidWorkspace(InvalidWorkspaceError {
                path: e.path,
                kind: e.kind,
                message: e.message,
            })
        }
        WorkspaceError::FileErrors(_) => PackageError::Read(err),
        WorkspaceError::Write(e) => PackageError::Io(IoError {
            path: e.path,
            kind: match &e.error {
                WriterError::Io(io) => io.kind,
                WriterError::Serialize(_) => std::io::ErrorKind::InvalidInput,
            },
            message: e.error.to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Package format helpers
// ---------------------------------------------------------------------------

const MAGIC: [u8; 4] = [0x41, 0x49, 0x4D, 0x54]; // "AIMT"
const VERSION: u8 = 0x01;

fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn read_u16_le(data: &[u8], offset: &mut usize) -> Result<u16, String> {
    if *offset + 2 > data.len() {
        return Err("truncated u16".to_string());
    }
    let v = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Ok(v)
}
fn read_u32_le(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    if *offset + 4 > data.len() {
        return Err("truncated u32".to_string());
    }
    let v = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    Ok(v)
}

// ---------------------------------------------------------------------------
// create — workspace -> package file
// ---------------------------------------------------------------------------

/// Create a package file from a workspace directory.
/// Workspace must be an existing directory with valid .pmap files.
/// Package parent must exist; package is overwritten if exists as file.
pub fn create(workspace: &Path, package: &Path) -> Result<(), PackageError> {
    // Validate package is not a directory if exists
    if let Ok(md) = std::fs::metadata(package)
        && md.is_dir()
    {
        return Err(PackageError::Io(IoError {
            path: package.to_path_buf(),
            kind: std::io::ErrorKind::IsADirectory,
            message: "package path is a directory".to_string(),
        }));
    }
    // Validate package parent exists if package has a parent
    if let Some(parent) = package.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(PackageError::Io(IoError {
                path: package.to_path_buf(),
                kind: std::io::ErrorKind::NotFound,
                message: format!("package parent does not exist: {}", parent.display()),
            }));
        }
        // Parent must be dir if exists
        if !parent.as_os_str().is_empty()
            && let Ok(md) = std::fs::metadata(parent)
            && !md.is_dir()
        {
            return Err(PackageError::Io(IoError {
                path: package.to_path_buf(),
                kind: std::io::ErrorKind::NotFound,
                message: "package parent is not a directory".to_string(),
            }));
        }
    }

    // Validate workspace via workspace::read (checks dir, filter, parse, validation)
    // Use workspace::read to ensure invalid .pmap fails
    workspace::read(workspace).map_err(|e| match e {
        WorkspaceError::Io(inner) => PackageError::Io(IoError {
            path: inner.path,
            kind: inner.kind,
            message: inner.message,
        }),
        WorkspaceError::InvalidWorkspace(inner) => {
            PackageError::InvalidWorkspace(InvalidWorkspaceError {
                path: inner.path,
                kind: inner.kind,
                message: inner.message,
            })
        }
        WorkspaceError::FileErrors(_) => PackageError::Read(e),
        WorkspaceError::Write(inner) => PackageError::Io(IoError {
            path: inner.path,
            kind: std::io::ErrorKind::Other,
            message: inner.error.to_string(),
        }),
    })?;

    // Discover sorted entries (already validated, but need paths for bytes)
    let entries = workspace::discover(workspace).map_err(map_workspace_error)?;

    // Collect (file_name, bytes) sorted lexical (discover already sorted)
    let mut collected: Vec<(String, Vec<u8>)> = Vec::new();
    for path in entries {
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        if !is_valid_entry_path(&file_name) {
            return Err(PackageError::InvalidEntry(InvalidEntryError {
                path: path.clone(),
                message: format!("invalid entry path '{}'", file_name),
            }));
        }
        let bytes = std::fs::read(&path).map_err(|e| {
            PackageError::Io(IoError {
                path: path.clone(),
                kind: e.kind(),
                message: e.to_string(),
            })
        })?;
        collected.push((file_name, bytes));
    }

    // Already sorted via discover, but ensure lexical by file name
    collected.sort_by(|a, b| a.0.cmp(&b.0));

    // Check duplicate entry paths (should not happen via filesystem, but defensive)
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (name, _) in &collected {
        if !seen.insert(name.clone()) {
            return Err(PackageError::DuplicateEntry(DuplicateEntryError {
                path: PathBuf::from(name),
                message: format!("duplicate entry '{}'", name),
            }));
        }
    }

    // Build package bytes deterministically
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    write_u32_le(&mut out, collected.len() as u32);
    for (name, bytes) in &collected {
        let name_bytes = name.as_bytes();
        write_u16_le(&mut out, name_bytes.len() as u16);
        out.extend_from_slice(name_bytes);
        write_u32_le(&mut out, bytes.len() as u32);
        out.extend_from_slice(bytes);
    }

    // Write package file
    match std::fs::write(package, &out) {
        Ok(_) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(package);
            Err(PackageError::Io(IoError {
                path: package.to_path_buf(),
                kind: e.kind(),
                message: e.to_string(),
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// extract — package file -> destination directory
// ---------------------------------------------------------------------------

/// Extract a package file into an existing destination directory.
pub fn extract(package: &Path, destination: &Path) -> Result<(), PackageError> {
    // Validate package is existing regular file
    match std::fs::metadata(package) {
        Ok(md) => {
            if md.is_dir() {
                return Err(PackageError::Io(IoError {
                    path: package.to_path_buf(),
                    kind: std::io::ErrorKind::IsADirectory,
                    message: "package path is a directory".to_string(),
                }));
            }
            if !md.is_file() {
                return Err(PackageError::Io(IoError {
                    path: package.to_path_buf(),
                    kind: std::io::ErrorKind::Other,
                    message: "package path is not a regular file".to_string(),
                }));
            }
        }
        Err(e) => {
            return Err(PackageError::Io(IoError {
                path: package.to_path_buf(),
                kind: e.kind(),
                message: e.to_string(),
            }));
        }
    }

    // Validate destination is existing directory
    match std::fs::metadata(destination) {
        Ok(md) => {
            if !md.is_dir() {
                return Err(PackageError::InvalidWorkspace(InvalidWorkspaceError {
                    path: destination.to_path_buf(),
                    kind: std::io::ErrorKind::Other,
                    message: "destination is not a directory".to_string(),
                }));
            }
        }
        Err(e) => {
            return Err(PackageError::Io(IoError {
                path: destination.to_path_buf(),
                kind: e.kind(),
                message: e.to_string(),
            }));
        }
    }

    let data = std::fs::read(package).map_err(|e| {
        PackageError::Io(IoError {
            path: package.to_path_buf(),
            kind: e.kind(),
            message: e.to_string(),
        })
    })?;

    // Parse header
    if data.len() < 9 {
        return Err(PackageError::InvalidPackage(InvalidPackageError {
            path: package.to_path_buf(),
            message: "package too short".to_string(),
        }));
    }
    if data[0..4] != MAGIC {
        return Err(PackageError::InvalidPackage(InvalidPackageError {
            path: package.to_path_buf(),
            message: "invalid magic".to_string(),
        }));
    }
    if data[4] != VERSION {
        return Err(PackageError::InvalidPackage(InvalidPackageError {
            path: package.to_path_buf(),
            message: format!("unsupported version {}", data[4]),
        }));
    }
    let mut offset = 5;
    let count = match read_u32_le(&data, &mut offset) {
        Ok(c) => c,
        Err(m) => {
            return Err(PackageError::InvalidPackage(InvalidPackageError {
                path: package.to_path_buf(),
                message: m,
            }));
        }
    };

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    for _ in 0..count {
        let path_len = match read_u16_le(&data, &mut offset) {
            Ok(v) => v as usize,
            Err(m) => {
                return Err(PackageError::InvalidPackage(InvalidPackageError {
                    path: package.to_path_buf(),
                    message: m,
                }));
            }
        };
        if path_len == 0 || path_len > 4096 {
            return Err(PackageError::InvalidPackage(InvalidPackageError {
                path: package.to_path_buf(),
                message: format!("invalid path length {}", path_len),
            }));
        }
        if offset + path_len > data.len() {
            return Err(PackageError::InvalidPackage(InvalidPackageError {
                path: package.to_path_buf(),
                message: "truncated path".to_string(),
            }));
        }
        let path_bytes = &data[offset..offset + path_len];
        offset += path_len;
        let path_str = match std::str::from_utf8(path_bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                return Err(PackageError::InvalidPackage(InvalidPackageError {
                    path: package.to_path_buf(),
                    message: "path not utf8".to_string(),
                }));
            }
        };
        if !is_valid_entry_path(&path_str) {
            return Err(PackageError::InvalidEntry(InvalidEntryError {
                path: PathBuf::from(&path_str),
                message: format!("invalid entry path '{}'", path_str),
            }));
        }
        if !seen.insert(path_str.clone()) {
            return Err(PackageError::DuplicateEntry(DuplicateEntryError {
                path: PathBuf::from(&path_str),
                message: format!("duplicate entry '{}'", path_str),
            }));
        }

        let content_len = match read_u32_le(&data, &mut offset) {
            Ok(v) => v as usize,
            Err(m) => {
                return Err(PackageError::InvalidPackage(InvalidPackageError {
                    path: package.to_path_buf(),
                    message: m,
                }));
            }
        };
        if offset + content_len > data.len() {
            return Err(PackageError::InvalidPackage(InvalidPackageError {
                path: package.to_path_buf(),
                message: "truncated content".to_string(),
            }));
        }
        let content = data[offset..offset + content_len].to_vec();
        offset += content_len;

        entries.push((path_str, content));
    }

    if offset != data.len() {
        return Err(PackageError::InvalidPackage(InvalidPackageError {
            path: package.to_path_buf(),
            message: "extra trailing bytes".to_string(),
        }));
    }

    // Write entries to destination with path safety
    for (path_str, bytes) in entries {
        // Double-check safety: already validated, but ensure no escape
        if path_str.contains('/')
            || path_str.contains('\\')
            || path_str.contains("..")
            || path_str.starts_with('.')
            || path_str.is_empty()
        {
            return Err(PackageError::InvalidEntry(InvalidEntryError {
                path: PathBuf::from(&path_str),
                message: "invalid entry path".to_string(),
            }));
        }
        let dest_path = destination.join(&path_str);
        // Ensure dest_path is inside destination (since path is single file name, this holds)
        if !dest_path.starts_with(destination) {
            return Err(PackageError::InvalidEntry(InvalidEntryError {
                path: PathBuf::from(&path_str),
                message: "path escapes destination".to_string(),
            }));
        }
        if let Err(e) = std::fs::write(&dest_path, &bytes) {
            return Err(PackageError::ExtractionFailed(IoError {
                path: dest_path,
                kind: e.kind(),
                message: e.to_string(),
            }));
        }
    }

    Ok(())
}
