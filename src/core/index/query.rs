use super::Index;
use crate::model::{AimtEntity, Relation};
use crate::syntax::Level;

impl Index {
    pub fn get(&self, id: &str) -> Option<&AimtEntity> {
        self.entities.get(id).map(|ie| &ie.entity)
    }

    pub fn get_indexed(&self, id: &str) -> Option<&super::IndexedEntity> {
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
}
