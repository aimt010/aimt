use std::path::Path;

fn plugin_path_for(dir: &Path) -> std::path::PathBuf {
    dir.join(".opencode/plugins/aimt.js")
}

fn ensure_plugin_installed(dir: &Path) -> std::path::PathBuf {
    let path = plugin_path_for(dir);
    if !path.exists() {
        let installer = aimt::plugins::installer::installer_for("opencode")
            .expect("opencode installer should exist");
        installer
            .install(dir)
            .expect("install opencode should succeed");
    }
    path
}

// Test A — OpenCode can load the plugin (self-contained: installs into temp project)
#[test]
fn opencode_can_load_plugin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = ensure_plugin_installed(dir.path());
    assert!(
        path.exists(),
        "OpenCode plugin should exist at .opencode/plugins/aimt.js, got {}",
        path.display()
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("tool"), "plugin should define a tool");
    assert!(content.contains("aimt"), "plugin should be for AIMT");
}

// Test B — OpenCode can discover the AIMT tool (self-contained)
#[test]
fn opencode_can_discover_aimt_tool() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = ensure_plugin_installed(dir.path());
    let content = std::fs::read_to_string(&path).unwrap();
    // Tool should be registered under a name like "aimt" or "aimt_workflow"
    assert!(
        content.contains("\"aimt\"")
            || content.contains("'aimt'")
            || content.contains("aimt_workflow")
            || content.contains("aimt-workflow"),
        "plugin should register tool named aimt or aimt_workflow"
    );
}

// Test C — Read operation reaches AIMT
#[test]
fn read_operation_reaches_aimt() {
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    // Call the Rust bridge via CLI: aimt read or aimt workflow
    // For now, we test via the workflow engine directly, but the plugin should delegate to it
    // The plugin's tool should ultimately call WorkflowEngine -> ReadWorkflow -> Store
    let output = std::process::Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "aimt", "--help"])
        .output();
    // If the CLI doesn't have `aimt` subcommand yet, this will fail — which is the failing test we want
    if let Ok(output) = output {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should not be "unknown command"
        assert!(
            !stderr.contains("unknown command") && !stdout.contains("unknown command"),
            "aimt tool should be known, got stderr: {} stdout: {}",
            stderr,
            stdout
        );
    } else {
        panic!("failed to run aimt command");
    }

    // Also verify via direct Store that fixture has 12 entities
    let store = aimt::core::data::store::Store::open(&aimt_path).unwrap();
    assert_eq!(store.len(), 12);
    assert!(store.validate().is_ok());
}

// Test D — Search
#[test]
fn search_auth_locates_entity() {
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = aimt::core::data::store::Store::open(&aimt_path).unwrap();
    // Simulate what the OpenCode tool would do: search for "auth"
    let results: Vec<_> = store
        .ids()
        .into_iter()
        .filter(|id| id.contains("auth"))
        .collect();
    assert!(results.contains(&"node_auth".to_string()));
    // Also via workflow search
    let wf = aimt::workflows::read::ReadWorkflow::new();
    let search_results = wf.search(&store, "auth");
    assert!(
        search_results
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_auth")
    );
}

// Test E — Source exploration
#[test]
fn source_exploration_from_node_auth() {
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = aimt::core::data::store::Store::open(&aimt_path).unwrap();
    let wf = aimt::workflows::read::ReadWorkflow::new();
    let node = wf.read(&store, "node_auth").unwrap();
    let parent = wf.follow_parent(&store, node).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "region_api");
    let file = wf
        .follow_file(&store, store.get("frame_auth_login").unwrap())
        .unwrap();
    assert_eq!(file.id().unwrap().as_str(), "file_auth");
    let relations = wf.follow_relations(&store, "node_auth");
    assert!(!relations.is_empty());
}

// Test F — Agent decision remains visible
#[test]
fn agent_decision_remains_visible() {
    use aimt::workflows::capabilities::AgentDecision;
    use aimt::workflows::context::WorkflowContext;
    use aimt::workflows::engine::{Suggestion, WorkflowEngine};

    let engine = WorkflowEngine::new();
    let mut ctx = WorkflowContext::new();
    ctx.mark_dirty();
    // Without intent, engine should ask agent, not silently execute
    let suggestion = engine.suggest_next(&ctx);
    match suggestion {
        Suggestion::AskAgent(AgentDecision::ShouldUpdate { .. }) => {}
        _ => panic!("expected AskAgent(ShouldUpdate) for dirty without intent"),
    }
    // The OpenCode tool should expose this decision, not hide it
    // For now, we test that the adapter would show it
    let adapter = aimt::plugins::opencode::OpenCodeAdapter;
    use aimt::plugins::adapter::WorkflowAdapter;
    let action = adapter.translate(suggestion);
    let s = action.to_string();
    assert!(
        !s.is_empty() && s.contains("ShouldUpdate"),
        "adapter should expose ShouldUpdate, got {}",
        s
    );
}

// Test G — Update remains protected
#[test]
fn update_remains_protected() {
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let tmp = std::env::temp_dir().join(format!(
        "aimt_opencode_g_{}_{}.aimt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(&aimt_path, &tmp).unwrap();
    let mut store = aimt::core::data::store::Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.mark_dirty();
    // No update_intent
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
            header: vec![f("id", "opencode_no_intent"), f("title", "Test")],
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
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind_str(), "Write");
    let store2 = aimt::core::data::store::Store::open(&tmp).unwrap();
    assert_eq!(store2.len(), 12);
    let _ = std::fs::remove_file(&tmp);
}

// Test H — Invalid update does not persist
#[test]
fn invalid_update_does_not_persist() {
    let aimt_path = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let tmp = std::env::temp_dir().join(format!(
        "aimt_opencode_h_{}_{}.aimt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::copy(&aimt_path, &tmp).unwrap();
    let mut store = aimt::core::data::store::Store::open(&tmp).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&tmp);
    ctx.grant_update_intent();
    let invalid = {
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
            header: vec![f("id", "bad_opencode")],
            body: vec![f("parent", "region_api")],
            relations: vec![],
        }
    };
    let result =
        aimt::workflows::update::UpdateWorkflow::new().execute(&mut store, &mut ctx, invalid);
    assert!(result.is_err());
    let store2 = aimt::core::data::store::Store::open(&tmp).unwrap();
    assert!(!store2.contains("bad_opencode"));
    assert_eq!(store2.len(), 12);
    let _ = std::fs::remove_file(&tmp);
}
