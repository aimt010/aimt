use aimt::model::{AimtEntity, Field, Level};
use aimt::reader::ReaderError;
use aimt::syntax::Span;
use aimt::workspace::{WorkspaceError, discover, read, write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_workspace_test_{}_{}_{}",
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

fn with_temp_workspace<F>(files: &[(&str, &str)], f: F)
where
    F: FnOnce(&Path),
{
    let dir = temp_dir();
    for (name, content) in files {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&p, content).unwrap();
    }
    f(&dir);
    let _ = std::fs::remove_dir_all(&dir);
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

const MINIMAL_AIMT: &str = "@aimt\n\n#header\n  id:\n    aimt_root\n  version:\n    0.1.0\n\n#body\n  title:\n    AIMT Root\n";
const MINIMAL_NODE: &str = "@node\n\n#header\n  id:\n    node_001\n  title:\n    Payment Service\n\n#body\n  parent:\n    domain_001\n  type:\n    service\n";
const MINIMAL_FILE: &str = "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/main.rs\n\n#body\n  hash:\n    abc123\n";
const MINIMAL_DOMAIN: &str = "@domain\n\n#header\n  id:\n    domain_001\n  title:\n    State\n\n#body\n  description:\n    hi\n";

// ---------------------------------------------------------------------------
// DISCOVERY
// ---------------------------------------------------------------------------

#[test]
fn discover_empty_directory() {
    let dir = temp_dir();
    let res = discover(&dir).unwrap();
    assert!(res.is_empty(), "empty dir should yield empty vec");
    let read_res = read(&dir).unwrap();
    assert!(read_res.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn discover_one_pmap() {
    with_temp_workspace(&[("entity.pmap", MINIMAL_AIMT)], |dir| {
        let v = discover(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].file_name().unwrap(), "entity.pmap");
    });
}

#[test]
fn discover_multiple_pmap_sorted() {
    // Create in reverse lexical order to ensure sorting
    with_temp_workspace(
        &[
            ("m_entity.pmap", MINIMAL_DOMAIN),
            ("b_entity.pmap", MINIMAL_FILE),
            ("a_entity.pmap", MINIMAL_NODE),
        ],
        |dir| {
            let v = discover(dir).unwrap();
            assert_eq!(v.len(), 3);
            let names: Vec<_> = v
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect();
            assert_eq!(
                names,
                vec!["a_entity.pmap", "b_entity.pmap", "m_entity.pmap"]
            );
            // Also ensure read returns same sorted order
            let entities = read(dir).unwrap();
            assert_eq!(entities.len(), 3);
            assert_eq!(entities[0].path.file_name().unwrap(), "a_entity.pmap");
            assert_eq!(entities[1].path.file_name().unwrap(), "b_entity.pmap");
            assert_eq!(entities[2].path.file_name().unwrap(), "m_entity.pmap");
        },
    );
}

#[test]
fn deterministic_ordering() {
    let files = [
        ("c.pmap", MINIMAL_AIMT),
        ("a.pmap", MINIMAL_NODE),
        ("b.pmap", MINIMAL_FILE),
    ];
    let files_rev = [
        ("b.pmap", MINIMAL_FILE),
        ("c.pmap", MINIMAL_AIMT),
        ("a.pmap", MINIMAL_NODE),
    ];
    let dir1 = temp_dir();
    let dir2 = temp_dir();
    for (name, content) in &files {
        std::fs::write(dir1.join(name), content).unwrap();
    }
    for (name, content) in &files_rev {
        std::fs::write(dir2.join(name), content).unwrap();
    }
    let v1 = discover(&dir1).unwrap();
    let v2 = discover(&dir2).unwrap();
    let n1: Vec<_> = v1
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect();
    let n2: Vec<_> = v2
        .iter()
        .map(|p| p.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(n1, n2, "discover must be deterministic sorted");
    let r1 = read(&dir1).unwrap();
    let r2 = read(&dir2).unwrap();
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(
            a.entity.id().unwrap().as_str(),
            b.entity.id().unwrap().as_str()
        );
    }
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn non_pmap_ignored() {
    with_temp_workspace(
        &[
            ("entity.pmap", MINIMAL_AIMT),
            ("README.md", "# readme"),
            ("random.txt", "hello"),
            (".gitkeep", ""),
            ("Cargo.toml", "[package]"),
        ],
        |dir| {
            let v = discover(dir).unwrap();
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].file_name().unwrap(), "entity.pmap");
            let r = read(dir).unwrap();
            assert_eq!(r.len(), 1);
        },
    );
}

#[test]
fn hidden_file_ignored() {
    with_temp_workspace(
        &[
            ("entity.pmap", MINIMAL_AIMT),
            (".hidden.pmap", MINIMAL_NODE),
            (".DS_Store", "foo"),
        ],
        |dir| {
            let v = discover(dir).unwrap();
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].file_name().unwrap(), "entity.pmap");
        },
    );
}

#[test]
fn hidden_directory_ignored() {
    with_temp_workspace(&[("entity.pmap", MINIMAL_AIMT)], |dir| {
        let hidden = dir.join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("inner.pmap"), MINIMAL_NODE).unwrap();
        let v = discover(dir).unwrap();
        assert_eq!(
            v.len(),
            1,
            "hidden dir file not discovered, also nested ignored"
        );
    });
}

#[test]
fn uppercase_extension_ignored() {
    with_temp_workspace(
        &[
            ("entity.pmap", MINIMAL_AIMT),
            ("entity.PMAP", MINIMAL_NODE),
            ("entity.Pmap", MINIMAL_NODE),
            ("ENTITY.PMAP", MINIMAL_NODE),
        ],
        |dir| {
            let v = discover(dir).unwrap();
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].file_name().unwrap(), "entity.pmap");
        },
    );
}

#[test]
fn nested_directory_ignored() {
    with_temp_workspace(&[("top.pmap", MINIMAL_AIMT)], |dir| {
        let sub = dir.join("subdir");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("nested.pmap"), MINIMAL_NODE).unwrap();
        std::fs::write(sub.join("deep.pmap"), MINIMAL_FILE).unwrap();
        let v = discover(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].file_name().unwrap(), "top.pmap");
        let r = read(dir).unwrap();
        assert_eq!(r.len(), 1);
    });
}

#[test]
fn directory_named_pmap_ignored() {
    with_temp_workspace(&[("entity.pmap", MINIMAL_AIMT)], |dir| {
        std::fs::create_dir_all(dir.join("foo.pmap")).unwrap();
        let v = discover(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].file_name().unwrap(), "entity.pmap");
        // Ensure file inside foo.pmap dir not discovered (non-recursive)
        std::fs::write(dir.join("foo.pmap").join("inner.pmap"), MINIMAL_NODE).unwrap();
        let v2 = discover(dir).unwrap();
        assert_eq!(v2.len(), 1);
    });
}

#[test]
fn symlink_ignored() {
    with_temp_workspace(&[("real.pmap", MINIMAL_AIMT)], |dir| {
        let target = dir.join("real.pmap");
        let link = dir.join("link.pmap");
        let link_dir = dir.join("link_dir");
        let real_sub = dir.join("real_sub");
        let _ = std::fs::create_dir_all(&real_sub);
        std::fs::write(real_sub.join("a.pmap"), MINIMAL_NODE).unwrap();

        // Try symlink file
        #[cfg(unix)]
        {
            let res = std::os::unix::fs::symlink(&target, &link);
            if res.is_err() {
                // Symlink not supported, skip
                return;
            }
            let res2 = std::os::unix::fs::symlink(&real_sub, &link_dir);
            if res2.is_err() {
                let _ = std::fs::remove_file(&link);
                return;
            }
            let v = discover(dir).unwrap();
            // Should only contain real.pmap, not link.pmap, not link_dir, not nested
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].file_name().unwrap(), "real.pmap");
            assert!(!v.iter().any(|p| p.file_name().unwrap() == "link.pmap"));
            assert!(!v.iter().any(|p| p.file_name().unwrap() == "link_dir"));
            let _ = std::fs::remove_file(&link);
            let _ = std::fs::remove_file(&link_dir);
        }
        #[cfg(not(unix))]
        {
            // On non-unix, skip
        }
    });
}

#[test]
fn missing_directory() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_missing_{}_{}",
        std::process::id(),
        9999999
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let err = discover(&missing).unwrap_err();
    assert!(matches!(err, WorkspaceError::Io(_)));
    assert_eq!(err.kind_str(), "Io");
    assert_eq!(err.path().unwrap(), missing.as_path());
    if let WorkspaceError::Io(e) = err {
        assert_eq!(e.kind, std::io::ErrorKind::NotFound);
    }
    let err2 = read(&missing).unwrap_err();
    assert!(matches!(err2, WorkspaceError::Io(_)));
}

#[test]
fn path_is_file() {
    let dir = temp_dir();
    let file = dir.join("not_a_dir");
    std::fs::write(&file, "hello").unwrap();
    let err = discover(&file).unwrap_err();
    assert!(matches!(err, WorkspaceError::InvalidWorkspace(_)));
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    assert_eq!(err.path().unwrap(), file.as_path());
    let err2 = read(&file).unwrap_err();
    assert!(matches!(err2, WorkspaceError::InvalidWorkspace(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// READING
// ---------------------------------------------------------------------------

#[test]
fn read_one_valid_entity() {
    with_temp_workspace(&[("a.pmap", MINIMAL_AIMT)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].path.file_name().unwrap(), "a.pmap");
        assert_eq!(v[0].entity.level, Level::Aimt);
        assert_eq!(v[0].entity.id().unwrap().as_str(), "aimt_root");
        assert_eq!(v[0].path, dir.join("a.pmap"));
    });
}

#[test]
fn read_multiple_valid_entities() {
    with_temp_workspace(
        &[
            ("a.pmap", MINIMAL_AIMT),
            ("b.pmap", MINIMAL_NODE),
            ("c.pmap", MINIMAL_FILE),
        ],
        |dir| {
            let v = read(dir).unwrap();
            assert_eq!(v.len(), 3);
            // Sorted order
            assert_eq!(v[0].path.file_name().unwrap(), "a.pmap");
            assert_eq!(v[1].path.file_name().unwrap(), "b.pmap");
            assert_eq!(v[2].path.file_name().unwrap(), "c.pmap");
            assert_eq!(v[0].entity.level, Level::Aimt);
            assert_eq!(v[1].entity.level, Level::Node);
            assert_eq!(v[2].entity.level, Level::File);
        },
    );
}

#[test]
fn read_invalid_syntax_in_one_file() {
    let invalid = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n"; // missing trailing newline? actually has trailing newline, need invalid like missing header
    let bad = "#header\n  id:\n    x\n#body\n  title:\n    x\n"; // missing level
    with_temp_workspace(&[("bad.pmap", bad), ("good.pmap", MINIMAL_NODE)], |dir| {
        let err = read(dir).unwrap_err();
        match err {
            WorkspaceError::FileErrors(vec) => {
                assert_eq!(vec.len(), 1);
                assert_eq!(vec[0].path.file_name().unwrap(), "bad.pmap");
                assert!(matches!(vec[0].error, ReaderError::Parse(_)));
            }
            other => panic!("expected FileErrors, got {:?}", other),
        }
        // Ensure discover still finds both files
        let d = discover(dir).unwrap();
        assert_eq!(d.len(), 2);
    });
    // Test with invalid file alone to ensure _invalid var not used
    let _ = invalid;
}

#[test]
fn read_validation_failure_in_one_file() {
    // @aimt with forbidden parent, or @region missing parent
    let bad = "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  parent:\n    x\n\n#body\n  title:\n    T\n";
    with_temp_workspace(&[("bad.pmap", bad), ("good.pmap", MINIMAL_AIMT)], |dir| {
        let err = read(dir).unwrap_err();
        match err {
            WorkspaceError::FileErrors(vec) => {
                assert_eq!(vec.len(), 1);
                assert_eq!(vec[0].path.file_name().unwrap(), "bad.pmap");
                assert!(matches!(vec[0].error, ReaderError::Validation(_)));
            }
            other => panic!("expected FileErrors, got {:?}", other),
        }
    });
}

#[test]
fn read_mixed_valid_and_invalid_sorted_errors() {
    let good = MINIMAL_NODE;
    let bad1 = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    \n"; // empty required parent -> validation error
    let bad2 = "not a pmap at all\n"; // parse error
    with_temp_workspace(
        &[
            ("a_good.pmap", good),
            ("b_bad1.pmap", bad1),
            ("c_bad2.pmap", bad2),
        ],
        |dir| {
            let err = read(dir).unwrap_err();
            match err {
                WorkspaceError::FileErrors(vec) => {
                    assert_eq!(vec.len(), 2);
                    // Sorted lexical: b_bad1 before c_bad2
                    assert_eq!(vec[0].path.file_name().unwrap(), "b_bad1.pmap");
                    assert_eq!(vec[1].path.file_name().unwrap(), "c_bad2.pmap");
                }
                other => panic!("expected FileErrors, got {:?}", other),
            }
            // Policy B: no partial Ok, all errors returned together
        },
    );
}

#[test]
fn source_path_preserved() {
    with_temp_workspace(&[("my.pmap", MINIMAL_AIMT)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v[0].path, dir.join("my.pmap"));
        // Ensure entity is from that path, not inferred
        assert_eq!(v[0].entity.id().unwrap().as_str(), "aimt_root");
    });
}

#[test]
fn deterministic_result_order() {
    let files = [
        ("z.pmap", MINIMAL_FILE),
        ("a.pmap", MINIMAL_AIMT),
        ("m.pmap", MINIMAL_NODE),
    ];
    let dir1 = temp_dir();
    let dir2 = temp_dir();
    for (n, c) in &files {
        std::fs::write(dir1.join(n), c).unwrap();
        std::fs::write(dir2.join(n), c).unwrap();
    }
    let r1 = read(&dir1).unwrap();
    let r2 = read(&dir2).unwrap();
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(
            a.path.file_name().unwrap(),
            b.path.file_name().unwrap(),
            "file names should match deterministically"
        );
        assert_eq!(a.entity.level, b.entity.level);
        assert_eq!(
            a.entity.id().unwrap().as_str(),
            b.entity.id().unwrap().as_str()
        );
    }
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn read_empty_workspace() {
    let dir = temp_dir();
    let v = discover(&dir).unwrap();
    assert!(v.is_empty());
    let r = read(&dir).unwrap();
    assert!(r.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// BOUNDARIES
// ---------------------------------------------------------------------------

#[test]
fn no_recursive_scan() {
    with_temp_workspace(&[("top.pmap", MINIMAL_AIMT)], |dir| {
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("inner.pmap"), MINIMAL_NODE).unwrap();
        let d = discover(dir).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].file_name().unwrap(), "top.pmap");
        // Ensure inner not read even though valid
        let r = read(dir).unwrap();
        assert_eq!(r.len(), 1);
    });
}

#[test]
fn no_scanning_outside_supplied_directory() {
    let dir = temp_dir();
    let outside = temp_dir();
    std::fs::write(outside.join("outside.pmap"), MINIMAL_AIMT).unwrap();
    std::fs::write(dir.join("inside.pmap"), MINIMAL_NODE).unwrap();
    let v = discover(&dir).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].file_name().unwrap(), "inside.pmap");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn no_filename_id_inference() {
    let content = "@node\n\n#header\n  id:\n    actual_id\n  title:\n    N\n\n#body\n  parent:\n    domain_001\n  type:\n    service\n";
    with_temp_workspace(&[("weird_name.pmap", content)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity.id().unwrap().as_str(), "actual_id");
        assert_ne!(v[0].entity.id().unwrap().as_str(), "weird_name");
    });
}

#[test]
fn no_reference_resolution() {
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n\n#body\n  parent:\n    does_not_exist\n  type:\n    service\n";
    with_temp_workspace(&[("a.pmap", content)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity.parent_ref().unwrap().as_str(), "does_not_exist");
    });
    let frame = "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n\n#body\n  file:\n    missing_file\n  target:\n    Foo\n  type:\n    class\n";
    with_temp_workspace(&[("f.pmap", frame)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v.len(), 1);
    });
}

#[test]
fn no_graph() {
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n\n#body\n  parent:\n    d1\n  type:\n    service\n  relations:\n    @relation\n      type:\n        depends_on\n      from:\n        unknown\n      to:\n        also_unknown\n";
    with_temp_workspace(&[("a.pmap", content)], |dir| {
        let v = read(dir).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].entity.relations.len(), 1);
    });
}

#[test]
fn no_indexing_cache_watch_runtime_package() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/storage/workspace.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    // Ensure no forbidden substrings that would imply step 7+ functionality
    // Tokenize to avoid false positives like "starts_with" containing "tar"
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["crud", "zip", "tar", "cache", "watch", "mcp", "database"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "src/core/storage/workspace.rs should not contain token '{}'",
            term
        );
    }
    // No walkdir/glob
    assert!(!src.contains("walkdir"), "no walkdir");
    assert!(!src.contains("glob"), "no glob");
    assert!(!src.contains("create_dir_all"), "no create_dir_all");
    // Must contain required primitives
    assert!(src.contains("read_dir"), "must use read_dir");
    assert!(
        src.contains("writer::write"),
        "must delegate to writer::write"
    );
    assert!(
        src.contains("reader::read") || src.contains("crate::reader::read"),
        "must use reader::read"
    );
}

#[test]
fn no_crud_api_exists() {
    // Ensure workspace module does not expose CRUD-like functions beyond discover/read/write
    // This is a compile-time check: if extra functions existed, they'd be callable, but we test absence via src inspection
    let src = std::fs::read_to_string(format!(
        "{}/src/core/storage/workspace.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(!src.contains("fn create"), "no create");
    assert!(!src.contains("fn update"), "no update");
    assert!(!src.contains("fn delete"), "no delete");
    assert!(!src.contains("fn find_by_id"), "no find_by_id");
    assert!(!src.contains("fn get_by_id"), "no get_by_id");
    assert!(!src.contains("fn resolve"), "no resolve");
    assert!(!src.contains("fn index"), "no index");
}

// ---------------------------------------------------------------------------
// WRITING
// ---------------------------------------------------------------------------

#[test]
fn write_explicit_absolute_file() {
    let dir = temp_dir();
    let abs = dir.join("out.pmap");
    let entity = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    write(&abs, &entity).unwrap();
    assert!(abs.exists());
    let back = aimt::reader::read(&abs).unwrap();
    assert_eq!(back.id().unwrap().as_str(), "a1");
    // Discover should find it if inside workspace dir
    let disc = discover(&dir).unwrap();
    assert_eq!(disc.len(), 1);
    assert_eq!(disc[0], abs);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_uses_writer_and_round_trip() {
    let dir = temp_dir();
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("type", "service")],
    );
    let abs = dir.join("node.pmap");
    write(&abs, &entity).unwrap();
    // Round-trip via workspace read
    let entities = read(&dir).unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].entity.id().unwrap().as_str(), "n1");
    // Also via direct reader
    let single = aimt::reader::read(&abs).unwrap();
    assert_eq!(single.id().unwrap().as_str(), "n1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_does_not_create_parent() {
    let dir = temp_dir();
    let nested = dir.join("nope/sub/out.pmap");
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let err = write(&nested, &entity).unwrap_err();
    assert!(matches!(err, WorkspaceError::Write(_)));
    assert_eq!(err.kind_str(), "Write");
    assert_eq!(err.path().unwrap(), nested.as_path());
    if let WorkspaceError::Write(e) = err {
        assert_eq!(e.error.kind_str(), "Io");
        // Underlying kind should be NotFound
        match e.error {
            aimt::writer::WriterError::Io(io) => assert_eq!(io.kind, std::io::ErrorKind::NotFound),
            _ => panic!("expected Io"),
        }
    }
    assert!(!dir.join("nope").exists(), "parent not created");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_rejects_relative() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    for rel in ["rel.pmap", "sub/rel.pmap", "../outside.pmap", ""] {
        let p = Path::new(rel);
        let err = write(p, &entity).unwrap_err();
        assert!(
            matches!(err, WorkspaceError::InvalidWorkspace(_)),
            "relative {:?} should be InvalidWorkspace",
            rel
        );
        assert_eq!(err.kind_str(), "InvalidWorkspace");
        assert_eq!(err.path().unwrap(), p);
    }
}

#[test]
fn write_overwrites() {
    let dir = temp_dir();
    let abs = dir.join("file.pmap");
    let e1 = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "First")],
    );
    let e2 = make_entity(
        Level::Aimt,
        vec![("id", "a2"), ("version", "0.1.0")],
        vec![("title", "Second")],
    );
    write(&abs, &e1).unwrap();
    write(&abs, &e2).unwrap();
    let back = aimt::reader::read(&abs).unwrap();
    assert_eq!(back.id().unwrap().as_str(), "a2");
    let ws = read(&dir).unwrap();
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0].entity.id().unwrap().as_str(), "a2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn empty_workspace_read_and_discover() {
    let dir = temp_dir();
    assert_eq!(discover(&dir).unwrap().len(), 0);
    assert_eq!(read(&dir).unwrap().len(), 0);
    // Add one file after empty check
    std::fs::write(dir.join("a.pmap"), MINIMAL_AIMT).unwrap();
    assert_eq!(discover(&dir).unwrap().len(), 1);
    assert_eq!(read(&dir).unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn error_kind_str_and_path_preservation() {
    let dir = temp_dir();
    let file = dir.join("not_a_dir_file");
    std::fs::write(&file, "x").unwrap();
    let err = discover(&file).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    assert_eq!(err.path().unwrap(), file.as_path());
    let missing = dir.join("missing_dir");
    let err2 = discover(&missing).unwrap_err();
    assert_eq!(err2.kind_str(), "Io");
    // Write error path preservation
    let nested = dir.join("nope/out.pmap");
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    let err3 = write(&nested, &e).unwrap_err();
    assert_eq!(err3.kind_str(), "Write");
    assert_eq!(err3.path().unwrap(), nested.as_path());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn arbitrary_directory_name_accepted() {
    let dir = temp_dir();
    // Rename-like: directory without .aimt suffix
    let arbitrary = dir.join("my_arbitrary_workspace_name");
    std::fs::create_dir_all(&arbitrary).unwrap();
    std::fs::write(arbitrary.join("a.pmap"), MINIMAL_AIMT).unwrap();
    let v = discover(&arbitrary).unwrap();
    assert_eq!(v.len(), 1);
    let r = read(&arbitrary).unwrap();
    assert_eq!(r.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_errors_sorted_lexically() {
    // Create two bad files with names that would be discovered in lexical order, but create in reverse
    let bad = "invalid content";
    let dir = temp_dir();
    std::fs::write(dir.join("z_bad.pmap"), bad).unwrap();
    std::fs::write(dir.join("a_bad.pmap"), bad).unwrap();
    let err = read(&dir).unwrap_err();
    match err {
        WorkspaceError::FileErrors(vec) => {
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0].path.file_name().unwrap(), "a_bad.pmap");
            assert_eq!(vec[1].path.file_name().unwrap(), "z_bad.pmap");
        }
        _ => panic!("expected FileErrors"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
