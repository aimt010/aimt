//! Hosted read-only engine — portable, generic, std-only.
//!
//! Architecture:
//! ```text
//! AIMT Hosted
//!     ↓
//! Single .aimt file
//!     ↓
//! Portable read-only engine (this module)
//!     ↓
//! Any AI / Agent / Tool
//!     ↓
//! Read only
//! ```
//!
//! Rules:
//! - Read-only by architecture: no `insert`, `update`, `remove`, `persist`, `open_mut`.
//! - Generic: no dependency on Claude/Codex/Cursor/Antigravity/OpenCode.
//! - Single artifact: the `.aimt` package is the portable artifact; no second file.
//! - Reuses `Store` and existing validation; does not duplicate parsing/entity logic.
//! - Std-only, no new runtime dependencies.
//! - No AI-specific translation: does not use `*Adapter` as the engine.

use std::path::{Path, PathBuf};

use crate::core::data::store::{InvalidWorkspaceError, IoError, Store, StoreError, StoreSource};
use crate::core::index::{IndexedEntity, IntegrityError};
use crate::core::model::{AimtEntity, Relation};
use crate::core::storage::workspace;
use crate::core::storage::workspace::WorkspaceError;
use crate::core::syntax::Level;

// ---------------------------------------------------------------------------
// HostedEngine — owns a Store, exposes only safe read operations
// ---------------------------------------------------------------------------

/// Portable read-only hosted engine.
///
/// Wraps `Store` and exposes only:
/// `discover`, `open`, `search`, `read/get`, `children_of`,
/// `follow_file`, `relations_from`, `validate`, `close`.
///
/// Mutation (`insert`/`update`/`remove`/`persist`/`open_mut`) is
/// impossible by architecture — these methods do not exist on this type
/// and the inner `Store` is private with no `&mut Store` accessor.
pub struct HostedEngine {
    store: Store,
}

impl HostedEngine {
    /// Open a workspace directory or a single-file `.aimt` package.
    ///
    /// Delegates to `Store::open` which already handles both forms and
    /// validates `MAGIC`/`VERSION`/entry paths. No duplicate parsing.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let store = Store::open(path)?;
        Ok(Self { store })
    }

    /// Discover AIMT sources for a directory.
    ///
    /// For hosted usage the single `.aimt` file is the artifact, but
    /// `discover` is still useful during development or when the hosted
    /// file is accompanied by a workspace. Delegates to
    /// `crate::core::storage::workspace::discover` (read-only, no I/O
    /// beyond directory listing) — reused, not duplicated.
    pub fn discover(path: &Path) -> Result<Vec<PathBuf>, StoreError> {
        workspace::discover(path).map_err(|e| match e {
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
        })
    }

    // ---- metadata (read-only) ----

    pub fn source(&self) -> StoreSource {
        self.store.source()
    }

    pub fn source_path(&self) -> &Path {
        self.store.source_path()
    }

    pub fn is_package(&self) -> bool {
        self.store.is_package()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn ids(&self) -> Vec<String> {
        self.store.ids()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.store.contains(id)
    }

    // ---- read / get ----

    /// Read a single entity by id — generic `read`/`get`.
    pub fn get(&self, id: &str) -> Option<&AimtEntity> {
        self.store.get(id)
    }

    /// Alias for `get` to satisfy `read/get` capability naming.
    pub fn read(&self, id: &str) -> Option<&AimtEntity> {
        self.get(id)
    }

    pub fn get_indexed(&self, id: &str) -> Option<&IndexedEntity> {
        self.store.get_indexed(id)
    }

    // ---- search ----

    /// Search entities by substring on id/title/level (case-insensitive).
    ///
    /// Same semantics as `workflows::read::ReadWorkflow::search` and
    /// visualizer search, but implemented directly on hosted engine so
    /// any AI can use it without installing AIMT workflows.
    pub fn search(&self, query: &str) -> Vec<&AimtEntity> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return self
                .store
                .ids()
                .into_iter()
                .filter_map(|id| self.store.get(&id))
                .collect();
        }
        self.store
            .ids()
            .into_iter()
            .filter_map(|id| self.store.get(&id))
            .filter(|e| {
                let id_contains = e
                    .id()
                    .is_some_and(|id| id.as_str().to_lowercase().contains(&q));
                let title_contains = e
                    .field("title")
                    .is_some_and(|f| f.value.to_lowercase().contains(&q));
                let level_contains = e.level.as_str().to_lowercase().contains(&q);
                id_contains || title_contains || level_contains
            })
            .collect()
    }

    /// Search via level exact match.
    pub fn find_by_level(&self, level: Level) -> Vec<&AimtEntity> {
        self.store.find_by_level(level)
    }

    // ---- hierarchy / relations / file ----

    /// `children_of` — entities whose `parent` field equals `parent_id`.
    pub fn children_of(&self, parent_id: &str) -> Vec<&AimtEntity> {
        self.store.children_of(parent_id)
    }

    /// `follow_file` — for a `@frame` (or any entity with `file` field),
    /// resolve the referenced `@file` entity.
    pub fn follow_file(&self, entity: &AimtEntity) -> Option<&AimtEntity> {
        let file_id = entity.field("file")?.value.clone();
        if file_id.is_empty() {
            return None;
        }
        self.store.get(&file_id)
    }

    /// `follow_parent` helper — resolve the parent entity of a given entity.
    pub fn follow_parent(&self, entity: &AimtEntity) -> Option<&AimtEntity> {
        let parent_id = entity.field("parent")?.value.clone();
        if parent_id.is_empty() {
            return None;
        }
        self.store.get(&parent_id)
    }

    /// `relations_from` — relations where `from == id`.
    pub fn relations_from(&self, id: &str) -> Vec<&Relation> {
        self.store.relations_from(id)
    }

    /// `relations_to` — relations where `to == id` (complementary to `relations_from`).
    pub fn relations_to(&self, id: &str) -> Vec<&Relation> {
        self.store.relations_to(id)
    }

    // ---- validate / close ----

    /// Validate the entire store (referential integrity).
    pub fn validate(&self) -> Result<(), Vec<IntegrityError>> {
        self.store.validate()
    }

    /// Close the engine — releases the in-memory representation.
    ///
    /// No persistence occurs; this is read-only. After `close`, drop the engine.
    pub fn close(self) {}
}
