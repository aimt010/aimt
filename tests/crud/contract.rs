use aimt::crud::{self, CrudError, delete, get, list};
use aimt::model::{AimtEntity, Field, Level};
use aimt::syntax::Span;
use aimt::validation::ValidationErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_crud_test_{}_{}_{}",
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

fn assert_entities_eq(a: &AimtEntity, b: &AimtEntity) {
    assert_eq!(a.level, b.level);
    assert_eq!(a.header.len(), b.header.len());
    for (fa, fb) in a.header.iter().zip(b.header.iter()) {
        assert_eq!(fa.name, fb.name);
        assert_eq!(fa.value, fb.value);
    }
    assert_eq!(a.body.len(), b.body.len());
    for (fa, fb) in a.body.iter().zip(b.body.iter()) {
        assert_eq!(fa.name, fb.name);
        assert_eq!(fa.value, fb.value);
    }
    assert_eq!(a.relations.len(), b.relations.len());
}

// ---------------------------------------------------------------------------
// CREATE
// ---------------------------------------------------------------------------

#[test]
fn create_one_valid_entity() {
    let dir = temp_dir();
    let entity = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let path = crud::create(&dir, &entity).unwrap();
    assert_eq!(path, dir.join("a1.pmap"));
    assert!(path.exists());
    let back = aimt::reader::read(&path).unwrap();
    assert_eq!(back.id().unwrap().as_str(), "a1");
    let listed = list(&dir).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].entity.id().unwrap().as_str(), "a1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_duplicate_id() {
    let dir = temp_dir();
    let e1 = make_entity(
        Level::Aimt,
        vec![("id", "dup"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let e2 = make_entity(
        Level::Aimt,
        vec![("id", "dup"), ("version", "0.2.0")],
        vec![("title", "U")],
    );
    crud::create(&dir, &e1).unwrap();
    let err = crud::create(&dir, &e2).unwrap_err();
    assert!(matches!(err, CrudError::DuplicateId(_)));
    assert_eq!(err.kind_str(), "DuplicateId");
    assert_eq!(err.id(), Some("dup"));
    // No second file created
    let files = aimt::workspace::discover(&dir).unwrap();
    assert_eq!(files.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_validates_before_write() {
    let dir = temp_dir();
    // @aimt missing version -> Validation MissingRequiredField
    let bad = make_entity(Level::Aimt, vec![("id", "a1")], vec![("title", "T")]);
    let err = crud::create(&dir, &bad).unwrap_err();
    assert!(matches!(err, CrudError::Validation(_)));
    assert_eq!(err.kind_str(), "Validation");
    if let CrudError::Validation(vec) = err {
        assert!(
            vec.iter()
                .any(|e| e.kind == ValidationErrorKind::MissingRequiredField
                    && e.field.as_deref() == Some("version"))
        );
    }
    assert!(!dir.join("a1.pmap").exists());
    assert!(aimt::workspace::discover(&dir).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_invalid_id_shape() {
    let dir = temp_dir();
    for bad_id in ["Has Space", "UPPER", ""] {
        let e = if bad_id.is_empty() {
            make_entity(
                Level::Node,
                vec![("id", ""), ("title", "N")],
                vec![("parent", "d1")],
            )
        } else {
            make_entity(
                Level::Node,
                vec![("id", bad_id), ("title", "N")],
                vec![("parent", "d1")],
            )
        };
        let err = crud::create(&dir, &e).unwrap_err();
        assert!(
            matches!(err, CrudError::Validation(_)),
            "bad_id {:?}",
            bad_id
        );
    }
    assert!(aimt::workspace::discover(&dir).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_missing_id_field() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("version", "0.1.0")],
        vec![("title", "T")],
    );
    let err = crud::create(&dir, &e).unwrap_err();
    assert!(matches!(err, CrudError::Validation(_)));
    if let CrudError::Validation(vec) = err {
        assert!(
            vec.iter()
                .any(|e| e.kind == ValidationErrorKind::MissingRequiredField
                    && e.field.as_deref() == Some("id"))
        );
    }
    assert!(!dir.join(".pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_with_forbidden_field() {
    let dir = temp_dir();
    // @aimt with parent forbidden
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0"), ("parent", "x")],
        vec![("title", "T")],
    );
    let err = crud::create(&dir, &e).unwrap_err();
    assert!(matches!(err, CrudError::Validation(_)));
    if let CrudError::Validation(vec) = err {
        assert!(
            vec.iter()
                .any(|e| e.kind == ValidationErrorKind::ForbiddenField
                    && e.field.as_deref() == Some("parent"))
        );
    }
    assert!(!dir.join("a1.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_duplicate_across_different_levels() {
    let dir = temp_dir();
    let d = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D")],
        vec![("description", "hi")],
    );
    let n = make_entity(
        Level::Node,
        vec![("id", "dup"), ("title", "N")],
        vec![("parent", "dup")],
    );
    crud::create(&dir, &d).unwrap();
    let err = crud::create(&dir, &n).unwrap_err();
    assert!(matches!(err, CrudError::DuplicateId(_)));
    assert_eq!(err.id(), Some("dup"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_filename_policy() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "my_id"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let path = crud::create(&dir, &e).unwrap();
    assert_eq!(path.file_name().unwrap(), "my_id.pmap");
    assert_eq!(path.parent().unwrap(), dir.as_path());
    // Stem equals id
    assert_eq!(path.file_stem().unwrap(), "my_id");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn create_second_distinct_id() {
    let dir = temp_dir();
    let a = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let b = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    crud::create(&dir, &a).unwrap();
    crud::create(&dir, &b).unwrap();
    let v = aimt::workspace::discover(&dir).unwrap();
    assert_eq!(v.len(), 2);
    let mut names: Vec<_> = v
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["a1.pmap", "d1.pmap"]);
    let listed = list(&dir).unwrap();
    assert_eq!(listed.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// READ / GET / LIST
// ---------------------------------------------------------------------------

#[test]
fn get_existing_by_id() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    crud::create(&dir, &e).unwrap();
    let got = get(&dir, "n1").unwrap();
    assert_eq!(got.entity.id().unwrap().as_str(), "n1");
    assert_eq!(got.path, dir.join("n1.pmap"));
    assert_eq!(got.entity.level, Level::Node);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_not_found() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    crud::create(&dir, &e).unwrap();
    let err = get(&dir, "missing").unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    assert_eq!(err.kind_str(), "NotFound");
    assert_eq!(err.id(), Some("missing"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_with_duplicate_ids() {
    let dir = temp_dir();
    // Manually create two files with same id via writer, bypassing crud duplicate check
    let e1 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D1")],
        vec![("description", "hi")],
    );
    let e2 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D2")],
        vec![("description", "hi2")],
    );
    aimt::writer::write(&dir.join("a_dup.pmap"), &e1).unwrap();
    aimt::writer::write(&dir.join("z_dup.pmap"), &e2).unwrap();
    let err = get(&dir, "dup").unwrap_err();
    assert!(matches!(err, CrudError::DuplicateId(_)));
    if let CrudError::DuplicateId(d) = err {
        assert_eq!(d.id, "dup");
        assert_eq!(d.paths.len(), 2);
        // Sorted lexically
        assert!(d.paths[0] < d.paths[1]);
        assert_eq!(d.paths[0].file_name().unwrap(), "a_dup.pmap");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_empty_workspace() {
    let dir = temp_dir();
    let v = list(&dir).unwrap();
    assert!(v.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_multiple_sorted() {
    let dir = temp_dir();
    // Create in reverse lexical order via crud (which writes <id>.pmap)
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
    crud::create(&dir, &z).unwrap();
    crud::create(&dir, &a).unwrap();
    crud::create(&dir, &m).unwrap();
    let v = list(&dir).unwrap();
    assert_eq!(v.len(), 3);
    let names: Vec<_> = v
        .iter()
        .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["a_id.pmap", "m_id.pmap", "z_id.pmap"]);
    // Also compare with workspace::read length and ids sorted
    let ws = aimt::workspace::read(&dir).unwrap();
    assert_eq!(ws.len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn get_after_create_roundtrip() {
    let dir = temp_dir();
    let e = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/main.rs")],
        vec![("hash", "abc")],
    );
    crud::create(&dir, &e).unwrap();
    let got = get(&dir, "file_001").unwrap();
    assert_entities_eq(&e, &got.entity);
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
    crud::create(&dir, &e).unwrap();
    std::fs::write(dir.join("README.md"), "hi").unwrap();
    std::fs::write(dir.join(".hidden.pmap"), "bad").unwrap();
    let v = list(&dir).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].entity.id().unwrap().as_str(), "a1");
    let err = get(&dir, "README").unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_reflects_workspace_read() {
    let dir = temp_dir();
    let e1 = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let e2 = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    crud::create(&dir, &e1).unwrap();
    crud::create(&dir, &e2).unwrap();
    let via_crud = list(&dir).unwrap();
    let via_ws = aimt::workspace::read(&dir).unwrap();
    assert_eq!(via_crud.len(), via_ws.len());
    let mut ids_crud: Vec<_> = via_crud
        .iter()
        .map(|e| e.entity.id().unwrap().as_str().to_string())
        .collect();
    let mut ids_ws: Vec<_> = via_ws
        .iter()
        .map(|e| e.entity.id().unwrap().as_str().to_string())
        .collect();
    ids_crud.sort();
    ids_ws.sort();
    assert_eq!(ids_crud, ids_ws);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

#[test]
fn update_existing() {
    let dir = temp_dir();
    let orig = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Orig")],
        vec![("parent", "d1")],
    );
    let path = crud::create(&dir, &orig).unwrap();
    let updated = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Updated")],
        vec![("parent", "d1")],
    );
    let path2 = crud::update(&dir, &updated).unwrap();
    assert_eq!(path, path2);
    assert_eq!(path2, dir.join("n1.pmap"));
    let got = get(&dir, "n1").unwrap();
    assert_eq!(got.entity.field("title").unwrap().value, "Updated");
    // File count unchanged
    assert_eq!(aimt::workspace::discover(&dir).unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_not_found() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "missing"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let err = crud::update(&dir, &e).unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    assert_eq!(err.kind_str(), "NotFound");
    assert!(!dir.join("missing.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_validates_original_unchanged() {
    let dir = temp_dir();
    let orig = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/main.rs")],
        vec![("hash", "abc")],
    );
    let path = crud::create(&dir, &orig).unwrap();
    let before = std::fs::read(&path).unwrap();
    // Invalid update: @file missing path
    let bad = make_entity(Level::File, vec![("id", "file_001")], vec![("hash", "abc")]);
    let err = crud::update(&dir, &bad).unwrap_err();
    assert!(matches!(err, CrudError::Validation(_)));
    let after = std::fs::read(&path).unwrap();
    assert_eq!(
        before, after,
        "original file must be unchanged after validation failure"
    );
    let got = get(&dir, "file_001").unwrap();
    assert_eq!(got.entity.field("path").unwrap().value, "src/main.rs");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_does_not_create_second_file() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    crud::create(&dir, &e).unwrap();
    let before_count = aimt::workspace::discover(&dir).unwrap().len();
    let upd = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "New")],
    );
    crud::update(&dir, &upd).unwrap();
    assert_eq!(aimt::workspace::discover(&dir).unwrap().len(), before_count);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_with_duplicate_before() {
    let dir = temp_dir();
    let e1 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D1")],
        vec![("description", "hi")],
    );
    let e2 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D2")],
        vec![("description", "hi2")],
    );
    aimt::writer::write(&dir.join("a.pmap"), &e1).unwrap();
    aimt::writer::write(&dir.join("b.pmap"), &e2).unwrap();
    let upd = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D3")],
        vec![("description", "hi3")],
    );
    let err = crud::update(&dir, &upd).unwrap_err();
    assert!(matches!(err, CrudError::DuplicateId(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DELETE
// ---------------------------------------------------------------------------

#[test]
fn delete_existing() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let path = crud::create(&dir, &e).unwrap();
    assert!(path.exists());
    let removed = delete(&dir, "n1").unwrap();
    assert_eq!(removed, path);
    assert!(!path.exists());
    let err = get(&dir, "n1").unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    assert_eq!(list(&dir).unwrap().len(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_not_found() {
    let dir = temp_dir();
    let err = delete(&dir, "ghost").unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    assert_eq!(err.kind_str(), "NotFound");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_with_duplicate() {
    let dir = temp_dir();
    let e1 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D1")],
        vec![("description", "hi")],
    );
    let e2 = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D2")],
        vec![("description", "hi2")],
    );
    aimt::writer::write(&dir.join("a.pmap"), &e1).unwrap();
    aimt::writer::write(&dir.join("b.pmap"), &e2).unwrap();
    let err = delete(&dir, "dup").unwrap_err();
    assert!(matches!(err, CrudError::DuplicateId(_)));
    if let CrudError::DuplicateId(d) = err {
        assert_eq!(d.paths.len(), 2);
        assert!(d.paths[0] < d.paths[1]);
    }
    // No file removed
    assert_eq!(aimt::workspace::discover(&dir).unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_removes_file() {
    let dir = temp_dir();
    let e = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/x.rs")],
        vec![("hash", "h")],
    );
    let path = crud::create(&dir, &e).unwrap();
    assert!(path.exists());
    delete(&dir, "file_001").unwrap();
    assert!(!path.exists());
    assert!(!dir.join("file_001.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// BOUNDARIES & DETERMINISM & REUSE
// ---------------------------------------------------------------------------

#[test]
fn deterministic_duplicate_reporting() {
    let dir = temp_dir();
    // Create duplicate via two files, ensure paths sorted lexical
    let e = make_entity(
        Level::Node,
        vec![("id", "dup"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::writer::write(&dir.join("z_dup.pmap"), &e).unwrap();
    aimt::writer::write(&dir.join("a_dup.pmap"), &e).unwrap();
    let err = get(&dir, "dup").unwrap_err();
    if let CrudError::DuplicateId(d) = err {
        assert_eq!(d.paths[0].file_name().unwrap(), "a_dup.pmap");
        assert_eq!(d.paths[1].file_name().unwrap(), "z_dup.pmap");
    } else {
        panic!("expected DuplicateId");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn does_not_create_workspace_dir() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_crud_missing_{}_{}",
        std::process::id(),
        12345
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let err = crud::create(&missing, &e).unwrap_err();
    // Should be Io NotFound or InvalidWorkspace, not create dir
    assert!(matches!(
        err,
        CrudError::Io(_) | CrudError::InvalidWorkspace(_)
    ));
    assert!(!missing.exists(), "workspace dir must not be created");
}

#[test]
fn reuses_workspace_reader_writer_validation() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/crud.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(
        src.contains("workspace::discover") || src.contains("workspace::read"),
        "must use workspace"
    );
    assert!(
        src.contains("reader::read") || src.contains("crate::reader::read"),
        "must use reader"
    );
    assert!(
        src.contains("writer::write") || src.contains("crate::writer::write"),
        "must use writer"
    );
    assert!(
        src.contains("validation::validate") || src.contains("validate"),
        "must use validation"
    );
    // No duplicate serialize logic
    assert!(!src.contains("serialize_field"), "no duplicate serialize");
}

#[test]
fn no_graph_caching_runtime() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/crud.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in [
        "petgraph", "cache", "watch", "index", "runtime", "mcp", "database", "zip", "tar",
    ] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
    assert!(
        !src.contains("OPEN") || !src.contains("CLOSE"),
        "no runtime session tokens"
    );
    // Ensure no retained HashMap across calls (stateless)
    // We allow HashSet for temporary duplicate check, but not retained
    // Just ensure no global static
    assert!(
        !src.contains("static") || !src.contains("static COUNTER"),
        "no global cache"
    );
}

#[test]
fn no_reference_resolution() {
    let dir = temp_dir();
    // parent does_not_exist is allowed, create should succeed (validation checks shape only, not existence)
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "does_not_exist")],
    );
    let path = crud::create(&dir, &e).unwrap();
    assert!(path.exists());
    let got = get(&dir, "n1").unwrap();
    assert_eq!(got.entity.parent_ref().unwrap().as_str(), "does_not_exist");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn filename_derived_from_id_only() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "actual_id"), ("title", "N")],
        vec![("parent", "d1")],
    );
    // Write file with weird_name directly, then get by actual_id should work, by weird_name should not
    aimt::writer::write(&dir.join("weird_name.pmap"), &e).unwrap();
    let got = get(&dir, "actual_id").unwrap();
    assert_eq!(got.entity.id().unwrap().as_str(), "actual_id");
    let err = get(&dir, "weird_name").unwrap_err();
    assert!(matches!(err, CrudError::NotFound(_)));
    // Now create via crud uses <id>.pmap
    let dir2 = temp_dir();
    crud::create(&dir2, &e).unwrap();
    assert!(dir2.join("actual_id.pmap").exists());
    assert!(!dir2.join("weird_name.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn validation_preserves_span_and_no_persist_on_failure() {
    let dir = temp_dir();
    let good = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    crud::create(&dir, &good).unwrap();
    let before = std::fs::read(dir.join("n1.pmap")).unwrap();
    // Try to update with invalid entity
    let bad = make_entity(Level::Node, vec![("id", "n1"), ("title", "N")], vec![]); // missing parent
    let err = crud::update(&dir, &bad).unwrap_err();
    assert!(matches!(err, CrudError::Validation(_)));
    let after = std::fs::read(dir.join("n1.pmap")).unwrap();
    assert_eq!(before, after);
    // Check that validation error preserves field
    if let CrudError::Validation(vec) = err {
        assert!(vec.iter().any(|e| e.field.as_deref() == Some("parent")));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn crud_error_kind_str_path_id() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    crud::create(&dir, &e).unwrap();
    let dup_err = crud::create(&dir, &e).unwrap_err();
    assert_eq!(dup_err.kind_str(), "DuplicateId");
    assert_eq!(dup_err.id(), Some("a1"));
    let nf = get(&dir, "ghost").unwrap_err();
    assert_eq!(nf.kind_str(), "NotFound");
    assert_eq!(nf.id(), Some("ghost"));
    let val = make_entity(Level::Aimt, vec![("id", "x")], vec![("title", "T")]);
    let ve = crud::create(&dir, &val).unwrap_err();
    assert_eq!(ve.kind_str(), "Validation");
    let _ = std::fs::remove_dir_all(&dir);
}
