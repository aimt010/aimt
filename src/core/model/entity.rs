use super::relation::Relation;
use super::value::{EntityId, Field, ReferenceId};
use crate::syntax::{Level, Span};

/// An AIMT entity — one `.pmap` represents one top-level entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AimtEntity {
    pub level: Level,
    pub level_span: Span,
    pub span: Span,
    /// Fields under `#header`
    pub header: Vec<Field>,
    /// Fields under `#body` excluding the `relations` field itself
    pub body: Vec<Field>,
    /// Explicit relations extracted from `relations` field (zero or more)
    pub relations: Vec<Relation>,
}

impl AimtEntity {
    /// Retrieve field by name searching header then body.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.header
            .iter()
            .find(|f| f.name == name)
            .or_else(|| self.body.iter().find(|f| f.name == name))
    }

    /// ID helper — looks up `id` field value as `EntityId`.
    pub fn id(&self) -> Option<EntityId> {
        self.field("id").map(|f| EntityId(f.value.clone()))
    }

    /// Reference helpers — remain IDs, not resolved.
    pub fn parent_ref(&self) -> Option<ReferenceId> {
        self.field("parent").map(|f| ReferenceId(f.value.clone()))
    }

    pub fn file_ref(&self) -> Option<ReferenceId> {
        self.field("file").map(|f| ReferenceId(f.value.clone()))
    }

    /// All fields (header + body) for iteration — unknown fields are included.
    pub fn all_fields(&self) -> impl Iterator<Item = &Field> {
        self.header.iter().chain(self.body.iter())
    }

    /// Check if entity contains a field with given name.
    pub fn has_field(&self, name: &str) -> bool {
        self.field(name).is_some()
    }
}

// Structural invariants (purely structural, not semantic validation)
impl AimtEntity {
    /// Structural invariant: one level per entity (always true by construction).
    pub fn invariant_one_level_per_entity(&self) -> bool {
        true
    }
    /// Structural invariant: level is one of seven (always true by construction via Level enum).
    pub fn invariant_level_is_closed_set(&self) -> bool {
        Level::all().contains(&self.level.as_str())
    }
    /// Structural invariant: relations are explicit objects, not levels.
    pub fn invariant_relations_not_levels(&self) -> bool {
        true
    }
}
