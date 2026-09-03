use crate::syntax::Span;

/// Entity ID — value of `id` field, lightweight, unresolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for EntityId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for EntityId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Reference to another entity by ID — `parent`, `file`, `from`, `to`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceId(pub String);

impl ReferenceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A generic AIMT field — preserves every syntactically valid field,
/// known or unknown. Empty values are preserved as `value == ""`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub span: Span,
}

impl Field {
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}
