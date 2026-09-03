use aimt::parser::ErrorKind as ParseKind;
use aimt::reader::{ReaderError, read};
use aimt::syntax::Level;
use aimt::validation::ValidationErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_reader_test_{}_{}_{}",
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

fn with_temp_pmap<F>(content: &str, f: F)
where
    F: FnOnce(&Path),
{
    let dir = temp_dir();
    let path = dir.join("test.pmap");
    std::fs::write(&path, content).unwrap();
    f(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

fn with_temp_pmap_bytes<F>(bytes: &[u8], f: F)
where
    F: FnOnce(&Path),
{
    let dir = temp_dir();
    let path = dir.join("test.pmap");
    std::fs::write(&path, bytes).unwrap();
    f(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

fn with_temp_dir<F>(f: F)
where
    F: FnOnce(&Path),
{
    let dir = temp_dir();
    f(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Successful reads
// ---------------------------------------------------------------------------

#[test]
fn valid_minimal_pmap() {
    let content = std::fs::read_to_string("tests/fixtures/parser/valid/minimal_aimt.pmap").unwrap();
    with_temp_pmap(&content, |path| {
        let entity = read(path).expect("should read minimal_aimt");
        assert_eq!(entity.level, Level::Aimt);
        assert_eq!(entity.id().unwrap().as_str(), "aimt_root");
    });
}

#[test]
fn valid_entity_for_each_of_seven_levels() {
    let fixtures = [
        (
            "@aimt\n\n#header\n  id:\n    aimt1\n  version:\n    0.1.0\n#body\n  title:\n    T\n",
            Level::Aimt,
        ),
        (
            "@map\n\n#header\n  id:\n    map1\n  title:\n    M\n#body\n  description:\n    d\n",
            Level::Map,
        ),
        (
            "@domain\n\n#header\n  id:\n    d1\n  title:\n    D\n#body\n  description:\n    hi\n",
            Level::Domain,
        ),
        (
            "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    d1\n",
            Level::Region,
        ),
        (
            "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
            Level::Node,
        ),
        (
            "@file\n\n#header\n  id:\n    file1\n  path:\n    src/main.rs\n#body\n  hash:\n    abc\n",
            Level::File,
        ),
        (
            "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    file1\n  target:\n    Foo\n  type:\n    class\n",
            Level::Frame,
        ),
    ];
    for (content, level) in fixtures {
        with_temp_pmap(content, |path| {
            let entity = read(path)
                .unwrap_or_else(|e| panic!("level {:?} should read ok, got {:?}", level, e));
            assert_eq!(entity.level, level);
        });
    }
}

#[test]
fn valid_multiline_content() {
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  description:\n    para1\n\n    para2\n";
    with_temp_pmap(content, |path| {
        let entity = read(path).unwrap();
        assert_eq!(entity.field("description").unwrap().value, "para1\n\npara2");
    });
}

#[test]
fn valid_relations() {
    let content =
        std::fs::read_to_string("tests/fixtures/parser/valid/node_with_relations.pmap").unwrap();
    with_temp_pmap(&content, |path| {
        let entity = read(path).unwrap();
        assert_eq!(entity.relations.len(), 2);
        assert_eq!(entity.level, Level::Node);
    });
}

#[test]
fn valid_optional_fields() {
    // absent optional vs present vs empty on @node summary (Optional)
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
        |p| {
            assert!(read(p).is_ok());
        },
    );
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  summary:\n    hello\n",
        |p| {
            assert!(read(p).is_ok());
        },
    );
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  summary:\n",
        |p| {
            // empty optional is allowed (validation allows empty optional)
            assert!(read(p).is_ok());
        },
    );
}

// ---------------------------------------------------------------------------
// Parser propagation
// ---------------------------------------------------------------------------

#[test]
fn parser_missing_level() {
    with_temp_pmap("#header\n  id:\n    x\n#body\n  title:\n    x\n", |path| {
        let err = read(path).unwrap_err();
        assert!(matches!(err, ReaderError::Parse(_)));
        if let ReaderError::Parse(e) = err {
            assert_eq!(e.kind, ParseKind::MissingLevel);
        }
    });
}

#[test]
fn parser_invalid_indentation() {
    let content =
        std::fs::read_to_string("tests/fixtures/parser/invalid/wrong_indent_3_spaces.pmap")
            .unwrap();
    with_temp_pmap(&content, |path| {
        let err = read(path).unwrap_err();
        assert!(matches!(err, ReaderError::Parse(_)));
        if let ReaderError::Parse(e) = err {
            assert_eq!(e.kind, ParseKind::InvalidIndent);
        }
    });
}

#[test]
fn parser_missing_header_body() {
    with_temp_pmap(
        &std::fs::read_to_string("tests/fixtures/parser/invalid/missing_header.pmap").unwrap(),
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Parse(_)));
        },
    );
    with_temp_pmap(
        &std::fs::read_to_string("tests/fixtures/parser/invalid/missing_body.pmap").unwrap(),
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Parse(_)));
        },
    );
}

#[test]
fn parser_invalid_relation_syntax() {
    with_temp_pmap(
        &std::fs::read_to_string("tests/fixtures/parser/invalid/relation_singular.pmap").unwrap(),
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Parse(_)));
            if let ReaderError::Parse(e) = err {
                assert_eq!(e.kind, ParseKind::RelationOutsideRelations);
            }
        },
    );
}

#[test]
fn parser_invalid_utf8() {
    with_temp_pmap_bytes(&[0xFF, 0xFF, b'\n'], |path| {
        let err = read(path).unwrap_err();
        assert!(matches!(err, ReaderError::Parse(_)));
        if let ReaderError::Parse(e) = err {
            assert_eq!(e.kind, ParseKind::InvalidUtf8);
        }
    });
}

#[test]
fn parser_bom() {
    let bytes = std::fs::read("tests/fixtures/parser/invalid/bom_present.pmap").unwrap();
    with_temp_pmap_bytes(&bytes, |path| {
        let err = read(path).unwrap_err();
        assert!(matches!(err, ReaderError::Parse(_)));
        if let ReaderError::Parse(e) = err {
            assert_eq!(e.kind, ParseKind::BomPresent);
        }
    });
}

#[test]
fn parser_missing_trailing_newline() {
    // content without final \n
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n";
    let without_newline = content.trim_end_matches('\n');
    with_temp_pmap_bytes(without_newline.as_bytes(), |path| {
        let err = read(path).unwrap_err();
        assert!(matches!(err, ReaderError::Parse(_)));
        if let ReaderError::Parse(e) = err {
            assert_eq!(e.kind, ParseKind::MissingTrailingNewline);
        }
    });
}

// ---------------------------------------------------------------------------
// Validation propagation
// ---------------------------------------------------------------------------

#[test]
fn validation_missing_required_field() {
    with_temp_pmap(
        "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  description:\n    hi\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::MissingRequiredField
                            && e.field.as_deref() == Some("parent"))
                );
            }
        },
    );
}

#[test]
fn validation_empty_required_field() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    \n  title:\n    N\n#body\n  parent:\n    d1\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::EmptyRequiredField
                            && e.field.as_deref() == Some("id"))
                );
            }
        },
    );
}

#[test]
fn validation_forbidden_field() {
    with_temp_pmap(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  parent:\n    x\n#body\n  title:\n    T\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::ForbiddenField
                            && e.field.as_deref() == Some("parent"))
                );
            }
        },
    );
}

#[test]
fn validation_unknown_field() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  custom_field:\n    hello\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::UnknownField
                            && e.field.as_deref() == Some("custom_field"))
                );
            }
        },
    );
}

#[test]
fn validation_invalid_id_shape() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    Has Space\n  title:\n    N\n#body\n  parent:\n    d1\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::InvalidFieldValue
                            && e.field.as_deref() == Some("id"))
                );
            }
        },
    );
}

#[test]
fn validation_invalid_relation_fields() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      from:\n        n1\n      to:\n        n2\n",
        |path| {
            let err = read(path).unwrap_err();
            assert!(matches!(err, ReaderError::Validation(_)));
            if let ReaderError::Validation(vec) = err {
                assert!(
                    vec.iter()
                        .any(|e| e.kind == ValidationErrorKind::MissingRequiredField
                            && e.field.as_deref() == Some("relations[0].type"))
                );
            }
        },
    );
}

// ---------------------------------------------------------------------------
// Filesystem errors
// ---------------------------------------------------------------------------

#[test]
fn filesystem_missing_file() {
    let path = Path::new("/tmp/does_not_exist_12345_aimt_reader_test.pmap");
    // Ensure it does not exist
    let _ = std::fs::remove_file(path);
    let err = read(path).unwrap_err();
    assert!(matches!(err, ReaderError::Io(_)));
    assert_eq!(err.kind_str(), "Io");
    assert_eq!(err.path().unwrap(), path);
    if let ReaderError::Io(e) = err {
        assert_eq!(e.kind, std::io::ErrorKind::NotFound);
        assert_eq!(e.path, path.to_path_buf());
    }
}

#[test]
fn filesystem_path_is_directory() {
    with_temp_dir(|dir| {
        let err = read(dir).unwrap_err();
        assert!(matches!(err, ReaderError::Io(_)));
        assert_eq!(err.kind_str(), "Io");
        // Should be IsADirectory or Other, but must be Io and preserve path
        assert_eq!(err.path().unwrap(), dir);
    });
}

// ---------------------------------------------------------------------------
// Boundary tests
// ---------------------------------------------------------------------------

#[test]
fn reads_only_one_file() {
    let dir = temp_dir();
    let path1 = dir.join("one.pmap");
    let path2 = dir.join("two.pmap");
    std::fs::write(
        &path1,
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
    )
    .unwrap();
    std::fs::write(
        &path2,
        "@node\n\n#header\n  id:\n    n2\n  title:\n    M\n#body\n  parent:\n    d2\n",
    )
    .unwrap();
    let entity = read(&path1).unwrap();
    assert_eq!(entity.id().unwrap().as_str(), "n1");
    // Ensure second file not loaded
    assert_ne!(entity.id().unwrap().as_str(), "n2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn does_not_scan_directories() {
    with_temp_dir(|dir| {
        std::fs::write(
            dir.join("a.pmap"),
            "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
        )
        .unwrap();
        let err = read(dir).unwrap_err();
        assert!(matches!(err, ReaderError::Io(_)));
    });
}

#[test]
fn does_not_resolve_references() {
    with_temp_pmap(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    does_not_exist\n  target:\n    Foo\n  type:\n    class\n",
        |path| {
            // file: does_not_exist is unresolved but validation passes (existence deferred)
            let entity = read(path).expect("should be ok, existence deferred");
            assert_eq!(entity.file_ref().unwrap().as_str(), "does_not_exist");
        },
    );
}

#[test]
fn does_not_build_graph() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        unknown\n      to:\n        also_unknown\n",
        |path| {
            let entity = read(path).unwrap();
            assert_eq!(entity.relations.len(), 1);
            assert_eq!(entity.relations[0].get_value("from"), Some("unknown"));
        },
    );
}

#[test]
fn does_not_write_modify_source() {
    with_temp_pmap(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
        |path| {
            let before = std::fs::read(path).unwrap();
            let _ = read(path).unwrap();
            let after = std::fs::read(path).unwrap();
            assert_eq!(
                before, after,
                "file bytes must be identical before and after read"
            );
        },
    );
}

#[test]
fn does_not_implement_workspace_discovery() {
    with_temp_dir(|dir| {
        // Create a fake workspace dir with two files, but read on dir should be Io, not Vec
        std::fs::write(
            dir.join("a.pmap"),
            "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
        )
        .unwrap();
        let err = read(dir).unwrap_err();
        assert!(matches!(err, ReaderError::Io(_)));
    });
}

#[test]
fn deterministic_behavior() {
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n";
    with_temp_pmap(content, |p1| {
        with_temp_pmap(content, |p2| {
            let e1 = read(p1).unwrap();
            let e2 = read(p2).unwrap();
            assert_eq!(e1, e2);
        });
    });
    // Same bytes produce same validation error
    let bad = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    Has Space\n";
    with_temp_pmap(bad, |p1| {
        with_temp_pmap(bad, |p2| {
            let err1 = read(p1).unwrap_err();
            let err2 = read(p2).unwrap_err();
            assert_eq!(format!("{:?}", err1), format!("{:?}", err2));
        });
    });
}

#[test]
fn source_preservation() {
    let content = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  description:\n    hello\n\n    world\n";
    with_temp_pmap(content, |path| {
        let entity = read(path).unwrap();
        assert_eq!(entity.field("description").unwrap().value, "hello\n\nworld");
        // Span preserved
        assert!(entity.field("description").unwrap().span.start_line > 0);
    });
}

#[test]
fn no_filename_inference() {
    // Write file with id different from filename, ensure id is from field, not filename
    let dir = temp_dir();
    let path = dir.join("weird_name.pmap");
    std::fs::write(
        &path,
        "@node\n\n#header\n  id:\n    actual_id\n  title:\n    N\n#body\n  parent:\n    d1\n",
    )
    .unwrap();
    let entity = read(&path).unwrap();
    assert_eq!(entity.id().unwrap().as_str(), "actual_id");
    assert_ne!(entity.id().unwrap().as_str(), "weird_name");
    let _ = std::fs::remove_dir_all(&dir);
}
