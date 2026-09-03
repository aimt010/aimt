use aimt::workflows::context::WorkflowContext;
use std::path::Path;
#[test]
fn dirty_and_update_intent_are_distinct() {
    let mut ctx = WorkflowContext::new();
    assert!(!ctx.is_dirty());
    assert!(!ctx.has_update_intent());
    ctx.mark_dirty(); // state changed
    assert!(ctx.is_dirty());
    assert!(!ctx.has_update_intent()); // still no permission
    ctx.grant_update_intent();
    assert!(ctx.has_update_intent());
    assert!(ctx.is_dirty()); // dirty stays true
}
#[test]
fn context_holds_path_and_selection_and_error() {
    let mut ctx = WorkflowContext::new();
    ctx.set_path(Path::new("project.aimt"));
    ctx.select("node_auth");
    assert_eq!(ctx.selected(), &["node_auth"]);
    assert!(ctx.aimt_path().is_some());
}
