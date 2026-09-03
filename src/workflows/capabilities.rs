use crate::core::data::store::Store;
use crate::core::model::AimtEntity;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Capability {
    Discover,
    Open,
    Search,
    Read,
    FollowParent,
    FollowFile,
    FollowRelation,
    Validate,
    Close,
    Update,
}

#[derive(Debug)]
pub enum AgentDecision {
    ShouldUpdate { reason: String },
    NeedsSource { entity_id: String },
    WhichEntity { query: String },
}

// Update is deterministic; ShouldUpdate is agentic — two types, not one flag.
impl Capability {
    pub fn core_api(&self) -> &'static str {
        match self {
            Self::Discover => "crate::core::storage::workspace::discover",
            Self::Open => "crate::core::data::store::Store::open",
            Self::Search => "crate::core::data::store::Store::find_by_level",
            Self::Read => "crate::core::data::store::Store::get",
            Self::FollowParent => "crate::core::data::store::Store::children_of",
            Self::FollowFile => "crate::core::model::AimtEntity::field(\"file\")",
            Self::FollowRelation => "crate::core::data::store::Store::relations_from",
            Self::Validate => "crate::core::data::store::Store::validate",
            Self::Close => "crate::core::data::store::Store::validate",
            Self::Update => "crate::core::data::store::Store::open_mut + validate + persist",
        }
    }
}

/// Thin wrapper that delegates to the real Core API `Store::get`.
///
/// No string registry — just a direct call to `store.get(id).cloned()`.
/// Exists to prove `Capability::Read` is backed by `crate::core` types,
/// not a duplicated string list.
pub fn execute_read(store: &Store, id: &str) -> Option<AimtEntity> {
    store.get(id).cloned()
}
