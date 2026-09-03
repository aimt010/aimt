use aimt::workflows::engine::WorkflowEngine; // not yet existent — will fail
#[test]
fn workflow_is_agentic_not_a_checklist() {
    // Behavioral assertion: engine must expose a way for agent to ask "what next?"
    // and for engine to branch, not just `Workflow { steps: Vec<Step> }`
    let engine = WorkflowEngine::new();
    // This test will fail until engine exists, but it documents the required behavior:
    // engine can be asked for next suggestion given a context, and can loop
    // discover -> open -> search -> read -> follow -> decide -> source/answer -> update -> validate
    assert!(engine.describe_behavior().contains("follow hierarchy"));
}
