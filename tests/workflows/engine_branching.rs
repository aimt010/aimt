use aimt::workflows::context::WorkflowContext;
use aimt::workflows::engine::{Suggestion, WorkflowEngine};
#[test]
fn engine_asks_agent_when_update_needed() {
    let engine = WorkflowEngine::new();
    let ctx = WorkflowContext::new(); // no dirty, no intent
    assert!(matches!(engine.suggest_next(&ctx), Suggestion::AskAgent(_)));
}
#[test]
fn engine_executes_read_when_search_found_something() {
    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.select("node_auth");
    assert!(matches!(engine.suggest_next(&ctx), Suggestion::Execute(_)));
}
