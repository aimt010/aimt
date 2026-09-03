use aimt::hosted::HostedEngine;
use aimt::model::Level;
use std::path::Path;

fn fixture_path() -> std::path::PathBuf {
    let p = Path::new("aimt-test-project.aimt");
    if p.exists() {
        p.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    }
}

// hosted engine can open a valid .aimt
#[test]
fn hosted_can_open_valid_aimt() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    assert_eq!(engine.len(), 12);
    assert!(engine.is_package());
    assert_eq!(engine.source_path(), fixture_path().as_path());
}

// hosted engine can read an entity
#[test]
fn hosted_can_read_entity() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    let e = engine.get("node_auth").expect("node_auth must exist");
    assert_eq!(e.id().unwrap().as_str(), "node_auth");
    assert_eq!(e.level.as_str(), "node");
    // read alias
    let e2 = engine.read("domain_backend").unwrap();
    assert_eq!(e2.level, Level::Domain);
}

// hosted engine can search
#[test]
fn hosted_can_search() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    let results = engine.search("auth");
    assert!(
        results
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_auth")
    );
    assert!(
        results
            .iter()
            .any(|e| e.id().unwrap().as_str() == "file_auth")
    );
    // level search
    let nodes = engine.find_by_level(Level::Node);
    assert!(!nodes.is_empty());
    assert!(nodes.iter().all(|e| e.level == Level::Node));
    // case-insensitive
    let upper = engine.search("AUTH");
    assert_eq!(upper.len(), results.len());
    // empty query returns all
    let all = engine.search("");
    assert_eq!(all.len(), 12);
}

// hosted engine can follow parent/relations/file
#[test]
fn hosted_can_follow_parent() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    let node = engine.get("node_auth").unwrap();
    let parent = engine.follow_parent(node).unwrap();
    assert_eq!(parent.id().unwrap().as_str(), "region_api");
    let children = engine.children_of("region_api");
    assert!(
        children
            .iter()
            .any(|e| e.id().unwrap().as_str() == "node_auth")
    );
}

#[test]
fn hosted_can_follow_file() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    let frame = engine.get("frame_auth_login").unwrap();
    let file = engine.follow_file(frame).unwrap();
    assert_eq!(file.id().unwrap().as_str(), "file_auth");
    assert_eq!(file.level, Level::File);
    // non-frame without file should return None
    let domain = engine.get("domain_backend").unwrap();
    assert!(engine.follow_file(domain).is_none());
}

#[test]
fn hosted_can_follow_relations() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    let rels_from = engine.relations_from("node_auth");
    assert!(!rels_from.is_empty());
    assert_eq!(rels_from[0].get_value("to"), Some("node_posts"));
    // relations_to complementary
    let rels_to = engine.relations_to("node_posts");
    assert!(
        rels_to
            .iter()
            .any(|r| r.get_value("from") == Some("node_auth"))
    );
}

// hosted engine can validate
#[test]
fn hosted_can_validate() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    assert!(engine.validate().is_ok());
}

// hosted engine exposes no update/write/persist operation
#[test]
fn hosted_exposes_no_mutation() {
    // Hosted engine is now split into mod.rs (wiring) + engine.rs (implementation).
    // Check all files under src/hosted/.
    let hosted_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted");
    let mut combined = String::new();
    for entry in std::fs::read_dir(hosted_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            combined.push_str(&std::fs::read_to_string(&path).unwrap());
            combined.push('\n');
        }
    }
    let code = combined.split("#[cfg(test)]").next().unwrap().to_string();
    for term in [
        concat!("pub fn ", "insert"),
        concat!("pub fn ", "update"),
        concat!("pub fn ", "remove"),
        concat!("pub fn ", "persist"),
        concat!("pub fn ", "open_mut"),
    ] {
        assert!(
            !code.contains(term),
            "hosted engine must not expose {}",
            term
        );
    }
    let before_docs = code
        .split("Read-only by architecture")
        .next()
        .unwrap_or(&code);
    assert!(
        !before_docs.contains("open_mut") || !code.contains("pub fn open_mut"),
        "hosted engine must not expose open_mut as pub fn"
    );
}

// hosted engine has no imports from Claude/Codex/Cursor/Antigravity/OpenCode
#[test]
fn hosted_has_no_ai_specific_imports() {
    let hosted_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted");
    let mut combined = String::new();
    for entry in std::fs::read_dir(hosted_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            combined.push_str(&std::fs::read_to_string(&path).unwrap());
            combined.push('\n');
        }
    }
    let code = combined.split("#[cfg(test)]").next().unwrap().to_string();
    for pat in [
        "plugins::claude",
        "plugins::codex",
        "plugins::antigravity",
        "plugins::opencode",
        "plugins::adapter",
    ] {
        assert!(!code.contains(pat), "hosted must not import {}", pat);
    }
    assert!(
        !code.contains("WorkflowAdapter") && !code.contains("GenericAdapter"),
        "hosted must not use Adapter"
    );
}

// existing development workflow behavior remains unchanged
#[test]
fn existing_development_workflow_unchanged() {
    // Store still supports mutation via open_mut/insert/persist
    use aimt::model::{AimtEntity, Field, Level};
    use aimt::store::Store;
    use aimt::syntax::Span;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_hosted_dev_check_{}_{}",
        std::process::id(),
        id
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let dummy_span = Span::range(1, 1, 1, 1);
    let e = AimtEntity {
        level: Level::Domain,
        level_span: dummy_span.clone(),
        span: dummy_span.clone(),
        header: vec![
            Field {
                name: "id".to_string(),
                value: "d1".to_string(),
                span: dummy_span.clone(),
            },
            Field {
                name: "title".to_string(),
                value: "D".to_string(),
                span: dummy_span.clone(),
            },
        ],
        body: vec![Field {
            name: "description".to_string(),
            value: "hi".to_string(),
            span: dummy_span.clone(),
        }],
        relations: vec![],
    };
    aimt::crud::create(&dir, &e).unwrap();
    let mut store = Store::open_mut(&dir).unwrap();
    assert_eq!(store.len(), 1);
    // Ensure HostedEngine cannot mutate, but Store still can
    let n = AimtEntity {
        level: Level::Node,
        level_span: dummy_span.clone(),
        span: dummy_span.clone(),
        header: vec![
            Field {
                name: "id".to_string(),
                value: "n1".to_string(),
                span: dummy_span.clone(),
            },
            Field {
                name: "title".to_string(),
                value: "N".to_string(),
                span: dummy_span.clone(),
            },
        ],
        body: vec![Field {
            name: "parent".to_string(),
            value: "d1".to_string(),
            span: dummy_span,
        }],
        relations: vec![],
    };
    store.insert(n).unwrap();
    store.persist().unwrap();
    assert_eq!(store.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);

    // Workflows still enforce read-only boundary via permissions
    use aimt::workflows::context::WorkflowContext;
    use aimt::workflows::definition::WorkflowKind;
    use aimt::workflows::permissions::{Permission, permission_for};
    let ctx = WorkflowContext::new();
    assert_eq!(
        permission_for(WorkflowKind::Read, &ctx),
        Permission::ReadOnly
    );
    assert_eq!(
        permission_for(WorkflowKind::Update, &ctx),
        Permission::ReadOnly
    );
}

// discover/open/search/read/children_of/follow_file/relations_from/validate/close all present
#[test]
fn hosted_supports_all_required_capabilities() {
    let engine = HostedEngine::open(&fixture_path()).unwrap();
    // discover (static) — use a temp dir to avoid workspace validation failures
    let tmp = std::env::temp_dir().join(format!("aimt_hosted_discover_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let discovered = HostedEngine::discover(&tmp).unwrap();
    let _ = discovered;
    let _ = std::fs::remove_dir_all(&tmp);
    // open already tested
    // search
    let _ = engine.search("domain");
    // read/get — use a real id from fixture
    let _ = engine.get("aimt_miniblog").unwrap();
    let _ = engine.read("map_miniblog").unwrap();
    // children_of
    let _ = engine.children_of("domain_backend");
    // follow_file
    let frame = engine.get("frame_auth_login").unwrap();
    let _ = engine.follow_file(frame).unwrap();
    // relations_from
    let _ = engine.relations_from("node_auth");
    // validate
    assert!(engine.validate().is_ok());
    // close (consumes)
    engine.close();
}

// Single .aimt artifact remains portable — no second file required
#[test]
fn hosted_single_artifact() {
    // Opening the .aimt file directly must succeed without needing a sibling file
    let pkg_path = fixture_path();
    assert!(pkg_path.is_file());
    let engine = HostedEngine::open(&pkg_path).unwrap();
    assert!(engine.is_package());
    // Ensure MAGIC/VERSION unchanged by reading raw bytes
    let data = std::fs::read(&pkg_path).unwrap();
    assert_eq!(&data[0..4], &[0x41, 0x49, 0x4D, 0x54]); // "AIMT"
    assert_eq!(data[4], 0x01); // VERSION
}

// HostedEngine cannot mutate even with key (protected package)
#[test]
fn hosted_cannot_mutate_even_with_key() {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!("aimt_hosted_mut_key_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    let pkg = dir.join("p.aimt");
    let out = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "init",
            &pkg.to_string_lossy(),
        ])
        .output()
        .expect("cargo run init");
    assert!(out.status.success());
    // Extract private for completeness, but HostedEngine has no API to use it
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let combined = format!("{stdout} {stderr}");
    let tokens: Vec<String> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .map(|s| s.to_lowercase())
        .collect();
    assert!(!tokens.is_empty());
    // HostedEngine open without key still works (public read)
    let engine = HostedEngine::open(&pkg).expect("hosted open protected without key");
    assert!(engine.get("aimt").is_some());
    assert!(engine.validate().is_ok());
    // No mutation API even though we have a valid key
    let hosted_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted");
    let mut combined_code = String::new();
    for entry in std::fs::read_dir(hosted_dir).unwrap() {
        let entry = entry.unwrap();
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            combined_code.push_str(&std::fs::read_to_string(&p).unwrap());
            combined_code.push('\n');
        }
    }
    let code = combined_code
        .split("#[cfg(test)]")
        .next()
        .unwrap()
        .to_string();
    for term in [
        concat!("pub fn ", "insert"),
        concat!("pub fn ", "update"),
        concat!("pub fn ", "remove"),
        concat!("pub fn ", "persist"),
        concat!("pub fn ", "open_mut"),
        "open_mut_authenticated",
        "write_key",
    ] {
        assert!(
            !code.contains(term),
            "hosted must not expose {} even with key",
            term
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
