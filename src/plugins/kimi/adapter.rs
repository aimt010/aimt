use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct KimiAdapter;

impl WorkflowAdapter for KimiAdapter {
    fn id(&self) -> &'static str {
        "kimi"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("kimi").translate(s)
    }
}
