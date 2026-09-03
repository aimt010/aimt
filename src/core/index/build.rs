use std::collections::BTreeMap;
use std::path::Path;

use super::error::{DuplicateIdError, IndexError, InvalidWorkspaceError, IoError};
use super::{Index, IndexedEntity};
use crate::workspace::{self, WorkspaceError};

impl Index {
    /// Build index from workspace directory. Validates via workspace::read and duplicate ids.
    pub fn build(workspace: &Path) -> Result<Self, IndexError> {
        let ws_entities = workspace::read(workspace).map_err(|e| match e {
            WorkspaceError::Io(inner) => IndexError::Io(IoError {
                path: inner.path,
                kind: inner.kind,
                message: inner.message,
            }),
            WorkspaceError::InvalidWorkspace(inner) => {
                IndexError::InvalidWorkspace(InvalidWorkspaceError {
                    path: inner.path,
                    kind: inner.kind,
                    message: inner.message,
                })
            }
            WorkspaceError::FileErrors(_) => IndexError::Read(e),
            WorkspaceError::Write(inner) => IndexError::Io(IoError {
                path: inner.path,
                kind: std::io::ErrorKind::Other,
                message: inner.error.to_string(),
            }),
        })?;

        let mut by_id: BTreeMap<String, Vec<std::path::PathBuf>> = BTreeMap::new();
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
                return Err(IndexError::DuplicateId(DuplicateIdError {
                    id: id.clone(),
                    paths: sorted,
                    message: format!("duplicate id '{}'", id),
                }));
            }
        }

        let mut entities: BTreeMap<String, IndexedEntity> = BTreeMap::new();
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
            workspace: workspace.to_path_buf(),
            entities,
        })
    }
}
