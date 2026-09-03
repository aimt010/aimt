use crate::plugins::adapter::{GenericAdapter, HostAction, Suggestion, WorkflowAdapter};

pub struct CursorAdapter;

impl WorkflowAdapter for CursorAdapter {
    fn id(&self) -> &'static str {
        "cursor"
    }

    fn translate(&self, s: Suggestion) -> HostAction {
        GenericAdapter::new("cursor").translate(s)
    }
}
