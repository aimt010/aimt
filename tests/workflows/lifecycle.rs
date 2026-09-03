use aimt::workflows::context::WorkflowContext;
use aimt::workflows::definition::WorkflowKind;
use aimt::workflows::lifecycle::{WorkflowState, transition};
#[test]
fn invalid_transition_is_error_not_silent_read() {
    let ctx = WorkflowContext::new();
    let err = transition(&WorkflowState::Idle, &WorkflowKind::Validate, &ctx).unwrap_err();
    assert_eq!(err.to_string(), "invalid transition: Idle -> Validate");
}
#[test]
fn valid_lifecycle() {
    let ctx = WorkflowContext::new();
    assert_eq!(
        transition(&WorkflowState::Idle, &WorkflowKind::Discover, &ctx).unwrap(),
        WorkflowState::Discovered
    );
}
