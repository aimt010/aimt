use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::AimtEntity;

#[derive(Debug, Clone)]
pub struct IndexedEntity {
    pub path: PathBuf,
    pub entity: AimtEntity,
}

#[derive(Debug)]
pub struct Index {
    pub(crate) workspace: PathBuf,
    pub(crate) entities: BTreeMap<String, IndexedEntity>,
}

impl Index {
    pub fn workspace(&self) -> &Path {
        &self.workspace
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
}
