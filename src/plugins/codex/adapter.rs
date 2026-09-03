use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct CodexAdapter;

impl WorkflowAdapter for CodexAdapter {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("codex").translate(s)
    }
}
