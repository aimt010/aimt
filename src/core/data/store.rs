use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::core::security::keys::verify_write_credential;
use crate::index::{IndexedEntity, IntegrityError, integrity};
use crate::model::{AimtEntity, Relation};
use crate::parser;
use crate::syntax::Level;
use crate::validation::{self, ValidationError, ValidationErrorKind};
use crate::workspace::{self, WorkspaceError};
use crate::writer::{self, WriterError};

// ---------------------------------------------------------------------------
// Store source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSource {
    Workspace,
    Package,
}

// ---------------------------------------------------------------------------
// Error types
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
pub struct DuplicateIdError {
    pub id: String,
    pub paths: Vec<PathBuf>,
    pub message: String,
}

#[derive(Debug)]
pub struct NotFoundError {
    pub id: String,
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
pub enum StoreError {
    Io(IoError),
    InvalidWorkspace(InvalidWorkspaceError),
    InvalidPackage(InvalidPackageError),
    InvalidEntry(InvalidEntryError),
    DuplicateEntry(DuplicateEntryError),
    DuplicateId(DuplicateIdError),
    Read(WorkspaceError),
    Validation(Vec<ValidationError>),
    NotFound(NotFoundError),
    Write(WriteError),
}

impl StoreError {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Io(_) => "Io",
            Self::InvalidWorkspace(_) => "InvalidWorkspace",
            Self::InvalidPackage(_) => "InvalidPackage",
            Self::InvalidEntry(_) => "InvalidEntry",
            Self::DuplicateEntry(_) => "DuplicateEntry",
            Self::DuplicateId(_) => "DuplicateId",
            Self::Read(_) => "Read",
            Self::Validation(_) => "Validation",
            Self::NotFound(_) => "NotFound",
            Self::Write(_) => "Write",
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io(e) => Some(&e.path),
            Self::InvalidWorkspace(e) => Some(&e.path),
            Self::InvalidPackage(e) => Some(&e.path),
            Self::InvalidEntry(e) => Some(&e.path),
            Self::DuplicateEntry(e) => Some(&e.path),
            Self::DuplicateId(e) => e.paths.first().map(|p| p.as_path()),
            Self::Read(e) => e.path(),
            Self::Validation(_) => None,
            Self::NotFound(_) => None,
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

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NotFound id '{}': {}", self.id, self.message)
    }
}
impl std::error::Error for NotFoundError {}

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

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidWorkspace(e) => write!(f, "{}", e),
            Self::InvalidPackage(e) => write!(f, "{}", e),
            Self::InvalidEntry(e) => write!(f, "{}", e),
            Self::DuplicateEntry(e) => write!(f, "{}", e),
            Self::DuplicateId(e) => write!(f, "{}", e),
            Self::Read(e) => write!(f, "Read {}", e),
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
            Self::Write(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for StoreError {}

// ---------------------------------------------------------------------------
// Helpers — package format (duplicated from package.rs to avoid modifying frozen)
// ---------------------------------------------------------------------------

const MAGIC: [u8; 4] = [0x41, 0x49, 0x4D, 0x54];
const VERSION: u8 = 0x01;

fn is_valid_entry_path(path_str: &str) -> bool {
    if path_str.is_empty()
        || path_str.starts_with('.')
        || path_str.contains('/')
        || path_str.contains('\\')
    {
        return false;
    }
    let p = Path::new(path_str);
    if p.is_absolute() {
        return false;
    }
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => return false,
            std::path::Component::CurDir => return false,
            _ => {}
        }
    }
    if !path_str.ends_with(".pmap") {
        return false;
    }
    if p.components().count() != 1 {
        return false;
    }
    if path_str.len() > 4096 {
        return false;
    }
    true
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

fn write_u16_le(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

// ---------------------------------------------------------------------------
// Store — unified workspace/package source
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Store {
    source: StoreSource,
    source_path: PathBuf,
    entities: BTreeMap<String, IndexedEntity>,
    removed: BTreeSet<String>,
    authenticated: bool,
    explicit_auth: bool,
}

impl Store {
    /// Open path as workspace directory or package file.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let md = std::fs::metadata(path).map_err(|e| {
            StoreError::Io(IoError {
                path: path.to_path_buf(),
                kind: e.kind(),
                message: e.to_string(),
            })
        })?;
        if md.is_dir() {
            Self::open_workspace(path)
        } else if md.is_file() {
            Self::open_package(path)
        } else {
            Err(StoreError::Io(IoError {
                path: path.to_path_buf(),
                kind: std::io::ErrorKind::Other,
                message: "path is not a regular file or directory".to_string(),
            }))
        }
    }

    fn open_workspace(path: &Path) -> Result<Self, StoreError> {
        let ws_entities = workspace::read(path).map_err(|e| match e {
            WorkspaceError::Io(inner) => StoreError::Io(IoError {
                path: inner.path,
                kind: inner.kind,
                message: inner.message,
            }),
            WorkspaceError::InvalidWorkspace(inner) => {
                StoreError::InvalidWorkspace(InvalidWorkspaceError {
                    path: inner.path,
                    kind: inner.kind,
                    message: inner.message,
                })
            }
            WorkspaceError::FileErrors(_) => StoreError::Read(e),
            WorkspaceError::Write(inner) => StoreError::Io(IoError {
                path: inner.path,
                kind: std::io::ErrorKind::Other,
                message: inner.error.to_string(),
            }),
        })?;

        let mut entities: BTreeMap<String, IndexedEntity> = BTreeMap::new();
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
                return Err(StoreError::DuplicateId(DuplicateIdError {
                    id: id.clone(),
                    paths: sorted,
                    message: format!("duplicate id '{}'", id),
                }));
            }
        }
        for we in ws_entities {
            if let Some(id) = we.entity.id() {
                entities.insert(
                    id.as_str().to_string(),
                    IndexedEntity {
                        path: we.path,
                        entity: we.entity,
                    },
                );
            }
        }
        Ok(Self {
            source: StoreSource::Workspace,
            source_path: path.to_path_buf(),
            entities,
            removed: BTreeSet::new(),
            authenticated: false,
            explicit_auth: false,
        })
    }

    fn open_package(path: &Path) -> Result<Self, StoreError> {
        let data = std::fs::read(path).map_err(|e| {
            StoreError::Io(IoError {
                path: path.to_path_buf(),
                kind: e.kind(),
                message: e.to_string(),
            })
        })?;

        if data.len() < 9 {
            return Err(StoreError::InvalidPackage(InvalidPackageError {
                path: path.to_path_buf(),
                message: "package too short".to_string(),
            }));
        }
        if data[0..4] != MAGIC {
            return Err(StoreError::InvalidPackage(InvalidPackageError {
                path: path.to_path_buf(),
                message: "invalid magic".to_string(),
            }));
        }
        if data[4] != VERSION {
            return Err(StoreError::InvalidPackage(InvalidPackageError {
                path: path.to_path_buf(),
                message: format!("unsupported version {}", data[4]),
            }));
        }
        let mut offset = 5;
        let count = read_u32_le(&data, &mut offset).map_err(|m| {
            StoreError::InvalidPackage(InvalidPackageError {
                path: path.to_path_buf(),
                message: m,
            })
        })? as usize;

        let mut raw_entries: Vec<(String, Vec<u8>)> = Vec::new();
        let mut seen_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        for _ in 0..count {
            let path_len = read_u16_le(&data, &mut offset).map_err(|m| {
                StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: m,
                })
            })? as usize;
            if path_len == 0 || path_len > 4096 {
                return Err(StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: format!("invalid path length {}", path_len),
                }));
            }
            if offset + path_len > data.len() {
                return Err(StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: "truncated path".to_string(),
                }));
            }
            let path_bytes = &data[offset..offset + path_len];
            offset += path_len;
            let path_str = String::from_utf8(path_bytes.to_vec()).map_err(|_| {
                StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: "path not utf8".to_string(),
                })
            })?;
            if !is_valid_entry_path(&path_str) {
                return Err(StoreError::InvalidEntry(InvalidEntryError {
                    path: PathBuf::from(&path_str),
                    message: format!("invalid entry path '{}'", path_str),
                }));
            }
            if !seen_paths.insert(path_str.clone()) {
                return Err(StoreError::DuplicateEntry(DuplicateEntryError {
                    path: PathBuf::from(&path_str),
                    message: format!("duplicate entry '{}'", path_str),
                }));
            }
            let content_len = read_u32_le(&data, &mut offset).map_err(|m| {
                StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: m,
                })
            })? as usize;
            if offset + content_len > data.len() {
                return Err(StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: "truncated content".to_string(),
                }));
            }
            let content = data[offset..offset + content_len].to_vec();
            offset += content_len;
            raw_entries.push((path_str, content));
        }

        if offset != data.len() {
            return Err(StoreError::InvalidPackage(InvalidPackageError {
                path: path.to_path_buf(),
                message: "extra trailing bytes".to_string(),
            }));
        }

        // Parse each entry bytes into AimtEntity, validate shape
        let mut entities: BTreeMap<String, IndexedEntity> = BTreeMap::new();
        let mut by_id: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();

        for (path_str, bytes) in raw_entries {
            // Use parser + model + validation via reader logic without filesystem
            let parsed = parser::parse_bytes(&bytes).map_err(|e| {
                StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: format!(
                        "entry '{}' parse error {}: {}",
                        path_str,
                        e.kind.as_str(),
                        e.message
                    ),
                })
            })?;
            let entity = AimtEntity::from_parsed(parsed);
            if let Err(errs) = validation::validate(&entity) {
                let msg = errs
                    .iter()
                    .map(|er| format!("{}:{:?}", er.kind.as_str(), er.field))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: format!("entry '{}' validation failed: {}", path_str, msg),
                }));
            }
            let id = entity.id().ok_or_else(|| {
                StoreError::InvalidPackage(InvalidPackageError {
                    path: path.to_path_buf(),
                    message: format!("entry '{}' missing id", path_str),
                })
            })?;
            let id_str = id.as_str().to_string();
            // Duplicate id check across package entries
            if by_id.contains_key(&id_str) {
                // Will be caught after loop, but also check now
            }
            by_id
                .entry(id_str.clone())
                .or_default()
                .push(PathBuf::from(&path_str));
            entities.insert(
                id_str,
                IndexedEntity {
                    path: PathBuf::from(path_str),
                    entity,
                },
            );
        }

        // Duplicate id across entries (same as workspace duplicate)
        for (id, paths) in &by_id {
            if paths.len() > 1 {
                let mut sorted = paths.clone();
                sorted.sort();
                return Err(StoreError::DuplicateId(DuplicateIdError {
                    id: id.clone(),
                    paths: sorted,
                    message: format!("duplicate id '{}'", id),
                }));
            }
        }

        Ok(Self {
            source: StoreSource::Package,
            source_path: path.to_path_buf(),
            entities,
            removed: BTreeSet::new(),
            authenticated: false,
            explicit_auth: false,
        })
    }

    pub fn source(&self) -> StoreSource {
        self.source
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn is_package(&self) -> bool {
        self.source == StoreSource::Package
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

    pub fn ids(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<&AimtEntity> {
        self.entities.get(id).map(|ie| &ie.entity)
    }

    pub fn get_indexed(&self, id: &str) -> Option<&IndexedEntity> {
        self.entities.get(id)
    }

    pub fn find_by_level(&self, level: Level) -> Vec<&AimtEntity> {
        self.entities
            .values()
            .filter(|ie| ie.entity.level == level)
            .map(|ie| &ie.entity)
            .collect()
    }

    pub fn children_of(&self, parent_id: &str) -> Vec<&AimtEntity> {
        self.entities
            .values()
            .filter(|ie| {
                ie.entity
                    .field("parent")
                    .map(|f| f.value == parent_id)
                    .unwrap_or(false)
            })
            .map(|ie| &ie.entity)
            .collect()
    }

    pub fn relations_from(&self, id: &str) -> Vec<&Relation> {
        let mut out = Vec::new();
        for ie in self.entities.values() {
            for rel in &ie.entity.relations {
                if rel.get_value("from") == Some(id) {
                    out.push(rel);
                }
            }
        }
        out
    }

    pub fn relations_to(&self, id: &str) -> Vec<&Relation> {
        let mut out = Vec::new();
        for ie in self.entities.values() {
            for rel in &ie.entity.relations {
                if rel.get_value("to") == Some(id) {
                    out.push(rel);
                }
            }
        }
        out
    }

    pub fn validate(&self) -> Result<(), Vec<IntegrityError>> {
        integrity::validate_entities(&self.entities)
    }

    // -----------------------------------------------------------------------
    // Auth helpers — owner write credential
    // -----------------------------------------------------------------------

    /// Reads @aimt owner_public_key field if present.
    pub fn owner_public_key(&self) -> Option<String> {
        // Find @aimt entity (level Aimt) and read owner_public_key field
        for ie in self.entities.values() {
            if ie.entity.level == Level::Aimt
                && let Some(f) = ie.entity.field("owner_public_key")
                && !f.value.is_empty()
            {
                return Some(f.value.clone());
            }
        }
        None
    }

    /// Whether this store requires auth (has owner_public_key).
    pub fn needs_auth(&self) -> bool {
        self.owner_public_key().is_some()
    }

    /// Verify private hex against stored owner_public_key. For legacy (no key) returns true.
    pub fn verify_write_key(&self, priv_hex: &str) -> bool {
        match self.owner_public_key() {
            Some(pub_key) => verify_write_credential(&pub_key, priv_hex),
            None => true,
        }
    }

    /// Whether this store is authenticated for writes.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Authenticate this store in-place with a private key hex. Returns verification result.
    pub fn authenticate(&mut self, priv_hex: &str) -> bool {
        let ok = self.verify_write_key(priv_hex);
        // Legacy (no owner key) always verifies true, but we still mark authenticated if verify succeeds
        self.authenticated = ok;
        self.explicit_auth = true;
        ok
    }

    /// Open for mutation with write credential verification.
    pub fn open_mut_authenticated(path: &Path, priv_hex: &str) -> Result<Self, StoreError> {
        let mut s = Self::open(path)?;
        s.authenticated = s.verify_write_key(priv_hex);
        s.explicit_auth = true;
        Ok(s)
    }

    fn ensure_write_authorized(&self) -> Result<(), StoreError> {
        if self.needs_auth() {
            if self.authenticated {
                return Ok(());
            }
            // If explicit auth was attempted and failed, do not fallback to keyring (old key should fail after rotate)
            if self.explicit_auth {
                return Err(StoreError::Write(WriteError {
                    path: self.source_path.clone(),
                    kind: std::io::ErrorKind::PermissionDenied,
                    message: "write unauthorized: missing or invalid write credential".to_string(),
                    field: None,
                }));
            }
            // Check OS keyring / fallback file for stored owner credential (login)
            if let Some(pub_key) = self.owner_public_key()
                && let Some(private) = crate::core::security::auth::load_credential(&pub_key)
                && self.verify_write_key(&private)
            {
                return Ok(());
            }
            return Err(StoreError::Write(WriteError {
                path: self.source_path.clone(),
                kind: std::io::ErrorKind::PermissionDenied,
                message: "write unauthorized: missing or invalid write credential".to_string(),
                field: None,
            }));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Mutable — .aimt as single source of truth (OPEN/READ/WRITE/CLOSE)
    // -----------------------------------------------------------------------

    /// Open for mutation (same as `open`, but semantically marks intent to WRITE).
    pub fn open_mut(path: &Path) -> Result<Self, StoreError> {
        Self::open(path)
    }

    /// Insert entity into memory (no I/O). Validates before mutate.
    pub fn insert(&mut self, entity: AimtEntity) -> Result<(), StoreError> {
        self.ensure_write_authorized()?;
        validation::validate(&entity).map_err(StoreError::Validation)?;
        let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
            StoreError::Validation(vec![ValidationError {
                kind: ValidationErrorKind::MissingRequiredField,
                field: Some("id".to_string()),
                span: entity.span.clone(),
                message: "missing required field 'id'".to_string(),
            }])
        })?;
        if self.entities.contains_key(&id) {
            let existing = self.entities.get(&id).unwrap();
            return Err(StoreError::DuplicateId(DuplicateIdError {
                id: id.clone(),
                paths: vec![existing.path.clone()],
                message: format!("id '{}' already exists in memory", id),
            }));
        }
        // Preserve existing entry name if re-inserting after remove? Use id.pmap
        let entry_path = PathBuf::from(format!("{}.pmap", id));
        // Validate entry path
        let entry_str = entry_path.to_string_lossy().to_string();
        if !is_valid_entry_path(&entry_str) {
            return Err(StoreError::InvalidEntry(InvalidEntryError {
                path: entry_path.clone(),
                message: format!("invalid entry path '{}'", entry_str),
            }));
        }
        let ie = IndexedEntity {
            path: entry_path,
            entity,
        };
        self.entities.insert(id.clone(), ie);
        self.removed.remove(&id);
        Ok(())
    }

    /// Update existing entity in memory (no I/O). Validates before mutate.
    pub fn update(&mut self, entity: AimtEntity) -> Result<(), StoreError> {
        self.ensure_write_authorized()?;
        validation::validate(&entity).map_err(StoreError::Validation)?;
        let id = entity.id().map(|e| e.as_str().to_string()).ok_or_else(|| {
            StoreError::Validation(vec![ValidationError {
                kind: ValidationErrorKind::MissingRequiredField,
                field: Some("id".to_string()),
                span: entity.span.clone(),
                message: "missing required field 'id'".to_string(),
            }])
        })?;
        let existing = self.entities.get_mut(&id).ok_or_else(|| {
            StoreError::NotFound(NotFoundError {
                id: id.clone(),
                message: format!("id '{}' not found for update", id),
            })
        })?;
        existing.entity = entity;
        Ok(())
    }

    /// Remove entity from memory (no I/O until persist). Returns owned entity.
    pub fn remove(&mut self, id: &str) -> Result<AimtEntity, StoreError> {
        self.ensure_write_authorized()?;
        if id.is_empty() {
            return Err(StoreError::NotFound(NotFoundError {
                id: id.to_string(),
                message: "id must not be empty".to_string(),
            }));
        }
        let ie = self.entities.remove(id).ok_or_else(|| {
            StoreError::NotFound(NotFoundError {
                id: id.to_string(),
                message: format!("id '{}' not found for remove", id),
            })
        })?;
        self.removed.insert(id.to_string());
        Ok(ie.entity)
    }

    /// Persist memory state to the original source (workspace or package).
    /// For workspace: writes `id.pmap` per entity and removes tombstones.
    /// For package: rewrites the whole `.aimt` file atomically.
    pub fn persist(&mut self) -> Result<(), StoreError> {
        self.ensure_write_authorized()?;
        match self.source {
            StoreSource::Workspace => self.persist_workspace(),
            StoreSource::Package => self.persist_package(),
        }
    }

    fn persist_workspace(&mut self) -> Result<(), StoreError> {
        for (id, ie) in &self.entities {
            let target = self.source_path.join(format!("{}.pmap", id));
            writer::write(&target, &ie.entity).map_err(|e| match e {
                WriterError::Io(io) => StoreError::Write(WriteError {
                    path: io.path,
                    kind: io.kind,
                    message: io.message,
                    field: None,
                }),
                WriterError::Serialize(se) => StoreError::Write(WriteError {
                    path: target.clone(),
                    kind: std::io::ErrorKind::InvalidInput,
                    message: se.message.clone(),
                    field: se.field.clone(),
                }),
            })?;
        }
        let mut removed_sorted: Vec<String> = self.removed.iter().cloned().collect();
        removed_sorted.sort();
        for id in removed_sorted {
            let target = self.source_path.join(format!("{}.pmap", id));
            if target.exists() {
                std::fs::remove_file(&target).map_err(|e| {
                    StoreError::Write(WriteError {
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

    fn persist_package(&mut self) -> Result<(), StoreError> {
        // Collect (entry_name, bytes) sorted lexical by entry_name
        let mut collected: Vec<(String, Vec<u8>)> = Vec::new();
        for (id, ie) in &self.entities {
            // Use stored path file_name if valid, else id.pmap
            let entry_name = ie
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.pmap", id));
            // Ensure valid and unique
            if !is_valid_entry_path(&entry_name) {
                return Err(StoreError::InvalidEntry(InvalidEntryError {
                    path: PathBuf::from(&entry_name),
                    message: format!("invalid entry path '{}'", entry_name),
                }));
            }
            let bytes = writer::serialize_to_bytes(&ie.entity).map_err(|e| match e {
                WriterError::Io(io) => StoreError::Write(WriteError {
                    path: self.source_path.clone(),
                    kind: io.kind,
                    message: io.message,
                    field: None,
                }),
                WriterError::Serialize(se) => StoreError::Write(WriteError {
                    path: PathBuf::from(&entry_name),
                    kind: std::io::ErrorKind::InvalidInput,
                    message: se.message.clone(),
                    field: se.field.clone(),
                }),
            })?;
            collected.push((entry_name, bytes));
        }
        // Check duplicate entry paths (e.g., two ids map to same file name)
        collected.sort_by(|a, b| a.0.cmp(&b.0));
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for (name, _) in &collected {
            if !seen.insert(name.clone()) {
                return Err(StoreError::DuplicateEntry(DuplicateEntryError {
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
        // Atomic write: write to temp then rename
        let tmp = self.source_path.with_extension("aimt.tmp");
        std::fs::write(&tmp, &out).map_err(|e| {
            StoreError::Write(WriteError {
                path: self.source_path.clone(),
                kind: e.kind(),
                message: e.to_string(),
                field: None,
            })
        })?;
        std::fs::rename(&tmp, &self.source_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            StoreError::Write(WriteError {
                path: self.source_path.clone(),
                kind: e.kind(),
                message: e.to_string(),
                field: None,
            })
        })?;
        self.removed.clear();
        Ok(())
    }

    /// Reload from disk, replacing memory. On failure, old state retained.
    pub fn reload(&mut self) -> Result<(), StoreError> {
        let new = Self::open(&self.source_path)?;
        self.entities = new.entities;
        self.removed.clear();
        // Keep original source/source_path (should be same, but use new's)
        self.source = new.source;
        self.authenticated = new.authenticated;
        self.explicit_auth = new.explicit_auth;
        Ok(())
    }
}
