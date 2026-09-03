use aimt::index::{Index, IntegrityErrorKind};
use aimt::model::{AimtEntity, Field, Level};
use aimt::syntax::Span;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_index_test_{}_{}_{}",
        std::process::id(),
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
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
fn make_entity(level: Level, header: Vec<(&str, &str)>, body: Vec<(&str, &str)>) -> AimtEntity {
    AimtEntity {
        level,
        level_span: dummy_span(),
        span: dummy_span(),
        header: header.into_iter().map(|(k, v)| field(k, v)).collect(),
        body: body.into_iter().map(|(k, v)| field(k, v)).collect(),
        relations: vec![],
    }
}
fn make_with_relations(
    level: Level,
    header: Vec<(&str, &str)>,
    body: Vec<(&str, &str)>,
    relations: Vec<Vec<(&str, &str)>>,
) -> AimtEntity {
    let mut e = make_entity(level, header, body);
    for rel_fields in relations {
        e.relations.push(aimt::model::Relation {
            fields: rel_fields.into_iter().map(|(k, v)| field(k, v)).collect(),
            span: dummy_span(),
        });
    }
    e
}

// ---------------------------------------------------------------------------
// BUILD
// ---------------------------------------------------------------------------

#[test]
fn build_empty_workspace() {
    let dir = temp_dir();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.is_empty());
    assert_eq!(idx.len(), 0);
    assert!(idx.ids().is_empty());
    assert_eq!(idx.workspace(), dir.as_path());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_single_valid_entity() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let idx = Index::build(&dir).unwrap();
    assert_eq!(idx.len(), 1);
    assert!(idx.contains("a1"));
    assert_eq!(idx.get("a1").unwrap().id().unwrap().as_str(), "a1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_multiple_sorted() {
    let dir = temp_dir();
    for id in ["z_id", "a_id", "m_id"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&dir, &e).unwrap();
    }
    let idx = Index::build(&dir).unwrap();
    assert_eq!(idx.ids(), vec!["a_id", "m_id", "z_id"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_missing_workspace() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_index_missing_{}_{}",
        std::process::id(),
        9999
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let err = Index::build(&missing).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
}

#[test]
fn build_file_as_workspace() {
    let dir = temp_dir();
    let file = dir.join("not_a_dir");
    std::fs::write(&file, "hi").unwrap();
    let err = Index::build(&file).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_with_invalid_pmap() {
    let dir = temp_dir();
    std::fs::write(dir.join("bad.pmap"), "not a pmap").unwrap();
    let err = Index::build(&dir).unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_duplicate_id() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::writer::write(&dir.join("a_dup.pmap"), &e).unwrap();
    aimt::writer::write(&dir.join("z_dup.pmap"), &e).unwrap();
    let err = Index::build(&dir).unwrap_err();
    assert_eq!(err.kind_str(), "DuplicateId");
    if let aimt::index::IndexError::DuplicateId(d) = err {
        assert_eq!(d.id, "dup");
        assert_eq!(d.paths.len(), 2);
        assert!(d.paths[0] < d.paths[1]);
    } else {
        panic!("expected DuplicateId");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// VALIDATE
// ---------------------------------------------------------------------------

#[test]
fn validate_valid_parent() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &d).unwrap();
    aimt::crud::create(&dir, &n).unwrap();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.validate().is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_unknown_parent() {
    let dir = temp_dir();
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "does_not_exist")],
    );
    aimt::crud::create(&dir, &n).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].kind, IntegrityErrorKind::UnknownReference);
    assert_eq!(errs[0].field.as_deref(), Some("parent"));
    assert_eq!(errs[0].id, "n1");
    assert_eq!(errs[0].target, "does_not_exist");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_wrong_parent_level() {
    let dir = temp_dir();
    let f = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/x.rs")],
        vec![("hash", "h")],
    );
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "file_001")],
    );
    aimt::crud::create(&dir, &f).unwrap();
    aimt::crud::create(&dir, &n).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.kind == IntegrityErrorKind::InvalidParentTarget
                && e.field.as_deref() == Some("parent"))
    );
    let e = errs
        .iter()
        .find(|e| e.field.as_deref() == Some("parent"))
        .unwrap();
    assert_eq!(e.level, Level::Node);
    assert_eq!(e.target_level, Some(Level::File));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_file_target() {
    let dir = temp_dir();
    let f = make_entity(
        Level::Frame,
        vec![("id", "f1"), ("title", "T")],
        vec![("file", "missing"), ("target", "Foo"), ("type", "class")],
    );
    aimt::crud::create(&dir, &f).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.kind == IntegrityErrorKind::UnknownReference
                && e.field.as_deref() == Some("file"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_file_wrong_level() {
    let dir = temp_dir();
    let n = make_entity(
        Level::Node,
        vec![("id", "node_001"), ("title", "N")],
        vec![("parent", "d1")],
    );
    // Need parent d1 for node to be valid, but we test file wrong level, so create d1 and n1 first
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &d).unwrap();
    aimt::crud::create(&dir, &n).unwrap();
    let f = make_entity(
        Level::Frame,
        vec![("id", "f1"), ("title", "T")],
        vec![("file", "node_001"), ("target", "Foo"), ("type", "class")],
    );
    aimt::crud::create(&dir, &f).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.kind == IntegrityErrorKind::InvalidFileTarget)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_relation_from_to() {
    let dir = temp_dir();
    let n1 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N1")],
        vec![("parent", "d1")],
    );
    let d1 = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &d1).unwrap();
    aimt::crud::create(&dir, &n1).unwrap();
    let n2 = make_with_relations(
        Level::Node,
        vec![("id", "n2"), ("title", "N2")],
        vec![("parent", "d1")],
        vec![vec![("type", "uses"), ("from", "unknown"), ("to", "n1")]],
    );
    aimt::crud::create(&dir, &n2).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.kind == IntegrityErrorKind::UnknownReference
                && e.field.as_deref() == Some("relations[0].from"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_multiple_errors() {
    let dir = temp_dir();
    let n1 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "ghost1")],
    );
    let n2 = make_entity(
        Level::Node,
        vec![("id", "n2"), ("title", "N")],
        vec![("parent", "ghost2")],
    );
    aimt::crud::create(&dir, &n1).unwrap();
    aimt::crud::create(&dir, &n2).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].id, "n1");
    assert_eq!(errs[1].id, "n2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_empty_is_ok() {
    let dir = temp_dir();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.validate().is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_with_parent_optional() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &d).unwrap();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.validate().is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_after_package() {
    let ws = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&ws, &d).unwrap();
    aimt::crud::create(&ws, &n).unwrap();
    let pkg = temp_dir().join("p.aimt");
    aimt::package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    aimt::package::extract(&pkg, &dest).unwrap();
    let idx = Index::build(&dest).unwrap();
    assert!(idx.validate().is_ok());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

// ---------------------------------------------------------------------------
// QUERY
// ---------------------------------------------------------------------------

#[test]
fn ids_lexical() {
    let dir = temp_dir();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&dir, &e).unwrap();
    }
    let idx = Index::build(&dir).unwrap();
    assert_eq!(idx.ids(), vec!["a", "m", "z"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_existing() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &d).unwrap();
    aimt::crud::create(&dir, &e).unwrap();
    let idx = Index::build(&dir).unwrap();
    let got = idx.get("n1").unwrap();
    assert_eq!(got.id().unwrap().as_str(), "n1");
    assert_eq!(got.level, Level::Node);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_missing() {
    let dir = temp_dir();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.get("ghost").is_none());
    assert!(idx.get_indexed("ghost").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_by_level() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &d).unwrap();
    aimt::crud::create(&dir, &n).unwrap();
    let idx = Index::build(&dir).unwrap();
    let nodes = idx.find_by_level(Level::Node);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id().unwrap().as_str(), "n1");
    let domains = idx.find_by_level(Level::Domain);
    assert_eq!(domains.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn children_of() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n1 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N1")],
        vec![("parent", "d1")],
    );
    let n2 = make_entity(
        Level::Node,
        vec![("id", "n2"), ("title", "N2")],
        vec![("parent", "d1")],
    );
    let n3 = make_entity(
        Level::Node,
        vec![("id", "n3"), ("title", "N3")],
        vec![("parent", "other")],
    );
    // need other parent domain for n3 to be valid parent existence
    let d_other = make_entity(
        Level::Domain,
        vec![("id", "other"), ("title", "O")],
        vec![("description", "hi")],
    );
    for e in [d, d_other, n1, n2, n3] {
        aimt::crud::create(&dir, &e).unwrap();
    }
    let idx = Index::build(&dir).unwrap();
    let children = idx.children_of("d1");
    let ids: Vec<_> = children
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert_eq!(ids, vec!["n1", "n2"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn relations_from_to() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n1 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N1")],
        vec![("parent", "d1")],
    );
    let n2 = make_with_relations(
        Level::Node,
        vec![("id", "n2"), ("title", "N2")],
        vec![("parent", "d1")],
        vec![vec![("type", "uses"), ("from", "n2"), ("to", "n1")]],
    );
    for e in [d, n1, n2] {
        aimt::crud::create(&dir, &e).unwrap();
    }
    let idx = Index::build(&dir).unwrap();
    assert_eq!(idx.relations_from("n2").len(), 1);
    assert_eq!(idx.relations_to("n1").len(), 1);
    assert_eq!(idx.relations_from("ghost").len(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn query_deterministic() {
    let dir1 = temp_dir();
    let dir2 = temp_dir();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::writer::write(&dir1.join(format!("{}.pmap", id)), &e).unwrap();
        aimt::writer::write(&dir2.join(format!("{}.pmap", id)), &e).unwrap();
    }
    let idx1 = Index::build(&dir1).unwrap();
    let idx2 = Index::build(&dir2).unwrap();
    assert_eq!(idx1.ids(), idx2.ids());
    assert_eq!(
        idx1.children_of("ghost").len(),
        idx2.children_of("ghost").len()
    );
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
}

// ---------------------------------------------------------------------------
// BOUNDARIES
// ---------------------------------------------------------------------------

#[test]
fn no_mcp_api_database_network() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/index/mod.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["mcp", "database", "network", "server", "api"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
}

#[test]
fn no_package_zip_tar() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/index/mod.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["zip", "tar", "package", "archive"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
}

#[test]
fn no_graph_cache_watch() {
    let dir = format!("{}/src/core/index", env!("CARGO_MANIFEST_DIR"));
    let mut src = String::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            src.push_str(&std::fs::read_to_string(&path).unwrap());
            src.push('\n');
        }
    }
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["petgraph", "cache", "watch"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
    // Ensure BTreeMap used somewhere in index module
    assert!(src.contains("BTreeMap"), "must use BTreeMap");
}

#[test]
fn reuses_workspace_validation_model() {
    // Index is now split across mod.rs / build.rs / query.rs / integrity.rs / error.rs
    // Collect all files in src/index/ and check combined content.
    let dir = format!("{}/src/core/index", env!("CARGO_MANIFEST_DIR"));
    let mut src = String::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            src.push_str(&std::fs::read_to_string(&path).unwrap());
            src.push('\n');
        }
    }
    assert!(src.contains("workspace::read"), "must use workspace::read");
    assert!(src.contains("BTreeMap"), "must use BTreeMap");
    assert!(src.contains("Level"), "must use Level");
}

#[test]
fn deterministic_ordering() {
    let dir = temp_dir();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&dir, &e).unwrap();
    }
    let idx = Index::build(&dir).unwrap();
    assert_eq!(idx.ids(), vec!["a", "m", "z"]);
    // validate sorted by (id,field)
    let dir2 = temp_dir();
    let n1 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "ghost1")],
    );
    let n2 = make_entity(
        Level::Node,
        vec![("id", "n2"), ("title", "N")],
        vec![("parent", "ghost2")],
    );
    aimt::crud::create(&dir2, &n1).unwrap();
    aimt::crud::create(&dir2, &n2).unwrap();
    let idx2 = Index::build(&dir2).unwrap();
    let errs = idx2.validate().unwrap_err();
    assert_eq!(errs[0].id, "n1");
    assert_eq!(errs[1].id, "n2");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn no_filename_id_inference() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "actual"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let d = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::writer::write(&dir.join("weird.pmap"), &e).unwrap();
    aimt::writer::write(&dir.join("d1.pmap"), &d).unwrap();
    let idx = Index::build(&dir).unwrap();
    assert!(idx.get("actual").is_some());
    assert!(idx.get("weird").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_reference_mutation() {
    let dir = temp_dir();
    let n = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "ghost")],
    );
    aimt::crud::create(&dir, &n).unwrap();
    let idx = Index::build(&dir).unwrap();
    let errs = idx.validate().unwrap_err();
    assert_eq!(errs.len(), 1);
    // Ensure original entity still has ghost parent, not fixed
    assert_eq!(
        idx.get("n1").unwrap().field("parent").unwrap().value,
        "ghost"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
