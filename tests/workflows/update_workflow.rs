use aimt::core::data::store::Store;
use aimt::core::model::{AimtEntity, Field, Level};
use aimt::core::syntax::Span;
use aimt::workflows::context::WorkflowContext;
use aimt::workflows::definition::WorkflowKind;
use aimt::workflows::permissions::{Permission, permission_for};
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
        "aimt_update_test_{}_{}_{}.aimt",
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

#[test]
fn update_workflow_read_only_by_default() {
    let _workflow = UpdateWorkflow::new();
    let mut ctx = WorkflowContext::new();
    ctx.mark_dirty();
    // dirty without intent must NOT allow persist
    assert_eq!(
        permission_for(WorkflowKind::Update, &ctx),
        Permission::ReadOnly
    );
    // UpdateWorkflow should refuse to persist without intent
    let tmp = temp_aimt_copy();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx2 = WorkflowContext::new();
    ctx2.set_path(&tmp);
    ctx2.mark_dirty();
    // try to execute update without intent — should fail at permission check, not call open_mut
    let entity = make_node("node_tmp_1", "region_api");
    let result = UpdateWorkflow::new().execute(&mut store, &mut ctx2, entity);
    assert!(result.is_err(), "should not persist without update_intent");
    assert_eq!(result.unwrap_err().kind_str(), "Write"); // or Validation/Permission
    // Ensure file was not mutated (still 12 entities)
    let store2 = Store::open(&tmp).unwrap();
    assert_eq!(store2.len(), 12);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn update_workflow_explicit_intent_enables_writing() {
    let _workflow = UpdateWorkflow::new();
    let tmp = temp_aimt_copy();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    ctx.mark_dirty();
    // Task 3: write credential required for protected stores; legacy allows dummy
    ctx.set_write_key("dummy_credential_for_legacy_test_aaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(
        permission_for(WorkflowKind::Update, &ctx),
        Permission::ReadWrite
    );
    let entity = make_node("node_tmp_2", "region_api");
    let result = UpdateWorkflow::new().execute(&mut store, &mut ctx, entity);
    assert!(
        result.is_ok(),
        "with intent should succeed: {:?}",
        result.err()
    );
    // Verify persisted
    let store2 = Store::open(&tmp).unwrap();
    assert!(store2.contains("node_tmp_2"));
    assert_eq!(store2.len(), 13);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn update_workflow_validation_before_persist() {
    let tmp = temp_aimt_copy();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    let entity = make_node("node_tmp_3", "region_api");
    // This should do open_mut → mutate → validate → persist and produce valid package
    let workflow = UpdateWorkflow::new();
    workflow.execute(&mut store, &mut ctx, entity).unwrap();
    let store2 = Store::open(&tmp).unwrap();
    assert!(store2.validate().is_ok());
    assert!(store2.contains("node_tmp_3"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn update_workflow_invalid_does_not_persist() {
    let tmp = temp_aimt_copy();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    // Create invalid entity: missing required field title for @node
    let invalid = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", "bad_node")],
        body: vec![field("parent", "region_api")],
        relations: vec![],
    };
    let workflow = UpdateWorkflow::new();
    let result = workflow.execute(&mut store, &mut ctx, invalid);
    assert!(result.is_err());
    // Ensure not persisted
    let store2 = Store::open(&tmp).unwrap();
    assert!(!store2.contains("bad_node"));
    assert_eq!(store2.len(), 12);
    // Original file unchanged
    assert!(store2.validate().is_ok());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn dirty_and_intent_are_separate() {
    let mut ctx = WorkflowContext::new();
    assert!(!ctx.is_dirty());
    assert!(!ctx.has_update_intent());
    ctx.mark_dirty();
    assert!(ctx.is_dirty());
    assert!(!ctx.has_update_intent());
    ctx = WorkflowContext::new();
    ctx.grant_update_intent();
    assert!(!ctx.is_dirty());
    assert!(ctx.has_update_intent());
}

#[test]
fn update_workflow_lifecycle_requires_validate_before_close() {
    use aimt::workflows::definition::WorkflowKind as K;
    use aimt::workflows::lifecycle::{WorkflowState, transition};
    // Valid Update path: Read -> Update (with intent) -> Validate -> Closed
    let mut ctx = WorkflowContext::new();
    ctx.grant_update_intent();
    let s = WorkflowState::Read;
    let s =
        transition(&s, &K::Update, &ctx).expect("Read->Update should be valid for update workflow");
    assert_eq!(s, WorkflowState::Dirty);
    let s = transition(&s, &K::Validate, &ctx).expect("Dirty->Validate should be valid");
    assert_eq!(s, WorkflowState::Validated);
    let s = transition(&s, &K::Close, &ctx).expect("Validated->Close should be valid");
    assert_eq!(s, WorkflowState::Closed);
    // Invalid: Update without intent
    let ctx2 = WorkflowContext::new();
    let err = transition(&WorkflowState::Read, &K::Update, &ctx2).unwrap_err();
    assert!(err.to_string().contains("without update_intent"));
    // Invalid: Update without going through Read, or Validate from Idle
    let err = transition(&WorkflowState::Idle, &K::Update, &ctx).unwrap_err();
    assert!(err.to_string().contains("invalid transition"));
    let err2 = transition(&WorkflowState::Idle, &K::Validate, &ctx).unwrap_err();
    assert!(err2.to_string().contains("invalid transition"));
    // Invalid: Dirty -> Close without Validate
    let mut ctx_dirty = WorkflowContext::new();
    ctx_dirty.mark_dirty();
    ctx_dirty.grant_update_intent();
    // Need to get to Dirty state first
    let s_dirty = transition(&WorkflowState::Read, &K::Update, &ctx_dirty).unwrap();
    assert_eq!(s_dirty, WorkflowState::Dirty);
    let err3 = transition(&s_dirty, &K::Close, &ctx_dirty).unwrap_err();
    assert!(err3.to_string().contains("must validate before close"));
}
