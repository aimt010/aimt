use aimt::core::data::store::Store;
use aimt::core::model::{AimtEntity, Field, Level};
use aimt::core::syntax::Span;
use aimt::workflows::capabilities::AgentDecision;
use aimt::workflows::context::WorkflowContext;
use aimt::workflows::definition::WorkflowKind;
use aimt::workflows::engine::{Suggestion, WorkflowEngine};
use aimt::workflows::lifecycle::{WorkflowState, transition};
use aimt::workflows::permissions::{Permission, permission_for};
use aimt::workflows::read::ReadWorkflow;
use aimt::workflows::update::UpdateWorkflow;
use std::path::Path;

fn dummy_span() -> Span {
    Span::range(1, 1, 1, 1)
}
fn field(name: &str, value: &str) -> Field {
    Field {
        name: name.to_string(),
        value: value.to_string(),
        span: dummy_span(),
    }
}
fn make_node(id: &str, parent: &str) -> AimtEntity {
    AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", id), field("title", "Test Node")],
        body: vec![
            field("parent", parent),
            field("description", "test"),
            field("type", "service"),
        ],
        relations: vec![],
    }
}
fn temp_aimt_copy() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let src = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let dst = std::env::temp_dir().join(format!(
        "aimt_agent_test_{}_{}_{}.aimt",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(&src, &dst).unwrap();
    dst
}

// 1. end_to_end_read_only_agent_flow
#[test]
fn end_to_end_read_only_agent_flow() {
    let path = temp_aimt_copy();
    // Use existing workflows without new abstraction
    let read_wf = ReadWorkflow::new();
    let _engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&path);

    // Discover / Open
    let store = read_wf.open(&path).unwrap();
    assert_eq!(store.len(), 12);
    let mut state = WorkflowState::Idle;
    state = transition(&state, &WorkflowKind::Discover, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Discovered);
    state = transition(&state, &WorkflowKind::Open, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Opened);

    // Search "auth"
    let results = read_wf.search(&store, "auth");
    assert!(
        results
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_auth")
    );
    state = transition(&state, &WorkflowKind::Search, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Searched);

    // Read node_auth
    ctx.select("node_auth");
    let e = read_wf.read(&store, "node_auth").unwrap();
    assert_eq!(e.id().unwrap().as_str(), "node_auth");
    state = transition(&state, &WorkflowKind::Read, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Read);

    // Agent decides more context is required -> follow parent
    // This is an agent decision, not engine determinism
    let agent_decision = AgentDecision::NeedsSource {
        entity_id: "node_auth".into(),
    };
    assert!(matches!(agent_decision, AgentDecision::NeedsSource { .. }));

    let parent = read_wf.follow_parent(&store, e).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "region_api");
    // Followed parent is still a Read capability
    state = transition(&state, &WorkflowKind::FollowParent, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Read);
    let _parent_read = read_wf.read(&store, "region_api").unwrap();

    // Agent decides information is sufficient -> Validate -> Close
    assert_eq!(
        permission_for(WorkflowKind::Read, &ctx),
        Permission::ReadOnly
    );
    state = transition(&state, &WorkflowKind::Validate, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Validated);
    state = transition(&state, &WorkflowKind::Close, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Closed);

    // No mutation occurred
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.len(), 12);
    assert!(store2.validate().is_ok());
    let _ = std::fs::remove_file(&path);
}

// 2. end_to_end_search_again
#[test]
fn end_to_end_search_again() {
    let path = temp_aimt_copy();
    let wf = ReadWorkflow::new();
    let store = wf.open(&path).unwrap();

    // First search
    let first: Vec<_> = wf
        .search(&store, "auth")
        .into_iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert!(first.contains(&"node_auth".to_string()));
    let _ = wf.read(&store, &first[0]).unwrap();

    // Agent: NeedsSource -> search again with different query (not hard-coded next step)
    let decision = AgentDecision::NeedsSource {
        entity_id: "node_auth".into(),
    };
    assert!(matches!(decision, AgentDecision::NeedsSource { .. }));

    // Second search is legitimate continuation, not a fixed Vec<Step>
    let second: Vec<_> = wf
        .search(&store, "posts")
        .into_iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert!(second.contains(&"node_posts".to_string()));
    assert_ne!(first, second);

    // Engine should allow Search -> Read -> Search -> Read without error
    let ctx = WorkflowContext::new();
    let mut state = WorkflowState::Opened;
    state = transition(&state, &WorkflowKind::Search, &ctx).unwrap();
    state = transition(&state, &WorkflowKind::Read, &ctx).unwrap();
    // branching: search again
    state = transition(&state, &WorkflowKind::Search, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Searched);
    state = transition(&state, &WorkflowKind::Read, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Read);

    let _ = std::fs::remove_file(&path);
}

// 3. end_to_end_multi_hop_exploration
#[test]
fn end_to_end_multi_hop_exploration() {
    let path = temp_aimt_copy();
    let wf = ReadWorkflow::new();
    let store = wf.open(&path).unwrap();

    // node_auth -> parent region_api -> parent domain_backend
    let mut cur = wf.read(&store, "node_auth").unwrap().clone();
    cur = wf.follow_parent(&store, &cur).unwrap().clone();
    assert_eq!(cur.id().unwrap().as_str(), "region_api");
    cur = wf.follow_parent(&store, &cur).unwrap().clone();
    assert_eq!(cur.id().unwrap().as_str(), "domain_backend");
    let _ = wf.read(&store, cur.id().unwrap().as_str()).unwrap();

    // Then optionally: node_auth -> relation -> node_posts -> file -> file_auth
    let rels = wf.follow_relations(&store, "node_auth");
    assert!(!rels.is_empty());
    let to_entity = rels[0];
    assert_eq!(to_entity.id().unwrap().as_str(), "node_posts");

    // file traversal
    let frame = wf.read(&store, "frame_auth_login").unwrap();
    let file = wf.follow_file(&store, frame).unwrap();
    assert_eq!(file.id().unwrap().as_str(), "file_auth");

    // No graph abstraction needed — just repeated follow_* calls
    let _ = std::fs::remove_file(&path);
}

// 4. end_to_end_should_update_requires_explicit_intent
#[test]
fn end_to_end_should_update_requires_explicit_intent() {
    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.mark_dirty();
    // Without intent, engine must ask agent, not execute update
    let suggestion = engine.suggest_next(&ctx);
    match suggestion {
        Suggestion::AskAgent(AgentDecision::ShouldUpdate { reason }) => {
            assert!(reason.contains("dirty"));
        }
        _ => panic!("expected AskAgent(ShouldUpdate) when dirty without intent"),
    }
    assert_eq!(
        permission_for(WorkflowKind::Update, &ctx),
        Permission::ReadOnly
    );
}

// 5. end_to_end_update_without_intent_is_rejected
#[test]
fn end_to_end_update_without_intent_is_rejected() {
    let path = temp_aimt_copy();
    let mut store = Store::open(&path).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&path);
    ctx.mark_dirty();
    // No grant_update_intent
    let entity = make_node("node_tmp_no_intent", "region_api");
    let result = UpdateWorkflow::new().execute(&mut store, &mut ctx, entity);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind_str(), "Write");
    // File remains unchanged
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.len(), 12);
    assert!(!store2.contains("node_tmp_no_intent"));
    let _ = std::fs::remove_file(&path);
}

// 6. end_to_end_invalid_update_does_not_persist
#[test]
fn end_to_end_invalid_update_does_not_persist() {
    let path = temp_aimt_copy();
    let mut store = Store::open(&path).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&path);
    ctx.grant_update_intent();
    let invalid = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", "bad_node2")],
        body: vec![field("parent", "region_api")],
        relations: vec![],
    };
    let result = UpdateWorkflow::new().execute(&mut store, &mut ctx, invalid);
    assert!(result.is_err());
    let store2 = Store::open(&path).unwrap();
    assert!(!store2.contains("bad_node2"));
    assert_eq!(store2.len(), 12);
    assert!(store2.validate().is_ok());
    let _ = std::fs::remove_file(&path);
}

// 7. end_to_end_valid_update_persists
#[test]
fn end_to_end_valid_update_persists() {
    let path = temp_aimt_copy();
    let mut store = Store::open(&path).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&path);
    ctx.grant_update_intent();
    ctx.mark_dirty();
    let entity = make_node("node_tmp_valid", "region_api");
    UpdateWorkflow::new()
        .execute(&mut store, &mut ctx, entity)
        .unwrap();
    // Reopen and verify new state exists (proves composition does not make Update read-only)
    let store2 = Store::open(&path).unwrap();
    assert!(store2.contains("node_tmp_valid"));
    assert_eq!(store2.len(), 13);
    assert!(store2.validate().is_ok());
    // Close semantics: Validate -> Close
    let mut ctx2 = WorkflowContext::new();
    ctx2.grant_update_intent();
    let state = WorkflowState::Dirty;
    let state = transition(&state, &WorkflowKind::Validate, &ctx2).unwrap();
    assert_eq!(state, WorkflowState::Validated);
    let state = transition(&state, &WorkflowKind::Close, &ctx2).unwrap();
    assert_eq!(state, WorkflowState::Closed);
    let _ = std::fs::remove_file(&path);
}

// 8. end_to_end_close_requires_validation
#[test]
fn end_to_end_close_requires_validation() {
    let ctx = WorkflowContext::new();
    // Read-only exploration -> Validate -> Close is valid
    let state = WorkflowState::Read;
    let state = transition(&state, &WorkflowKind::Validate, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Validated);
    let state = transition(&state, &WorkflowKind::Close, &ctx).unwrap();
    assert_eq!(state, WorkflowState::Closed);

    // Dirty update must not bypass Validate
    let mut dirty_ctx = WorkflowContext::new();
    dirty_ctx.mark_dirty();
    dirty_ctx.grant_update_intent();
    let dirty_state = WorkflowState::Dirty;
    let err = transition(&dirty_state, &WorkflowKind::Close, &dirty_ctx).unwrap_err();
    assert!(err.to_string().contains("must validate before close"));

    // Invalid transition still explicit error, no silent fallback
    let err2 = transition(&WorkflowState::Idle, &WorkflowKind::Read, &ctx).unwrap_err();
    assert!(err2.to_string().contains("invalid transition"));
}
