use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct OpenCodeAdapter;

impl WorkflowAdapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("opencode").translate(s)
    }
}
