use aimt::plugins::adapter::{HostAction, Suggestion, WorkflowAdapter};
use aimt::plugins::claude::ClaudeAdapter;
use aimt::workflows::capabilities::{AgentDecision, Capability};

#[test]
fn claude_translates_all_capabilities() {
    let adapter = ClaudeAdapter;
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
        assert!(!s.is_empty(), "Claude capability should be observable");
        assert_ne!(s, "ShowMessage()", "Claude capability should not be silent");
        assert_ne!(s, "ToolCall()", "Claude capability should not be silent");
        assert!(
            matches!(action, HostAction::ToolCall(_)),
            "Claude Execute should be ToolCall, got {}",
            s
        );
    }
}

#[test]
fn claude_translates_all_agent_decisions() {
    let adapter = ClaudeAdapter;
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
        assert!(!s.is_empty(), "Claude decision should be observable");
        assert_ne!(s, "ShowMessage()", "Claude decision should not be silent");
        assert!(
            matches!(action, HostAction::ShowMessage(_)),
            "Claude AskAgent should be ShowMessage, got {}",
            s
        );
    }
}

#[test]
fn claude_displays_decision_context() {
    let adapter = ClaudeAdapter;
    let action = adapter.translate(Suggestion::AskAgent(AgentDecision::NeedsSource {
        entity_id: "node_auth".into(),
    }));
    match action {
        HostAction::ShowMessage(s) => assert!(
            s.contains("node_auth"),
            "Claude NeedsSource should contain entity_id, got {}",
            s
        ),
        _ => panic!("expected ShowMessage for NeedsSource"),
    }
    let action2 = adapter.translate(Suggestion::AskAgent(AgentDecision::ShouldUpdate {
        reason: "dirty without intent".into(),
    }));
    match action2 {
        HostAction::ShowMessage(s) => assert!(
            s.contains("ShouldUpdate"),
            "should contain ShouldUpdate, got {}",
            s
        ),
        _ => panic!("expected ShowMessage"),
    }
}

#[test]
fn claude_portability_same_suggestion() {
    let claude = ClaudeAdapter;
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    assert_ne!(claude.id(), opencode.id());
    assert_eq!(claude.id(), "claude");
    assert_eq!(opencode.id(), "opencode");
    // Same suggestion works for both
    let s = Suggestion::Execute(Capability::Read);
    let a1 = opencode.translate(s);
    // Need to recreate suggestion since it was moved
    let s2 = Suggestion::Execute(Capability::Read);
    let a2 = claude.translate(s2);
    assert!(matches!(a1, HostAction::ToolCall(_)));
    assert!(matches!(a2, HostAction::ToolCall(_)));
    assert_ne!(a1.to_string(), "ShowMessage()");
    assert_ne!(a2.to_string(), "ShowMessage()");
}

#[test]
fn claude_cannot_bypass_update_permission() {
    // Adapter is translator only, not permission layer
    // Even if adapter translates Execute(Update) to ToolCall, the workflow engine and UpdateWorkflow still enforce permission
    use std::path::Path;
    let src = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let tmp = std::env::temp_dir().join(format!(
        "aimt_claude_perm_{}_{}.aimt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(&src, &tmp).unwrap();
    let mut store = aimt::core::data::store::Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.mark_dirty();
    // No intent
    let before = store.len();
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
            header: vec![f("id", "claude_perm_test"), f("title", "Test")],
            body: vec![
                f("parent", "region_api"),
                f("description", "test"),
                f("type", "service"),
            ],
            relations: vec![],
        }
    };
    let result =
        aimt::workflows::update::UpdateWorkflow::new().execute(&mut store, &mut ctx, entity);
    assert!(
        result.is_err(),
        "without intent, update must be denied even though adapter would translate Execute(Update) to ToolCall"
    );
    assert_eq!(result.unwrap_err().kind_str(), "Write");
    let store2 = aimt::core::data::store::Store::open(&tmp).unwrap();
    assert_eq!(store2.len(), before);
    let _ = std::fs::remove_file(&tmp);
}
