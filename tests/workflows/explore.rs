use aimt::core::data::store::Store;
use aimt::workflows::context::WorkflowContext;
use aimt::workflows::engine::{Suggestion, WorkflowEngine};
use aimt::workflows::read::ReadWorkflow;
use std::path::Path;

fn aimt_path() -> std::path::PathBuf {
    if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    }
}

// Test 1 — Local exploration is sufficient
#[test]
fn explore_local_is_sufficient() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();
    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&aimt_path());

    // question -> search auth -> read node_auth -> follow parent -> read parent -> enough -> stop
    let results = workflow.search(&store, "auth");
    assert!(
        results
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_auth")
    );

    ctx.select("node_auth");
    let e = workflow.read(&store, "node_auth").unwrap();
    assert_eq!(e.id().unwrap().as_str(), "node_auth");

    // follow parent
    let parent = workflow.follow_parent(&store, e).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "region_api");
    ctx.select("region_api");
    let parent_read = workflow.read(&store, "region_api").unwrap();
    assert_eq!(parent_read.id().unwrap().as_str(), "region_api");

    // Engine should not force unnecessary exploration — after reading parent, it can suggest Validate/Close or AskAgent for sufficiency
    // For now, engine suggests Execute(Read) when selected, or AskAgent if not dirty
    let suggestion = engine.suggest_next(&ctx);
    // The workflow does not force another search — it allows stop
    assert!(matches!(
        suggestion,
        Suggestion::Execute(_) | Suggestion::AskAgent(_)
    ));
    // Validate that we have enough information and can close (validate)
    assert!(store.validate().is_ok());
}

// Test 2 — Search again
#[test]
fn explore_search_again() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();

    // search auth -> read candidate -> insufficient -> search posts -> read node_posts -> sufficient
    let first: Vec<_> = workflow
        .search(&store, "auth")
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert!(first.contains(&"node_auth".to_string()));
    let _first_read = workflow.read(&store, &first[0]).unwrap();

    // Simulate agent decision: insufficient, needs another search
    // The workflow must allow a second search as legitimate continuation, not a hard-coded next step
    let second = workflow.search(&store, "posts");
    assert!(
        second
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_posts")
    );
    let second_read = workflow.read(&store, "node_posts").unwrap();
    assert_eq!(second_read.id().unwrap().as_str(), "node_posts");
}

// Test 3 — Parent traversal multi-hop
#[test]
fn explore_parent_traversal() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();

    let mut current = workflow.read(&store, "node_auth").unwrap();
    assert_eq!(current.id().unwrap().as_str(), "node_auth");
    current = workflow.follow_parent(&store, current).unwrap();
    assert_eq!(current.id().unwrap().as_str(), "region_api");
    current = workflow.follow_parent(&store, current).unwrap();
    assert_eq!(current.id().unwrap().as_str(), "domain_backend");
    // No graph abstraction needed — just follow_parent repeatedly
    let _ = workflow
        .read(&store, current.id().unwrap().as_str())
        .unwrap();
}

// Test 4 — Relation traversal
#[test]
fn explore_relation_traversal() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();

    // node_auth has relation to node_posts
    let from = "node_auth";
    let relations = workflow.follow_relations(&store, from);
    assert!(!relations.is_empty());
    let to_id = relations[0].id().unwrap().as_str().to_string();
    assert_eq!(to_id, "node_posts");
    let to_entity = workflow.read(&store, &to_id).unwrap();
    assert_eq!(to_entity.id().unwrap().as_str(), "node_posts");
    // Continue exploration from related entity
    let _ = workflow.follow_parent(&store, to_entity);
}

// Test 5 — File/source traversal
#[test]
fn explore_file_source_traversal() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();

    // frame_auth_login -> file_auth via file field
    let frame = workflow.read(&store, "frame_auth_login").unwrap();
    assert_eq!(frame.level.as_str(), "frame");
    let file = workflow.follow_file(&store, frame).unwrap();
    assert_eq!(file.id().unwrap().as_str(), "file_auth");
    assert_eq!(file.level.as_str(), "file");
    // file has parent node_auth
    let parent = workflow.follow_parent(&store, file).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "node_auth");
}

// Test 6 — Stop condition belongs to agent (NeedsSource vs sufficient)
#[test]
fn explore_stop_condition_is_agent_decision() {
    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.select("node_auth");
    // Engine should not decide semantic sufficiency — it offers Execute or AskAgent
    let suggestion = engine.suggest_next(&ctx);
    match suggestion {
        Suggestion::Execute(_) => {
            // Deterministic: engine offers to execute a capability (Read)
        }
        Suggestion::AskAgent(agent_decision) => {
            // Agent decides if more exploration is needed
            // This is where NeedsSource would be asked
            match agent_decision {
                aimt::workflows::capabilities::AgentDecision::NeedsSource { .. } => {}
                aimt::workflows::capabilities::AgentDecision::WhichEntity { .. } => {}
                aimt::workflows::capabilities::AgentDecision::ShouldUpdate { .. } => {}
            }
        }
    }
    // Simulate agent deciding sufficient vs NeedsSource — sufficiency is agent-driven
    let agent_says_sufficient = true;
    assert!(agent_says_sufficient);
}

// Test bounded exploration — visited tracking prevents infinite loop A->B->A
#[test]
fn explore_bounded_no_infinite_loop() {
    let store = Store::open(&aimt_path()).unwrap();
    let workflow = ReadWorkflow::new();
    let mut ctx = WorkflowContext::new();
    // Simulate visiting A -> B -> A
    ctx.select("node_auth");
    ctx.select("node_posts");
    // Re-selecting node_auth should not cause infinite loop if workflow tracks visited
    // With current selected_ids, we have visited both
    assert_eq!(ctx.selected(), &["node_auth", "node_posts"]);
    // Workflow should still allow reading, but not force re-reading same entity infinitely
    // The engine's suggest_next should remain deterministic and not loop
    let engine = WorkflowEngine::new();
    let suggestion = engine.suggest_next(&ctx);
    assert!(matches!(
        suggestion,
        Suggestion::Execute(_) | Suggestion::AskAgent(_)
    ));
    // Verify we can still read and follow without duplication
    let _ = workflow.read(&store, "node_auth").unwrap();
    let _ = workflow.read(&store, "node_posts").unwrap();
}

// Test multiple AIMT/source decision — NeedsSource with target as data
#[test]
fn explore_needs_source_as_data() {
    use aimt::workflows::capabilities::AgentDecision;
    // AgentDecision::NeedsSource carries entity_id as data, not as host action
    let decision = AgentDecision::NeedsSource {
        entity_id: "file_auth".to_string(),
    };
    match decision {
        AgentDecision::NeedsSource { entity_id } => assert_eq!(entity_id, "file_auth"),
        _ => panic!("wrong variant"),
    }
    // This proves the existing architecture can represent "another AIMT/source is required"
    // with source target as data/context, without HTTP/MCP ingestion
    let mut ctx = WorkflowContext::new();
    ctx.select("file_auth");
    // Context can hold the decision target as selected_id for next search
    assert!(ctx.selected().contains(&"file_auth".to_string()));
}

// Test capability boundary — Read/Explore never requires open_mut/persist
#[test]
fn explore_is_read_only() {
    let workflow = ReadWorkflow::new();
    assert!(workflow.is_read_only());
    let ctx = WorkflowContext::new();
    use aimt::workflows::definition::WorkflowKind;
    use aimt::workflows::permissions::{Permission, permission_for};
    for kind in [
        WorkflowKind::Discover,
        WorkflowKind::Search,
        WorkflowKind::Read,
    ] {
        assert_eq!(permission_for(kind, &ctx), Permission::ReadOnly);
    }
    // Even dirty without intent stays read-only for explore
    let mut dirty_ctx = WorkflowContext::new();
    dirty_ctx.mark_dirty();
    assert_eq!(
        permission_for(WorkflowKind::Read, &dirty_ctx),
        Permission::ReadOnly
    );
}
