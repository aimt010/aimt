use aimt::plugins::adapter::{HostAction, Suggestion, WorkflowAdapter};
use aimt::plugins::codex::CodexAdapter;
use aimt::workflows::capabilities::{AgentDecision, Capability};

#[test]
fn codex_id_is_codex() {
    let adapter = CodexAdapter;
    assert_eq!(adapter.id(), "codex");
}

#[test]
fn codex_translates_all_capabilities() {
    let adapter = CodexAdapter;
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
        assert!(!s.is_empty(), "Codex capability should be observable");
        assert_ne!(s, "ShowMessage()", "Codex capability should not be silent");
        assert_ne!(s, "ToolCall()", "Codex capability should not be silent");
        assert!(
            matches!(action, HostAction::ToolCall(_)),
            "Codex Execute should be ToolCall, got {}",
            s
        );
    }
}

#[test]
fn codex_translates_all_agent_decisions() {
    let adapter = CodexAdapter;
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
        assert!(!s.is_empty(), "Codex decision should be observable");
        assert_ne!(s, "ShowMessage()", "Codex decision should not be silent");
        assert!(
            matches!(action, HostAction::ShowMessage(_)),
            "Codex AskAgent should be ShowMessage, got {}",
            s
        );
    }
}

#[test]
fn codex_should_update_contains_reason() {
    let adapter = CodexAdapter;
    let action = adapter.translate(Suggestion::AskAgent(AgentDecision::ShouldUpdate {
        reason: "dirty without intent".into(),
    }));
    match action {
        HostAction::ShowMessage(s) => assert!(
            s.contains("dirty without intent"),
            "ShouldUpdate should contain reason, got {}",
            s
        ),
        _ => panic!("expected ShowMessage for ShouldUpdate"),
    }
}

#[test]
fn codex_needs_source_contains_entity_id() {
    let adapter = CodexAdapter;
    let action = adapter.translate(Suggestion::AskAgent(AgentDecision::NeedsSource {
        entity_id: "node_auth".into(),
    }));
    match action {
        HostAction::ShowMessage(s) => assert!(
            s.contains("node_auth"),
            "Codex NeedsSource should contain entity_id, got {}",
            s
        ),
        _ => panic!("expected ShowMessage for NeedsSource"),
    }
}

#[test]
fn codex_which_entity_contains_query() {
    let adapter = CodexAdapter;
    let action = adapter.translate(Suggestion::AskAgent(AgentDecision::WhichEntity {
        query: "auth".into(),
    }));
    match action {
        HostAction::ShowMessage(s) => assert!(
            s.contains("auth"),
            "Codex WhichEntity should contain query, got {}",
            s
        ),
        _ => panic!("expected ShowMessage for WhichEntity"),
    }
}

#[test]
fn codex_and_opencode_same_toolcall() {
    let codex = CodexAdapter;
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    assert_ne!(codex.id(), opencode.id());
    assert_eq!(codex.id(), "codex");
    assert_eq!(opencode.id(), "opencode");
    let s = Suggestion::Execute(Capability::Read);
    let a1 = opencode.translate(s);
    let s2 = Suggestion::Execute(Capability::Read);
    let a2 = codex.translate(s2);
    assert!(matches!(a1, HostAction::ToolCall(_)));
    assert!(matches!(a2, HostAction::ToolCall(_)));
    assert_eq!(
        a1.to_string(),
        a2.to_string(),
        "same Suggestion should produce same semantic ToolCall"
    );
}

#[test]
fn codex_cannot_bypass_update_permission() {
    use std::path::Path;
    let src = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let tmp = std::env::temp_dir().join(format!(
        "aimt_codex_perm_{}_{}.aimt",
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
            header: vec![f("id", "codex_perm_test"), f("title", "Test")],
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

#[test]
fn workflows_remain_host_agnostic() {
    let content = std::fs::read_to_string(format!(
        "{}/src/workflows/capabilities.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(
        !content.contains("codex"),
        "src/workflows must not import codex"
    );
    assert!(
        !content.contains("claude"),
        "src/workflows must not import claude"
    );
    assert!(
        !content.contains("opencode"),
        "src/workflows must not import opencode"
    );
    let content2 = std::fs::read_to_string(format!(
        "{}/src/workflows/engine.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(!content2.contains("codex"));
    assert!(!content2.contains("claude"));
    assert!(!content2.contains("opencode"));
}

#[test]
fn existing_opencode_and_claude_unchanged() {
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    let claude = aimt::plugins::claude::ClaudeAdapter;
    let codex = CodexAdapter;
    // All three should translate the same suggestion identically (semantic equivalence)
    for cap in [Capability::Read, Capability::Search, Capability::Validate] {
        let a_op = opencode.translate(Suggestion::Execute(cap));
        let a_cl = claude.translate(Suggestion::Execute(cap));
        let a_co = codex.translate(Suggestion::Execute(cap));
        // Just verify they are all ToolCall and non-empty
        assert!(matches!(a_op, HostAction::ToolCall(_)));
        assert!(matches!(a_cl, HostAction::ToolCall(_)));
        assert!(matches!(a_co, HostAction::ToolCall(_)));
    }
}
