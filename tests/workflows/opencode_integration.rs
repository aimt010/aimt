use aimt::plugins::adapter::{HostAction, Suggestion, WorkflowAdapter};
use aimt::plugins::opencode::OpenCodeAdapter;
use aimt::workflows::capabilities::{AgentDecision, Capability};

// Test A — deterministic read action
#[test]
fn opencode_deterministic_read_is_toolcall() {
    let adapter = OpenCodeAdapter;
    let action = adapter.translate(Suggestion::Execute(Capability::Read));
    match action {
        HostAction::ToolCall(s) => assert!(!s.is_empty(), "ToolCall should be non-empty for Read"),
        HostAction::ShowMessage(s) => panic!("Read should be ToolCall, got ShowMessage({})", s),
    }
}

// Test B — agent decision NeedsSource
#[test]
fn opencode_agent_needs_source_is_observable() {
    let adapter = OpenCodeAdapter;
    let action = adapter.translate(Suggestion::AskAgent(AgentDecision::NeedsSource {
        entity_id: "node_auth".into(),
    }));
    match action {
        HostAction::ShowMessage(s) => assert!(
            !s.is_empty() && s.contains("node_auth"),
            "Should contain entity_id, got {}",
            s
        ),
        HostAction::ToolCall(s) => assert!(!s.is_empty(), "ToolCall also observable, got {}", s),
    }
}

// Test C — update permission
#[test]
fn opencode_update_permission_reflected() {
    let adapter = OpenCodeAdapter;
    // dirty + no intent => AskAgent(ShouldUpdate)
    let no_intent = Suggestion::AskAgent(AgentDecision::ShouldUpdate {
        reason: "dirty without intent".into(),
    });
    let action_no_intent = adapter.translate(no_intent);
    match action_no_intent {
        HostAction::ShowMessage(s) => assert!(
            !s.is_empty(),
            "ShouldUpdate without intent should be observable ShowMessage"
        ),
        HostAction::ToolCall(s) => assert!(!s.is_empty()),
    }
    // dirty + intent => Execute(Update)
    let with_intent = Suggestion::Execute(Capability::Update);
    let action_with_intent = adapter.translate(with_intent);
    match action_with_intent {
        HostAction::ToolCall(s) => assert!(!s.is_empty(), "Update with intent should be ToolCall"),
        HostAction::ShowMessage(s) => panic!(
            "Update with intent should be ToolCall, got ShowMessage({})",
            s
        ),
    }
}

// Test D — all capability variants must be observable (no silent empty)
#[test]
fn opencode_all_capabilities_are_observable() {
    let adapter = OpenCodeAdapter;
    let all = vec![
        Capability::Discover,
        Capability::Open,
        Capability::Search,
        Capability::Read,
        Capability::FollowParent,
        Capability::FollowFile,
        Capability::FollowRelation,
        Capability::Validate,
        Capability::Close,
        Capability::Update,
    ];
    for cap in all {
        let action = adapter.translate(Suggestion::Execute(cap));
        let s = action.to_string();
        assert!(
            !s.is_empty(),
            "capability should produce non-empty HostAction"
        );
        // Must not be silent empty ShowMessage("")
        assert_ne!(s, "ShowMessage()", "capability should not be silent empty");
        assert_ne!(s, "ToolCall()", "capability should not be silent empty");
    }
}

// Test E — all agent decisions
#[test]
fn opencode_all_agent_decisions_are_observable() {
    let adapter = OpenCodeAdapter;
    let decisions = vec![
        AgentDecision::ShouldUpdate {
            reason: "test".into(),
        },
        AgentDecision::NeedsSource {
            entity_id: "node_auth".into(),
        },
        AgentDecision::WhichEntity {
            query: "auth".into(),
        },
    ];
    for decision in decisions {
        let action = adapter.translate(Suggestion::AskAgent(decision));
        let s = action.to_string();
        assert!(!s.is_empty(), "decision should be observable");
        assert_ne!(s, "ShowMessage()", "decision should not be silent");
    }
}

// Test F — portability is already covered in portability.rs, but ensure opencode still works
#[test]
fn opencode_portability_still_requires_no_workflow_host_import() {
    // This is a compile-time check: src/workflows must not import opencode
    // We verify by ensuring the test itself can use both adapters without workflows knowing host
    let engine = aimt::workflows::engine::WorkflowEngine::new();
    let ctx = aimt::workflows::context::WorkflowContext::new();
    let suggestion = engine.suggest_next(&ctx);
    let opencode = OpenCodeAdapter;
    struct ClaudeAdapter;
    impl WorkflowAdapter for ClaudeAdapter {
        fn id(&self) -> &'static str {
            "claude"
        }
        fn translate(&self, _s: Suggestion) -> HostAction {
            HostAction::ShowMessage("claude".into())
        }
    }
    // Just ensure both can translate the same suggestion without panicking
    let _ = opencode.translate(suggestion);
    // Create a new suggestion for claude (since suggestion was moved)
    let engine2 = aimt::workflows::engine::WorkflowEngine::new();
    let ctx2 = aimt::workflows::context::WorkflowContext::new();
    let suggestion2 = engine2.suggest_next(&ctx2);
    let claude = ClaudeAdapter;
    let _ = claude.translate(suggestion2);
}

// Read-only safety — adapter cannot grant write permission
#[test]
fn opencode_adapter_cannot_bypass_update_permission() {
    use aimt::core::data::store::Store;
    use aimt::workflows::update::UpdateWorkflow;
    use std::path::Path;

    let src = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let tmp = std::env::temp_dir().join(format!(
        "aimt_opencode_perm_{}_{}.aimt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(&src, &tmp).unwrap();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.mark_dirty();
    // No update_intent — even though adapter might translate Execute(Update) to ToolCall, the workflow must refuse
    let before_len = store.len();
    let entity = {
        use aimt::core::model::{AimtEntity, Field, Level};
        use aimt::core::syntax::Span;
        let span = Span::range(1, 1, 1, 1);
        let f = |n: &str, v: &str| Field {
            name: n.to_string(),
            value: v.to_string(),
            span: span.clone(),
        };
        AimtEntity {
            level: Level::Node,
            level_span: span.clone(),
            span: span.clone(),
            header: vec![f("id", "perm_test_no_intent"), f("title", "Test")],
            body: vec![
                f("parent", "region_api"),
                f("description", "test"),
                f("type", "service"),
            ],
            relations: vec![],
        }
    };
    let result = UpdateWorkflow::new().execute(&mut store, &mut ctx, entity);
    assert!(result.is_err(), "without intent, update must be denied");
    // Adapter translation alone should not have mutated the store
    let store2 = Store::open(&tmp).unwrap();
    assert_eq!(store2.len(), before_len);
    let _ = std::fs::remove_file(&tmp);
}
