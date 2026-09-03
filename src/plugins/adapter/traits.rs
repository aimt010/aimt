use crate::workflows::engine::Suggestion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostAction {
    ToolCall(String),
    ShowMessage(String),
}

impl std::fmt::Display for HostAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolCall(s) => write!(f, "ToolCall({})", s),
            Self::ShowMessage(s) => write!(f, "ShowMessage({})", s),
        }
    }
}

pub trait WorkflowAdapter {
    fn id(&self) -> &'static str;
    fn translate(&self, s: Suggestion) -> HostAction;
}
