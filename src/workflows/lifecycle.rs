use crate::workflows::context::WorkflowContext;
use crate::workflows::definition::WorkflowKind;
use crate::workflows::definition::WorkflowKind as K;

use self::WorkflowState as S;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WorkflowState {
    Idle,
    Discovered,
    Opened,
    Searched,
    Read,
    Dirty,
    Validated,
    Closed,
    Failed,
}
#[derive(Debug)]
pub struct WorkflowError(pub String);
impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for WorkflowError {}

pub fn transition(
    state: &WorkflowState,
    kind: &WorkflowKind,
    ctx: &WorkflowContext,
) -> Result<WorkflowState, WorkflowError> {
    match (state, kind) {
        (S::Idle, K::Discover) => Ok(S::Discovered),
        (S::Discovered, K::Open) => Ok(S::Opened),
        (S::Opened, K::Search) => Ok(S::Searched),
        (S::Searched, K::Search) => Ok(S::Searched),
        (S::Searched, K::Read) => Ok(S::Read),
        (S::Searched, K::FollowParent) => Ok(S::Read),
        (S::Searched, K::FollowFile) => Ok(S::Read),
        (S::Searched, K::FollowRelation) => Ok(S::Read),
        (S::Read, K::Search) => Ok(S::Searched),
        (S::Read, K::Read) => Ok(S::Read),
        (S::Read, K::FollowParent) => Ok(S::Read),
        (S::Read, K::FollowFile) => Ok(S::Read),
        (S::Read, K::FollowRelation) => Ok(S::Read),
        (S::Read, K::Update) => {
            if ctx.has_update_intent() {
                Ok(S::Dirty)
            } else {
                Err(WorkflowError(format!(
                    "invalid transition: {:?} -> {:?} without update_intent",
                    state, kind
                )))
            }
        }
        (S::Dirty, K::Validate) => Ok(S::Validated),
        (S::Read, K::Validate) => Ok(S::Validated),
        (S::Searched, K::Validate) => Ok(S::Validated),
        (S::Opened, K::Validate) => Ok(S::Validated),
        (S::Validated, K::Close) => Ok(S::Closed),
        (S::Read, K::Close) => Ok(S::Closed),
        (S::Searched, K::Close) => Ok(S::Closed),
        (S::Opened, K::Close) => Ok(S::Closed),
        (S::Discovered, K::Close) => Ok(S::Closed),
        (S::Dirty, K::Close) => Err(WorkflowError(format!(
            "invalid transition: {:?} -> {:?} (must validate before close when dirty)",
            state, kind
        ))),
        _ => Err(WorkflowError(format!(
            "invalid transition: {:?} -> {:?}",
            state, kind
        ))),
    }
}
