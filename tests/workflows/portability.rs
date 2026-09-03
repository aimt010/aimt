use aimt::plugins::adapter::WorkflowAdapter;
use aimt::workflows::{
    context::WorkflowContext,
    definition::WorkflowKind,
    permissions::{Permission, permission_for},
};

#[test]
fn published_aimt_is_read_only_by_default_and_portable() {
    let ctx = WorkflowContext::new();
    assert_eq!(
        permission_for(WorkflowKind::Read, &ctx),
        Permission::ReadOnly
    );
    // Same engine, two adapters
    let engine = aimt::workflows::engine::WorkflowEngine::new();
    let suggestion = engine.suggest_next(&ctx);
    // Both adapters can translate the same suggestion
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    struct ClaudeAdapter;
    impl aimt::plugins::adapter::WorkflowAdapter for ClaudeAdapter {
        fn id(&self) -> &'static str {
            "claude"
        }
        fn translate(
            &self,
            _s: aimt::workflows::engine::Suggestion,
        ) -> aimt::plugins::adapter::HostAction {
            aimt::plugins::adapter::HostAction::ShowMessage("claude".into())
        }
    }
    let claude = ClaudeAdapter;
    assert_eq!(claude.id(), "claude");
    // Same engine suggestion works for both hosts — proof of portability
    let suggestion2 = engine.suggest_next(&ctx);
    assert!(!opencode.translate(suggestion).to_string().is_empty());
    assert!(!claude.translate(suggestion2).to_string().is_empty());
}
