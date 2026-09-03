use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct ClaudeAdapter;

impl WorkflowAdapter for ClaudeAdapter {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("claude").translate(s)
    }
}
