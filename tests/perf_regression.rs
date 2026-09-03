use aimt::core::data::store::Store;
use aimt::core::model::{AimtEntity, Field, Level};
use aimt::core::syntax::Span;
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

// 1. Store validation still detects all existing invalid conditions and ordering remains deterministic
#[test]
fn validation_still_detects_and_ordering_deterministic() {
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    assert!(store.validate().is_ok());

    // Create a store with an invalid parent reference and check that validate still catches it
    // Use a temp package with one invalid node
    let tmp = std::env::temp_dir().join(format!("aimt_perf_val_{}.aimt", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let src = Path::new("aimt-test-project.aimt");
    std::fs::copy(src, &tmp).unwrap();
    let _store = Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    let bad = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", "bad_validate"), field("title", "Bad")],
        body: vec![
            field("parent", "nonexistent_parent_123"),
            field("description", "x"),
            field("type", "service"),
        ],
        relations: vec![],
    };
    // Use UpdateWorkflow to insert and validate (will fail integrity, not field validation)
    // For this test, we directly insert and check store.validate
    let mut store2 = Store::open(&tmp).unwrap();
    // Manually insert invalid parent via Store::insert (field validation passes, integrity will fail on store.validate)
    store2.insert(bad).unwrap();
    let errs = store2.validate().unwrap_err();
    assert!(!errs.is_empty());
    assert!(
        errs.iter()
            .any(|e| e.field.as_deref() == Some("parent") && e.id == "bad_validate")
    );
    // Check deterministic ordering: errors sorted by (id, field) — manual compare without PartialEq
    let mut sorted = errs.clone();
    sorted.sort_by(|a, b| a.id.cmp(&b.id).then(a.field.cmp(&b.field)));
    assert_eq!(errs.len(), sorted.len());
    for (a, b) in errs.iter().zip(sorted.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.field, b.field);
    }
    let _ = std::fs::remove_file(&tmp);
}

// 2. Search results remain identical before/after optimization
#[test]
fn search_results_identical() {
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    let wf = aimt::workflows::read::ReadWorkflow::new();
    let queries = [
        "auth",
        "Auth",
        "AUTH",
        "node",
        "DOMAIN",
        "region",
        "file_auth",
        "",
    ];
    for q in queries {
        let r1 = wf.search(&store, q);
        // Simulate old behavior: clone + to_lowercase for each field
        let q_lower = q.to_lowercase();
        let r2: Vec<_> = store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(&id))
            .filter(|e| {
                let id = e
                    .id()
                    .map(|i| i.as_str().to_lowercase())
                    .unwrap_or_default();
                let title = e
                    .field("title")
                    .map(|f| f.value.clone().to_lowercase())
                    .unwrap_or_default();
                let level = e.level.as_str().to_lowercase();
                id.contains(&q_lower) || title.contains(&q_lower) || level.contains(&q_lower)
            })
            .collect();
        let ids1: Vec<_> = r1
            .iter()
            .map(|e| e.id().unwrap().as_str().to_string())
            .collect();
        let ids2: Vec<_> = r2
            .iter()
            .map(|e| e.id().unwrap().as_str().to_string())
            .collect();
        assert_eq!(
            ids1, ids2,
            "search results must be identical for query {:?}",
            q
        );
    }
}

// 3. Case-insensitive semantics remain unchanged (Unicode)
#[test]
fn case_insensitive_unicode_preserved() {
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    let wf = aimt::workflows::read::ReadWorkflow::new();
    // ASCII case-insensitive
    assert_eq!(
        wf.search(&store, "auth").len(),
        wf.search(&store, "AUTH").len()
    );
    assert_eq!(
        wf.search(&store, "auth").len(),
        wf.search(&store, "Auth").len()
    );
    // Unicode: title with accent (if any) — test with synthetic entity
    // Create a temp entity with title "Café" and search for "café" vs "CAFÉ"
    let tmp = std::env::temp_dir().join(format!("aimt_perf_unicode_{}.aimt", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::fs::copy("aimt-test-project.aimt", &tmp).unwrap();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    let ent = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", "node_unicode"), field("title", "Café")],
        body: vec![
            field("parent", "region_api"),
            field("description", "x"),
            field("type", "service"),
        ],
        relations: vec![],
    };
    aimt::workflows::update::UpdateWorkflow::new()
        .execute(&mut store, &mut ctx, ent)
        .unwrap();
    let store2 = Store::open(&tmp).unwrap();
    let wf2 = aimt::workflows::read::ReadWorkflow::new();
    assert_eq!(
        wf2.search(&store2, "café").len(),
        wf2.search(&store2, "CAFÉ").len()
    );
    assert_eq!(wf2.search(&store2, "café").len(), 1);
    let _ = std::fs::remove_file(&tmp);
}

// 4. ReadWorkflow behavior remains unchanged
#[test]
fn read_workflow_unchanged() {
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    let wf = aimt::workflows::read::ReadWorkflow::new();
    assert!(wf.is_read_only());
    assert_eq!(wf.search(&store, "auth").len(), 3);
    assert!(wf.read(&store, "node_auth").is_some());
    assert_eq!(
        wf.follow_parent(&store, wf.read(&store, "node_auth").unwrap())
            .unwrap()
            .id()
            .unwrap()
            .as_str(),
        "region_api"
    );
}

// 5. UpdateWorkflow behavior remains unchanged
#[test]
fn update_workflow_unchanged() {
    let src = Path::new("aimt-test-project.aimt");
    let tmp = std::env::temp_dir().join(format!("aimt_perf_update_{}.aimt", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::fs::copy(src, &tmp).unwrap();
    let mut store = Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    let ent = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("id", "node_perf_test"), field("title", "Perf")],
        body: vec![
            field("parent", "region_api"),
            field("description", "x"),
            field("type", "service"),
        ],
        relations: vec![],
    };
    assert!(
        aimt::workflows::update::UpdateWorkflow::new()
            .execute(&mut store, &mut ctx, ent)
            .is_ok()
    );
    let store2 = Store::open(&tmp).unwrap();
    assert!(store2.contains("node_perf_test"));
    let _ = std::fs::remove_file(&tmp);
}

// 6. AIMT fixture remains unchanged
#[test]
fn fixture_unchanged() {
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    assert_eq!(store.len(), 12);
    assert!(store.validate().is_ok());
    // Check expected ids still present
    for id in [
        "aimt_miniblog",
        "map_miniblog",
        "domain_backend",
        "region_api",
        "node_auth",
        "file_auth",
        "frame_auth_login",
    ] {
        assert!(store.contains(id), "missing {}", id);
    }
}
