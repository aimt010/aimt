use super::value::{Field, ReferenceId};
use crate::syntax::Span;

/// A relation object — explicit relation data, not a level.
/// Stored inside `relations` field, contains its own fields and spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub fields: Vec<Field>,
    pub span: Span,
}

impl Relation {
    /// Helper to get field by name (e.g., "type", "from", "to")
    pub fn get(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.get(name).map(|f| f.value.as_str())
    }
    /// Reference helpers — remain IDs, not resolved
    pub fn from_ref(&self) -> Option<ReferenceId> {
        self.get_value("from").map(|s| ReferenceId(s.to_string()))
    }
    pub fn to_ref(&self) -> Option<ReferenceId> {
        self.get_value("to").map(|s| ReferenceId(s.to_string()))
    }
}
