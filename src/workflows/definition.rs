#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WorkflowKind {
    Discover,
    Open,
    Search,
    Read,
    FollowParent,
    FollowFile,
    FollowRelation,
    Validate,
    Close,
    Init,
    Update,
}
#[derive(Debug)]
pub struct Workflow {
    pub kind: WorkflowKind,
}
impl Workflow {
    pub fn for_kind(kind: WorkflowKind) -> Self {
        Self { kind }
    }
}
// No Step, no Decision yet — YAGNI. Add only when a workflow needs branching.
