use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct KiloAdapter;

impl WorkflowAdapter for KiloAdapter {
    fn id(&self) -> &'static str {
        "kilo"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("kilo").translate(s)
    }
}
