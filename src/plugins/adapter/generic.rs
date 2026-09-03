use crate::workflows::capabilities::{AgentDecision, Capability};
use crate::workflows::engine::Suggestion;

use super::traits::{HostAction, WorkflowAdapter};

pub struct GenericAdapter {
    id: &'static str,
}

impl GenericAdapter {
    pub fn new(id: &'static str) -> Self {
        Self { id }
    }
}

impl WorkflowAdapter for GenericAdapter {
    fn id(&self) -> &'static str {
        self.id
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        match s {
            Suggestion::Execute(cap) => match cap {
                Capability::Discover => HostAction::ToolCall("Store::open: discover .aimt".into()),
                Capability::Open => HostAction::ToolCall("Store::open".into()),
                Capability::Search => HostAction::ToolCall("Store::find_by_level".into()),
                Capability::Read => HostAction::ToolCall("Store::get".into()),
                Capability::FollowParent => HostAction::ToolCall("Store::children_of".into()),
                Capability::FollowFile => HostAction::ToolCall("AimtEntity::field(file)".into()),
                Capability::FollowRelation => HostAction::ToolCall("Store::relations_from".into()),
                Capability::Validate => HostAction::ToolCall("Store::validate".into()),
                Capability::Close => HostAction::ToolCall("Store::validate + close".into()),
                Capability::Update => {
                    HostAction::ToolCall("Store::open_mut + validate + persist".into())
                }
            },
            Suggestion::AskAgent(decision) => match decision {
                AgentDecision::ShouldUpdate { reason } => {
                    HostAction::ShowMessage(format!("ShouldUpdate: {}", reason))
                }
                AgentDecision::NeedsSource { entity_id } => {
                    HostAction::ShowMessage(format!("NeedsSource: {}", entity_id))
                }
                AgentDecision::WhichEntity { query } => {
                    HostAction::ShowMessage(format!("WhichEntity: {}", query))
                }
            },
        }
    }
}
