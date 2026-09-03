use aimt::core::data::store::Store;
use aimt::workflows::context::WorkflowContext;
use aimt::workflows::engine::{Suggestion, WorkflowEngine};
use aimt::workflows::read::ReadWorkflow;
use std::path::Path;

#[test]
fn read_workflow_discover_open_search_read_follow_and_continue() {
    // This test requires the real ReadWorkflow to exist — it will fail until implemented
    let _workflow = ReadWorkflow::new();
    // Use real .aimt fixture with 12 entities
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = Store::open(&aimt_path).unwrap();
    assert_eq!(store.len(), 12);
    assert!(store.validate().is_ok());

    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&aimt_path);

    // 1. discover/open — engine should suggest Discover then Open
    // For now, we directly open via Store, but workflow should track state
    // Simulate discover -> open
    assert!(matches!(engine.suggest_next(&ctx), Suggestion::AskAgent(_))); // initially needs WhichEntity

    // 2. search for "auth"
    let results = store.find_by_level(aimt::core::model::Level::Node);
    let auth_nodes: Vec<_> = results
        .iter()
        .filter(|e| e.id().unwrap().as_str().contains("auth"))
        .collect();
    assert!(!auth_nodes.is_empty());

    // Also test search via Store::ids filtered (as visualizer does)
    let search_results: Vec<_> = store
        .ids()
        .into_iter()
        .filter(|id| id.contains("auth"))
        .collect();
    assert!(search_results.contains(&"node_auth".to_string()));

    // 3. read entity
    ctx.select("node_auth");
    let suggestion = engine.suggest_next(&ctx);
    assert!(matches!(suggestion, Suggestion::Execute(_)));

    let entity = store.get("node_auth").unwrap();
    assert_eq!(entity.level.as_str(), "node");
    assert_eq!(entity.id().unwrap().as_str(), "node_auth");

    // 4. follow at least one valid relationship — parent
    let parent_id = entity.field("parent").unwrap().value.clone();
    assert_eq!(parent_id, "region_api");
    let parent = store.get(&parent_id).unwrap();
    assert_eq!(parent.level.as_str(), "region");

    // follow file for a frame
    let frame = store.get("frame_auth_login").unwrap();
    let file_id = frame.field("file").unwrap().value.clone();
    assert_eq!(file_id, "file_auth");
    let file_entity = store.get(&file_id).unwrap();
    assert_eq!(file_entity.level.as_str(), "file");

    // follow relation
    let relations = store.relations_from("node_auth");
    assert!(!relations.is_empty());
    let rel = relations[0];
    assert_eq!(rel.get_value("to"), Some("node_posts"));

    // 5. produce next workflow suggestion — after read, engine should allow continuation
    // Simulate that after reading, context has selected, so engine suggests Read (or Follow)
    // The key is that workflow does not force a fixed linear sequence
    let mut ctx2 = WorkflowContext::new();
    ctx2.select("node_auth");
    let s = engine.suggest_next(&ctx2);
    assert!(matches!(s, Suggestion::Execute(_)) || matches!(s, Suggestion::AskAgent(_)));

    // 6. allow continuation rather than forcing fixed sequence — search again
    // After insufficient context, agent can search again with different query
    let second_search: Vec<_> = store
        .ids()
        .into_iter()
        .filter(|id| id.contains("posts"))
        .collect();
    assert!(second_search.contains(&"node_posts".to_string()));
    // Engine should allow another search after read (branching)
    let mut ctx3 = WorkflowContext::new();
    ctx3.select("node_auth");
    // Simulate that after reading node_auth, agent decides to search again
    // Engine should not prevent search — it should suggest AskAgent(WhichEntity) if no selection, or allow Search
    // For now, just verify engine can handle a new query after previous read
    ctx3.select("node_posts");
    let s3 = engine.suggest_next(&ctx3);
    assert!(matches!(
        s3,
        Suggestion::Execute(_) | Suggestion::AskAgent(_)
    ));

    // 7. eventually reach validate/close — read-only, so validate should succeed
    assert!(store.validate().is_ok());
    // Close is just validate + drop, no mutation
}

#[test]
fn read_workflow_branch_search_again_and_follow_parent() {
    let _workflow = ReadWorkflow::new();
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = Store::open(&aimt_path).unwrap();

    // Branch 1: search -> read -> insufficient -> search again
    let first_results: Vec<_> = store
        .ids()
        .into_iter()
        .filter(|id| id.contains("auth"))
        .collect();
    assert!(!first_results.is_empty());
    // Simulate reading first result but finding it insufficient, so search again with different query
    let second_results: Vec<_> = store
        .ids()
        .into_iter()
        .filter(|id| id.contains("region"))
        .collect();
    assert!(second_results.contains(&"region_api".to_string()));
    assert_ne!(first_results, second_results);

    // Branch 2: read child -> follow parent -> read parent
    let child = store.get("node_auth").unwrap();
    let parent_id = child.field("parent").unwrap().value.clone();
    let parent = store.get(&parent_id).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "region_api");
    // Follow parent's parent
    let grandparent_id = parent.field("parent").unwrap().value.clone();
    let grandparent = store.get(&grandparent_id).unwrap();
    assert_eq!(grandparent.id().unwrap().as_str(), "domain_backend");

    // Ensure workflow allows this chaining without forcing a fixed sequence
    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.select("node_auth");
    let s1 = engine.suggest_next(&ctx);
    // After reading node_auth, engine should not force Close, it should allow FollowParent
    assert!(matches!(
        s1,
        Suggestion::Execute(_) | Suggestion::AskAgent(_)
    ));
    ctx.select(&parent_id);
    let s2 = engine.suggest_next(&ctx);
    assert!(matches!(
        s2,
        Suggestion::Execute(_) | Suggestion::AskAgent(_)
    ));
}

#[test]
fn read_workflow_is_read_only() {
    let _workflow = ReadWorkflow::new();
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = Store::open(&aimt_path).unwrap();
    let ctx = WorkflowContext::new();
    // Read workflow must be read-only: no open_mut, no persist
    // Use permissions check
    use aimt::workflows::definition::WorkflowKind;
    use aimt::workflows::permissions::{Permission, permission_for};
    assert_eq!(
        permission_for(WorkflowKind::Read, &ctx),
        Permission::ReadOnly
    );
    assert_eq!(
        permission_for(WorkflowKind::Search, &ctx),
        Permission::ReadOnly
    );
    assert_eq!(
        permission_for(WorkflowKind::Discover, &ctx),
        Permission::ReadOnly
    );
    // Even with dirty, without update_intent, Read should stay ReadOnly
    let mut ctx_dirty = WorkflowContext::new();
    ctx_dirty.mark_dirty();
    assert_eq!(
        permission_for(WorkflowKind::Read, &ctx_dirty),
        Permission::ReadOnly
    );
    // Store itself should not have been mutated
    assert_eq!(store.len(), 12);
}
