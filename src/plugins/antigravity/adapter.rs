use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct AntigravityAdapter;

impl WorkflowAdapter for AntigravityAdapter {
    fn id(&self) -> &'static str {
        "antigravity"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("antigravity").translate(s)
    }
}
