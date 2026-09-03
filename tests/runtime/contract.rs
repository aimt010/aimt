use aimt::model::{AimtEntity, Field, Level};
use aimt::runtime::{Runtime, RuntimeError};
use aimt::syntax::Span;
use aimt::validation::ValidationErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_runtime_test_{}_{}_{}",
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

// ---------------------------------------------------------------------------
// LOAD
// ---------------------------------------------------------------------------

#[test]
fn load_empty_workspace() {
    let dir = temp_dir();
    let rt = Runtime::load(&dir).unwrap();
    assert!(rt.is_empty());
    assert_eq!(rt.len(), 0);
    assert_eq!(rt.workspace(), dir.as_path());
    assert!(rt.list().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_one_valid_entity() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let rt = Runtime::load(&dir).unwrap();
    assert_eq!(rt.len(), 1);
    let got = rt.get("a1").unwrap();
    assert_eq!(got.id().unwrap().as_str(), "a1");
    assert_eq!(got.level, Level::Aimt);
    assert!(rt.contains("a1"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_multiple_sorted() {
    let dir = temp_dir();
    let z = make_entity(
        Level::Node,
        vec![("id", "z_id"), ("title", "Z")],
        vec![("parent", "d1")],
    );
    let a = make_entity(
        Level::Node,
        vec![("id", "a_id"), ("title", "A")],
        vec![("parent", "d1")],
    );
    let m = make_entity(
        Level::Node,
        vec![("id", "m_id"), ("title", "M")],
        vec![("parent", "d1")],
    );
    // Create in reverse order to test determinism
    aimt::crud::create(&dir, &z).unwrap();
    aimt::crud::create(&dir, &a).unwrap();
    aimt::crud::create(&dir, &m).unwrap();
    let rt = Runtime::load(&dir).unwrap();
    let ids: Vec<_> = rt
        .list()
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert_eq!(ids, vec!["a_id", "m_id", "z_id"]);
    let paths: Vec<_> = rt
        .list_entities()
        .iter()
        .map(|we| we.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(paths, vec!["a_id.pmap", "m_id.pmap", "z_id.pmap"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_missing_workspace() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_runtime_missing_{}_{}",
        std::process::id(),
        99991
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let err = Runtime::load(&missing).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
    assert!(matches!(err, RuntimeError::Io(_)));
}

#[test]
fn load_file_as_workspace() {
    let dir = temp_dir();
    let file = dir.join("not_a_dir");
    std::fs::write(&file, "hello").unwrap();
    let err = Runtime::load(&file).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_with_invalid_file() {
    let dir = temp_dir();
    let good = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &good).unwrap();
    std::fs::write(dir.join("bad.pmap"), "not a pmap").unwrap();
    let err = Runtime::load(&dir).unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    assert!(matches!(err, RuntimeError::Read(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_with_duplicate_id() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::writer::write(&dir.join("a_dup.pmap"), &e).unwrap();
    aimt::writer::write(&dir.join("z_dup.pmap"), &e).unwrap();
    let err = Runtime::load(&dir).unwrap_err();
    assert_eq!(err.kind_str(), "DuplicateId");
    if let RuntimeError::DuplicateId(d) = err {
        assert_eq!(d.id, "dup");
        assert_eq!(d.paths.len(), 2);
        assert!(d.paths[0] < d.paths[1]);
        assert_eq!(d.paths[0].file_name().unwrap(), "a_dup.pmap");
    } else {
        panic!("expected DuplicateId");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// QUERY
// ---------------------------------------------------------------------------

#[test]
fn get_existing() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let rt = Runtime::load(&dir).unwrap();
    let got = rt.get("n1").unwrap();
    assert_eq!(got.id().unwrap().as_str(), "n1");
    assert_eq!(got.level, Level::Node);
    let we = rt.get_entity("n1").unwrap();
    assert_eq!(we.path, dir.join("n1.pmap"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_not_found() {
    let dir = temp_dir();
    let rt = Runtime::load(&dir).unwrap();
    assert!(rt.get("ghost").is_none());
    assert!(rt.get_entity("ghost").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_empty() {
    let dir = temp_dir();
    let rt = Runtime::load(&dir).unwrap();
    assert!(rt.list().is_empty());
    assert!(rt.list_entities().is_empty());
    assert!(rt.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_after_load() {
    let dir = temp_dir();
    for id in ["a", "b", "c"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&dir, &e).unwrap();
    }
    let rt = Runtime::load(&dir).unwrap();
    assert_eq!(rt.len(), 3);
    assert_eq!(rt.list().len(), 3);
    let ids: Vec<_> = rt
        .list()
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn contains() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let rt = Runtime::load(&dir).unwrap();
    assert!(rt.contains("a1"));
    assert!(!rt.contains("ghost"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_ignores_non_pmap_hidden() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    std::fs::write(dir.join("README.md"), "hi").unwrap();
    std::fs::write(dir.join(".hidden.pmap"), "bad").unwrap();
    let rt = Runtime::load(&dir).unwrap();
    assert_eq!(rt.len(), 1);
    assert!(rt.contains("a1"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// INSERT (memory only)
// ---------------------------------------------------------------------------

#[test]
fn insert_valid() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    rt.insert(e.clone()).unwrap();
    assert!(rt.contains("a1"));
    assert_eq!(rt.len(), 1);
    // No file until persist
    assert!(!dir.join("a1.pmap").exists());
    rt.persist().unwrap();
    assert!(dir.join("a1.pmap").exists());
    let back = aimt::reader::read(&dir.join("a1.pmap")).unwrap();
    assert_eq!(back.id().unwrap().as_str(), "a1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn insert_duplicate() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "dup"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(e.clone()).unwrap();
    let err = rt.insert(e).unwrap_err();
    assert_eq!(err.kind_str(), "DuplicateId");
    assert_eq!(rt.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn insert_validates() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let bad = make_entity(Level::Aimt, vec![("id", "a1")], vec![("title", "T")]); // missing version
    let err = rt.insert(bad).unwrap_err();
    assert_eq!(err.kind_str(), "Validation");
    if let RuntimeError::Validation(vec) = err {
        assert!(
            vec.iter()
                .any(|e| e.kind == ValidationErrorKind::MissingRequiredField)
        );
    }
    assert!(rt.is_empty());
    assert!(!dir.join("a1.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn insert_invalid_id_shape() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let bad = make_entity(
        Level::Node,
        vec![("id", "Has Space"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let err = rt.insert(bad).unwrap_err();
    assert_eq!(err.kind_str(), "Validation");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// UPDATE (memory only)
// ---------------------------------------------------------------------------

#[test]
fn update_existing() {
    let dir = temp_dir();
    let orig = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Orig")],
        vec![("parent", "d1")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(orig).unwrap();
    rt.persist().unwrap();
    let updated = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Updated")],
        vec![("parent", "d1")],
    );
    rt.update(updated).unwrap();
    assert_eq!(
        rt.get("n1").unwrap().field("title").unwrap().value,
        "Updated"
    );
    // File not yet changed until persist
    let before = std::fs::read_to_string(dir.join("n1.pmap")).unwrap();
    assert!(before.contains("Orig"));
    rt.persist().unwrap();
    let after = std::fs::read_to_string(dir.join("n1.pmap")).unwrap();
    assert!(after.contains("Updated"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_not_found() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let e = make_entity(
        Level::Node,
        vec![("id", "ghost"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let err = rt.update(e).unwrap_err();
    assert_eq!(err.kind_str(), "NotFound");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_validates() {
    let dir = temp_dir();
    let good = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/main.rs")],
        vec![("hash", "abc")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(good).unwrap();
    rt.persist().unwrap();
    let before = std::fs::read(dir.join("file_001.pmap")).unwrap();
    let bad = make_entity(Level::File, vec![("id", "file_001")], vec![("hash", "abc")]); // missing path
    let err = rt.update(bad).unwrap_err();
    assert_eq!(err.kind_str(), "Validation");
    let after = std::fs::read(dir.join("file_001.pmap")).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        rt.get("file_001").unwrap().field("path").unwrap().value,
        "src/main.rs"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// REMOVE (memory only)
// ---------------------------------------------------------------------------

#[test]
fn remove_existing() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(e).unwrap();
    rt.persist().unwrap();
    assert!(dir.join("n1.pmap").exists());
    let removed = rt.remove("n1").unwrap();
    assert_eq!(removed.id().unwrap().as_str(), "n1");
    assert!(!rt.contains("n1"));
    assert_eq!(rt.len(), 0);
    // File still exists until persist
    assert!(dir.join("n1.pmap").exists());
    rt.persist().unwrap();
    assert!(!dir.join("n1.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remove_not_found() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let err = rt.remove("ghost").unwrap_err();
    assert_eq!(err.kind_str(), "NotFound");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remove_then_insert_same_id() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(e.clone()).unwrap();
    rt.persist().unwrap();
    rt.remove("n1").unwrap();
    // Re-insert same id should succeed (tombstone allows reuse)
    rt.insert(e).unwrap();
    assert!(rt.contains("n1"));
    rt.persist().unwrap();
    assert!(dir.join("n1.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// PERSIST
// ---------------------------------------------------------------------------

#[test]
fn persist_after_insert() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let e = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    rt.insert(e).unwrap();
    rt.persist().unwrap();
    assert!(dir.join("d1.pmap").exists());
    let v = aimt::workspace::discover(&dir).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].file_name().unwrap(), "d1.pmap");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_after_update() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut rt = Runtime::load(&dir).unwrap();
    let upd = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "New")],
    );
    rt.update(upd).unwrap();
    rt.persist().unwrap();
    let back = aimt::reader::read(&dir.join("a1.pmap")).unwrap();
    assert_eq!(back.field("title").unwrap().value, "New");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_after_remove() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut rt = Runtime::load(&dir).unwrap();
    rt.remove("n1").unwrap();
    rt.persist().unwrap();
    assert!(!dir.join("n1.pmap").exists());
    assert!(aimt::workspace::discover(&dir).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_idempotent() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(e).unwrap();
    rt.persist().unwrap();
    let first = std::fs::read(dir.join("a1.pmap")).unwrap();
    rt.persist().unwrap();
    let second = std::fs::read(dir.join("a1.pmap")).unwrap();
    assert_eq!(first, second);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_does_not_create_workspace_dir() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    rt.insert(e).unwrap();
    // Remove workspace dir externally before persist
    std::fs::remove_dir_all(&dir).unwrap();
    let err = rt.persist().unwrap_err();
    assert!(matches!(err, RuntimeError::Write(_) | RuntimeError::Io(_)));
    assert!(!dir.exists());
}

#[test]
fn persist_writes_lexical_order() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        rt.insert(e).unwrap();
    }
    rt.persist().unwrap();
    let v = aimt::workspace::discover(&dir).unwrap();
    let names: Vec<_> = v
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["a.pmap", "m.pmap", "z.pmap"]);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// RELOAD
// ---------------------------------------------------------------------------

#[test]
fn reload_after_external_file_added() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    assert!(rt.is_empty());
    let e = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    rt.reload().unwrap();
    assert_eq!(rt.len(), 1);
    assert!(rt.contains("d1"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reload_after_external_invalid_file() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut rt = Runtime::load(&dir).unwrap();
    assert_eq!(rt.len(), 1);
    std::fs::write(dir.join("bad.pmap"), "bad content").unwrap();
    let err = rt.reload().unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    // Old state retained
    assert_eq!(rt.len(), 1);
    assert!(rt.contains("a1"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reload_clears_removed() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut rt = Runtime::load(&dir).unwrap();
    rt.remove("n1").unwrap();
    assert!(!rt.contains("n1"));
    // Without persist, reload should bring it back
    rt.reload().unwrap();
    assert!(rt.contains("n1"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// BOUNDARIES
// ---------------------------------------------------------------------------

#[test]
fn no_open_close_read_write_session() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/runtime.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    // Step 9 lifecycle names must not appear as method names `fn open`, `fn close`, `fn read`, `fn write`
    // Check for `fn open(` etc. Step 8 uses load/get/list/insert/update/remove/persist/reload
    assert!(!src.contains("fn open"), "must not have fn open");
    assert!(!src.contains("fn close"), "must not have fn close");
    // Ensure we don't have fn read / fn write as session API (writer::write is allowed as qualified)
    // Check for `fn read(` at line start
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub fn read(") || trimmed.starts_with("fn read(") {
            panic!("runtime must not expose fn read as session, got: {}", line);
        }
        if trimmed.starts_with("pub fn write(") || trimmed.starts_with("fn write(") {
            panic!("runtime must not expose fn write as session, got: {}", line);
        }
    }
}

#[test]
fn no_package_zip_tar() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/runtime.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["zip", "tar", "package", "archive", "mcp"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
}

#[test]
fn reuses_workspace_reader_writer_validation() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/runtime.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(
        src.contains("workspace::read") || src.contains("workspace::discover"),
        "must use workspace"
    );
    assert!(
        src.contains("writer::write"),
        "must use writer::write for persist"
    );
    assert!(
        src.contains("validation::validate") || src.contains("validate"),
        "must use validation"
    );
    assert!(
        src.contains("BTreeMap"),
        "must use BTreeMap for deterministic ordering"
    );
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
    let rt1 = Runtime::load(&dir).unwrap();
    let rt2 = Runtime::load(&dir).unwrap();
    let ids1: Vec<_> = rt1
        .list()
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    let ids2: Vec<_> = rt2
        .list()
        .iter()
        .map(|e| e.id().unwrap().as_str().to_string())
        .collect();
    assert_eq!(ids1, ids2);
    assert_eq!(ids1, vec!["a", "m", "z"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_reference_resolution() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "does_not_exist")],
    );
    let mut rt = Runtime::load(&dir).unwrap();
    rt.insert(e).unwrap();
    rt.persist().unwrap();
    let rt2 = Runtime::load(&dir).unwrap();
    assert!(rt2.contains("n1"));
    assert_eq!(
        rt2.get("n1").unwrap().parent_ref().unwrap().as_str(),
        "does_not_exist"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn filename_policy_via_persist() {
    let dir = temp_dir();
    let mut rt = Runtime::load(&dir).unwrap();
    let e = make_entity(
        Level::Node,
        vec![("id", "my_id"), ("title", "N")],
        vec![("parent", "d1")],
    );
    rt.insert(e).unwrap();
    rt.persist().unwrap();
    assert!(dir.join("my_id.pmap").exists());
    assert_eq!(dir.join("my_id.pmap").file_stem().unwrap(), "my_id");
    let _ = std::fs::remove_dir_all(&dir);
}
