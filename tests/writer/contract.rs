use aimt::model::{AimtEntity, Field, Relation};
use aimt::reader::read;
use aimt::syntax::{INDENT_WIDTH, Level, Span};
use aimt::writer::{WriterError, serialize, serialize_to_bytes, write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Temp helpers — std only, no tempfile crate
// ---------------------------------------------------------------------------

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_writer_test_{}_{}_{}",
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

fn with_temp_path<F>(f: F)
where
    F: FnOnce(&Path),
{
    let dir = temp_dir();
    let path = dir.join("out.pmap");
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

fn relation(fields: Vec<(&str, &str)>) -> Relation {
    Relation {
        fields: fields.into_iter().map(|(k, v)| field(k, v)).collect(),
        span: dummy_span(),
    }
}

fn make_entity(
    level: Level,
    header: Vec<(&str, &str)>,
    body: Vec<(&str, &str)>,
    relations: Vec<Relation>,
) -> AimtEntity {
    AimtEntity {
        level,
        level_span: dummy_span(),
        span: dummy_span(),
        header: header.into_iter().map(|(k, v)| field(k, v)).collect(),
        body: body.into_iter().map(|(k, v)| field(k, v)).collect(),
        relations,
    }
}

fn entity_from_fixture(name: &str) -> AimtEntity {
    let path = format!(
        "{}/tests/fixtures/parser/valid/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {name}"));
    AimtEntity::parse_and_convert(&content).unwrap_or_else(|e| {
        panic!(
            "parse fixture {name} failed {} at {}:{}",
            e.kind.as_str(),
            e.span.start_line,
            e.span.start_col
        )
    })
}

fn assert_entities_eq(a: &AimtEntity, b: &AimtEntity) {
    assert_eq!(a.level, b.level, "level mismatch");
    assert_eq!(a.header.len(), b.header.len(), "header len mismatch");
    for (fa, fb) in a.header.iter().zip(b.header.iter()) {
        assert_eq!(fa.name, fb.name, "header field name mismatch");
        assert_eq!(
            fa.value, fb.value,
            "header field value mismatch for {}",
            fa.name
        );
    }
    assert_eq!(a.body.len(), b.body.len(), "body len mismatch");
    for (fa, fb) in a.body.iter().zip(b.body.iter()) {
        assert_eq!(fa.name, fb.name, "body field name mismatch");
        assert_eq!(
            fa.value, fb.value,
            "body field value mismatch for {}",
            fa.name
        );
    }
    assert_eq!(
        a.relations.len(),
        b.relations.len(),
        "relations len mismatch"
    );
    for (ra, rb) in a.relations.iter().zip(b.relations.iter()) {
        assert_eq!(
            ra.fields.len(),
            rb.fields.len(),
            "relation fields len mismatch"
        );
        for (fa, fb) in ra.fields.iter().zip(rb.fields.iter()) {
            assert_eq!(fa.name, fb.name, "relation field name mismatch");
            assert_eq!(
                fa.value, fb.value,
                "relation field value mismatch for {}",
                fa.name
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 1. serialize minimal entity
// ---------------------------------------------------------------------------

#[test]
fn serialize_minimal_entity() {
    let entity = entity_from_fixture("minimal_aimt.pmap");
    let out = serialize(&entity).expect("serialize minimal should ok");
    assert!(out.contains("@aimt\n"), "should contain @aimt");
    assert!(out.contains("#header\n"), "should contain #header");
    assert!(out.contains("#body\n"), "should contain #body");
    assert!(out.contains("  id:\n    aimt_root\n"), "should contain id");
    assert!(out.ends_with('\n'), "should end with LF");
    assert!(!out.contains('\r'), "no CR");
    // No BOM
    assert_ne!(&out.as_bytes()[..3], &[0xEF, 0xBB, 0xBF], "no BOM");
}

// ---------------------------------------------------------------------------
// 2. serialize all seven levels
// ---------------------------------------------------------------------------

#[test]
fn serialize_all_seven_levels() {
    let cases = [
        (Level::Aimt, "aimt"),
        (Level::Map, "map"),
        (Level::Domain, "domain"),
        (Level::Region, "region"),
        (Level::Node, "node"),
        (Level::File, "file"),
        (Level::Frame, "frame"),
    ];
    for (level, name) in cases {
        let entity = match level {
            Level::Aimt => make_entity(
                level,
                vec![("id", "a1"), ("version", "0.1.0")],
                vec![("title", "T")],
                vec![],
            ),
            Level::Map => make_entity(
                level,
                vec![("id", "m1"), ("title", "M")],
                vec![("description", "d")],
                vec![],
            ),
            Level::Domain => make_entity(
                level,
                vec![("id", "d1"), ("title", "D")],
                vec![("description", "hi")],
                vec![],
            ),
            Level::Region => make_entity(
                level,
                vec![("id", "r1"), ("title", "R")],
                vec![("parent", "d1")],
                vec![],
            ),
            Level::Node => make_entity(
                level,
                vec![("id", "n1"), ("title", "N")],
                vec![("parent", "d1")],
                vec![],
            ),
            Level::File => make_entity(
                level,
                vec![("id", "f1"), ("path", "src/main.rs")],
                vec![("hash", "abc")],
                vec![],
            ),
            Level::Frame => make_entity(
                level,
                vec![("id", "f1"), ("title", "T")],
                vec![("file", "file1"), ("target", "Foo"), ("type", "class")],
                vec![],
            ),
        };
        let out =
            serialize(&entity).unwrap_or_else(|e| panic!("level {name} serialize failed {:?}", e));
        assert!(
            out.starts_with(&format!("@{name}\n")),
            "level {name} should start with @{name}"
        );
        assert!(out.contains(&format!("@{name}")), "should contain marker");
    }
}

// ---------------------------------------------------------------------------
// 3. serialize multiline values
// ---------------------------------------------------------------------------

#[test]
fn serialize_multiline_values() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("description", "para1\n\npara2")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    // Should contain description field with blank line between paragraphs at indent 4
    assert!(
        out.contains("  description:\n    para1\n\n    para2\n"),
        "multiline should preserve blank line"
    );
    // Ensure exactly single blank line, not double
    assert!(
        !out.contains("\n\n\n"),
        "should not have double blank inside value beyond single"
    );
}

// ---------------------------------------------------------------------------
// 4. serialize empty values
// ---------------------------------------------------------------------------

#[test]
fn serialize_empty_values() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("summary", "")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    // Empty field should be "  summary:\n" with no following indented value lines
    // Next field after summary is none, so next should be EOF or relations, but we have no next body field
    // Check that empty value does not produce indented line after
    assert!(
        out.contains("  summary:\n"),
        "empty field should be name:\\n"
    );
    // Ensure not "  summary:\n    " with value
    // The line after "  summary:\n" should be either not indented at 4 or EOF
    let lines: Vec<&str> = out.lines().collect();
    let idx = lines
        .iter()
        .position(|l| *l == "  summary:")
        .expect("summary line");
    // If summary is last body field, next line should not be indented value at 4 with content
    if idx + 1 < lines.len() {
        let next = lines[idx + 1];
        // next should not be value at indent 4 belonging to summary
        // Since summary is last, next should be absent or relations, but we have no relations, so should be EOF -> no next
        assert!(
            !next.starts_with("    ") || next.trim().is_empty(),
            "empty field should not have value line"
        );
    }
    assert!(out.ends_with('\n'), "ends with LF");
}

// ---------------------------------------------------------------------------
// 5. serialize zero relations
// ---------------------------------------------------------------------------

#[test]
fn serialize_zero_relations() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    assert!(
        !out.contains("relations:"),
        "zero relations should omit relations field"
    );
    assert!(
        !out.contains("@relation"),
        "zero relations should not contain @relation"
    );
}

// ---------------------------------------------------------------------------
// 6. serialize multiple relations
// ---------------------------------------------------------------------------

#[test]
fn serialize_multiple_relations() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![
            relation(vec![("type", "depends_on"), ("from", "n1"), ("to", "n2")]),
            relation(vec![("type", "uses"), ("from", "n1"), ("to", "n3")]),
        ],
    );
    let out = serialize(&entity).unwrap();
    assert!(
        out.contains("  relations:\n"),
        "should contain relations field"
    );
    // Count @relation occurrences
    assert_eq!(
        out.matches("@relation").count(),
        2,
        "should have 2 @relation"
    );
    // Check order: depends_on before uses
    let pos_depends = out.find("depends_on").unwrap();
    let pos_uses = out.find("uses").unwrap();
    assert!(pos_depends < pos_uses, "relation order preserved");
    // Check indents: relations at 2, @relation at 4, fields at 6, values at 8
    assert!(out.contains("    @relation\n"), " @relation at indent 4");
    assert!(out.contains("      type:\n"), "type at indent 6");
    assert!(out.contains("        depends_on\n"), "value at indent 8");
}

// ---------------------------------------------------------------------------
// 7. preserve unknown fields
// ---------------------------------------------------------------------------

#[test]
fn preserve_unknown_fields() {
    // Build entity with custom_field via programmatic (unknown to validation)
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("custom_field", "hello")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    assert!(
        out.contains("  custom_field:\n    hello\n"),
        "unknown field should be preserved"
    );
    // Also test unknown field in relation
    let entity2 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![relation(vec![
            ("type", "a"),
            ("from", "n1"),
            ("to", "n2"),
            ("custom_rel", "preserved"),
        ])],
    );
    let out2 = serialize(&entity2).unwrap();
    assert!(
        out2.contains("      custom_rel:\n"),
        "unknown relation field preserved at indent 6"
    );
    assert!(
        out2.contains("        preserved\n"),
        "unknown relation value at indent 8"
    );
}

// ---------------------------------------------------------------------------
// 8. deterministic serialization
// ---------------------------------------------------------------------------

#[test]
fn deterministic_serialization() {
    let entity = entity_from_fixture("node_with_relations.pmap");
    let a = serialize(&entity).unwrap();
    let b = serialize(&entity).unwrap();
    assert_eq!(a, b, "same entity should serialize deterministically");
    // Clone entity and serialize again
    let cloned = entity.clone();
    let c = serialize(&cloned).unwrap();
    assert_eq!(a, c, "cloned entity same bytes");
}

// ---------------------------------------------------------------------------
// 9. preserve header/body field order
// ---------------------------------------------------------------------------

#[test]
fn preserve_header_body_field_order() {
    // Header order: title then id (non-alphabetical) should be preserved
    let entity = AimtEntity {
        level: Level::Node,
        level_span: dummy_span(),
        span: dummy_span(),
        header: vec![field("title", "MyTitle"), field("id", "n1")],
        body: vec![
            field("type", "service"),
            field("parent", "d1"),
            field("description", "hi"),
        ],
        relations: vec![],
    };
    let out = serialize(&entity).unwrap();
    let pos_title = out.find("  title:").unwrap();
    let pos_id = out.find("  id:").unwrap();
    assert!(
        pos_title < pos_id,
        "header order preserved: title before id"
    );
    let pos_type = out.find("  type:").unwrap();
    let pos_parent = out.find("  parent:").unwrap();
    let pos_desc = out.find("  description:").unwrap();
    assert!(pos_type < pos_parent, "body order: type before parent");
    assert!(
        pos_parent < pos_desc,
        "body order: parent before description"
    );
    // Ensure not sorted alphabetically
    // If sorted, id would be before title, but we have title before id
}

// ---------------------------------------------------------------------------
// 10. preserve relation order
// ---------------------------------------------------------------------------

#[test]
fn preserve_relation_order() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![
            relation(vec![("type", "depends_on"), ("from", "n1"), ("to", "n2")]),
            relation(vec![("type", "uses"), ("from", "n1"), ("to", "n3")]),
            relation(vec![("type", "a"), ("from", "n1"), ("to", "n4")]),
        ],
    );
    let out = serialize(&entity).unwrap();
    let p0 = out.find("depends_on").unwrap();
    let p1 = out.find("uses").unwrap();
    let p2 = out.find("\n        a\n").unwrap();
    assert!(p0 < p1 && p1 < p2, "relation order preserved");
}

// ---------------------------------------------------------------------------
// 11. exact indentation
// ---------------------------------------------------------------------------

#[test]
fn exact_indentation() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![relation(vec![
            ("type", "depends_on"),
            ("from", "n1"),
            ("to", "n2"),
        ])],
    );
    let out = serialize(&entity).unwrap();
    for line in out.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('@') || line.starts_with('#') {
            // level and section markers at column 0, no indent
            assert!(
                !line.starts_with(' '),
                "marker should be at col 0: {:?}",
                line
            );
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        assert_eq!(
            indent % INDENT_WIDTH,
            0,
            "indent must be multiple of 2: {:?}",
            line
        );
        // Specific depths: field at 2, value at 4, @relation at 4, rel field at 6, rel value at 8
        // We verify no odd indents like 3
        assert_ne!(indent, 3, "no 3-space indent");
        assert_ne!(indent, 1, "no 1-space indent");
        assert_ne!(indent, 5, "no 5-space indent");
        assert_ne!(indent, 7, "no 7-space indent");
    }
    // Explicit checks for known levels
    assert!(out.contains("  id:\n"), "field at indent 2");
    assert!(
        out.contains("    n1\n") || out.contains("    N\n"),
        "value at indent 4"
    );
    assert!(
        out.contains("    @relation\n"),
        "relation marker at indent 4"
    );
    assert!(out.contains("      type:\n"), "relation field at indent 6");
    assert!(
        out.contains("        depends_on\n"),
        "relation value at indent 8"
    );
    // Ensure no line has 3 leading spaces
    for line in out.lines() {
        assert!(
            !line.starts_with("   ") || line.starts_with("    "),
            "no 3-space line"
        );
        // The above allows 4,6,8 but not 3
        if line.starts_with("   ") && !line.starts_with("    ") {
            panic!("found 3-space indent: {:?}", line);
        }
    }
}

// ---------------------------------------------------------------------------
// 12. no tabs
// ---------------------------------------------------------------------------

#[test]
fn no_tabs() {
    let entity = entity_from_fixture("node_with_relations.pmap");
    let out = serialize(&entity).unwrap();
    assert!(!out.contains('\t'), "output must not contain tabs");
    // Also check multiline entity
    let e2 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("description", "hello\n\nworld")],
        vec![],
    );
    let out2 = serialize(&e2).unwrap();
    assert!(!out2.contains('\t'), "multiline output no tabs");
}

// ---------------------------------------------------------------------------
// 13. final single LF
// ---------------------------------------------------------------------------

#[test]
fn final_single_lf() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    assert!(out.ends_with('\n'), "must end with LF");
    assert!(!out.ends_with("\n\n"), "must not end with blank line");
    assert!(!out.ends_with("\r\n"), "no CRLF");
    // Exactly one LF at EOF: remove last char should not end with \n
    let without_last = &out[..out.len() - 1];
    assert!(
        !without_last.ends_with('\n')
            || !without_last.ends_with("\n\n") && out.matches('\n').count() >= 1
    );
    // Count trailing LFs: should be exactly one
    let trailing = out.chars().rev().take_while(|c| *c == '\n').count();
    assert_eq!(trailing, 1, "exactly one trailing LF");
    // Check via bytes
    let bytes = serialize_to_bytes(&entity).unwrap();
    assert_eq!(bytes.last(), Some(&b'\n'), "bytes end with LF");
    assert_ne!(
        bytes[bytes.len() - 2..],
        [b'\n', b'\n'],
        "not double LF at end"
    );
}

// ---------------------------------------------------------------------------
// 14. no BOM
// ---------------------------------------------------------------------------

#[test]
fn no_bom() {
    let entity = entity_from_fixture("minimal_aimt.pmap");
    let out = serialize(&entity).unwrap();
    assert!(!out.starts_with('\u{FEFF}'), "no BOM char");
    let bytes = serialize_to_bytes(&entity).unwrap();
    assert_ne!(
        &bytes[0..3.min(bytes.len())],
        &[0xEF, 0xBB, 0xBF],
        "bytes no BOM"
    );
    // Also empty value entity
    let e2 = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
        vec![],
    );
    let b2 = serialize_to_bytes(&e2).unwrap();
    assert!(!b2.starts_with(&[0xEF, 0xBB, 0xBF]), "no BOM for aimt");
}

// ---------------------------------------------------------------------------
// 15. no CRLF
// ---------------------------------------------------------------------------

#[test]
fn no_crlf() {
    let entity = entity_from_fixture("multiline_two_paragraphs.pmap");
    let out = serialize(&entity).unwrap();
    assert!(!out.contains("\r\n"), "no CRLF");
    assert!(!out.contains('\r'), "no CR at all");
    let bytes = serialize_to_bytes(&entity).unwrap();
    assert!(!bytes.contains(&b'\r'), "bytes no CR");
    // Also test entity with relations
    let e2 = entity_from_fixture("node_with_relations.pmap");
    let out2 = serialize(&e2).unwrap();
    assert!(!out2.contains('\r'), "node_with_relations no CR");
}

// ---------------------------------------------------------------------------
// 16. invalid field name → WriterError::Serialize
// ---------------------------------------------------------------------------

#[test]
fn invalid_field_name_serialize_error() {
    // Uppercase letter
    let entity = make_entity(
        Level::Node,
        vec![("Bad", "x"), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    let err = serialize(&entity).unwrap_err();
    assert!(
        matches!(err, WriterError::Serialize(_)),
        "should be Serialize"
    );
    assert_eq!(err.kind_str(), "Serialize");
    if let WriterError::Serialize(e) = err {
        assert_eq!(e.kind, aimt::writer::SerializeErrorKind::InvalidFieldName);
        assert_eq!(e.field.as_deref(), Some("Bad"));
    }
    // Invalid starting with underscore
    let entity2 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("_bad", "x")],
        vec![],
    );
    let err2 = serialize(&entity2).unwrap_err();
    assert!(matches!(err2, WriterError::Serialize(_)));
    // Relation field invalid
    let entity3 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![relation(vec![("type", "a"), ("Bad", "x"), ("to", "n2")])],
    );
    // Missing "from" but invalid name should trigger Serialize before validation
    let err3 = serialize(&entity3).unwrap_err();
    assert!(matches!(err3, WriterError::Serialize(_)));
    if let WriterError::Serialize(e) = err3 {
        assert_eq!(e.field.as_deref(), Some("Bad"));
    }
}

// ---------------------------------------------------------------------------
// 17. serialize_to_bytes equals serialize().into_bytes()
// ---------------------------------------------------------------------------

#[test]
fn serialize_to_bytes_equals_serialize() {
    let entity = entity_from_fixture("node_with_relations.pmap");
    let s = serialize(&entity).unwrap();
    let b = serialize_to_bytes(&entity).unwrap();
    assert_eq!(b, s.into_bytes(), "bytes should equal string's bytes");
    // Also test minimal
    let e2 = entity_from_fixture("minimal_aimt.pmap");
    assert_eq!(
        serialize_to_bytes(&e2).unwrap(),
        serialize(&e2).unwrap().into_bytes()
    );
    // Multiline
    let e3 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("description", "a\n\nb")],
        vec![],
    );
    assert_eq!(
        serialize_to_bytes(&e3).unwrap(),
        serialize(&e3).unwrap().into_bytes()
    );
}

// ---------------------------------------------------------------------------
// 18. write creates one file
// ---------------------------------------------------------------------------

#[test]
fn write_creates_one_file() {
    let entity = entity_from_fixture("minimal_aimt.pmap");
    let expected = serialize(&entity).unwrap();
    with_temp_path(|path| {
        // Ensure not exists before
        assert!(!path.exists(), "path should not exist before write");
        write(path, &entity).expect("write should succeed");
        assert!(path.exists(), "file should exist after write");
        assert!(path.is_file(), "should be regular file");
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(
            bytes,
            expected.into_bytes(),
            "file bytes should equal serialize"
        );
        // Ensure no extra files created via glob? Check parent dir has exactly one file
        let parent = path.parent().unwrap();
        let count = std::fs::read_dir(parent).unwrap().count();
        assert_eq!(count, 1, "should create exactly one file");
    });
}

// ---------------------------------------------------------------------------
// 19. write overwrites existing file
// ---------------------------------------------------------------------------

#[test]
fn write_overwrites_existing_file() {
    let entity1 = entity_from_fixture("minimal_aimt.pmap");
    let entity2 = entity_from_fixture("file_no_parent.pmap");
    with_temp_path(|path| {
        write(path, &entity1).unwrap();
        let bytes1 = std::fs::read(path).unwrap();
        assert_eq!(bytes1, serialize(&entity1).unwrap().into_bytes());
        write(path, &entity2).unwrap();
        let bytes2 = std::fs::read(path).unwrap();
        assert_eq!(bytes2, serialize(&entity2).unwrap().into_bytes());
        assert_ne!(bytes1, bytes2, "bytes should differ after overwrite");
        let read_back = read(path).unwrap();
        assert_entities_eq(&entity2, &read_back);
        assert_eq!(read_back.level, Level::File);
    });
}

// ---------------------------------------------------------------------------
// 20. write directory → Io error
// ---------------------------------------------------------------------------

#[test]
fn write_directory_io_error() {
    let entity = entity_from_fixture("minimal_aimt.pmap");
    with_temp_dir(|dir| {
        let err = write(dir, &entity).unwrap_err();
        assert!(matches!(err, WriterError::Io(_)), "should be Io error");
        assert_eq!(err.kind_str(), "Io");
        assert_eq!(err.path(), Some(dir));
        if let WriterError::Io(e) = err {
            // IsADirectory or other, but must be Io
            // On macOS, writing to directory yields IsADirectory
            assert!(
                e.kind == std::io::ErrorKind::IsADirectory
                    || e.kind == std::io::ErrorKind::PermissionDenied
                    || e.kind == std::io::ErrorKind::Other,
                "kind should be directory-related, got {:?}",
                e.kind
            );
            assert_eq!(e.path, dir.to_path_buf());
        }
    });
}

// ---------------------------------------------------------------------------
// 21. missing parent → Io(NotFound)
// ---------------------------------------------------------------------------

#[test]
fn missing_parent_io_not_found() {
    let entity = entity_from_fixture("minimal_aimt.pmap");
    with_temp_dir(|dir| {
        let missing = dir.join("no_such_dir/out.pmap");
        let err = write(&missing, &entity).unwrap_err();
        assert!(matches!(err, WriterError::Io(_)));
        assert_eq!(err.kind_str(), "Io");
        if let WriterError::Io(e) = err {
            assert_eq!(
                e.kind,
                std::io::ErrorKind::NotFound,
                "missing parent should be NotFound"
            );
            assert_eq!(e.path, missing);
        }
        // Ensure file not created
        assert!(!missing.exists(), "file should not exist");
    });
}

// ---------------------------------------------------------------------------
// 22. writer does not create parent directories
// ---------------------------------------------------------------------------

#[test]
fn writer_does_not_create_parent_directories() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    with_temp_dir(|dir| {
        let nested = dir.join("a/b/c/out.pmap");
        // a/b/c should not exist before
        assert!(!dir.join("a").exists());
        let err = write(&nested, &entity).unwrap_err();
        assert!(matches!(err, WriterError::Io(_)));
        if let WriterError::Io(e) = err {
            assert_eq!(e.kind, std::io::ErrorKind::NotFound);
        }
        // Verify parent directories not created
        assert!(
            !dir.join("a").exists(),
            "writer should not create parent dirs"
        );
        assert!(!nested.exists(), "file not created");
    });
}

// ---------------------------------------------------------------------------
// 23. valid Writer → Reader round-trip
// ---------------------------------------------------------------------------

#[test]
fn valid_writer_reader_round_trip() {
    let fixtures = [
        "minimal_aimt.pmap",
        "file_no_parent.pmap",
        "node_with_relations.pmap",
        "multiline_two_paragraphs.pmap",
        "empty_relations.pmap",
    ];
    for name in fixtures {
        let entity = entity_from_fixture(name);
        with_temp_path(|path| {
            write(path, &entity).expect("write should succeed");
            let read_back =
                read(path).unwrap_or_else(|e| panic!("round-trip read failed for {name}: {:?}", e));
            assert_entities_eq(&entity, &read_back);
        });
    }
}

// ---------------------------------------------------------------------------
// 24. multiline/empty values round-trip
// ---------------------------------------------------------------------------

#[test]
fn multiline_empty_values_round_trip() {
    // Multiline with two paragraphs
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![
            ("parent", "d1"),
            (
                "description",
                "This is the first paragraph.\n\nThis is the second paragraph.",
            ),
            ("type", "service"),
        ],
        vec![],
    );
    with_temp_path(|path| {
        write(path, &entity).unwrap();
        let back = read(path).unwrap();
        assert_eq!(
            back.field("description").unwrap().value,
            "This is the first paragraph.\n\nThis is the second paragraph."
        );
        assert_entities_eq(&entity, &back);
    });
    // Empty value round-trip
    let entity2 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("description", ""), ("type", "service")],
        vec![],
    );
    with_temp_path(|path| {
        write(path, &entity2).unwrap();
        let back = read(path).unwrap();
        assert_eq!(back.field("description").unwrap().value, "");
        assert!(back.field("description").unwrap().is_empty());
        // Ensure empty vs missing distinct after round-trip: description present as empty
        assert!(back.has_field("description"));
        // Check serialization preserves empty: read file string should have "  description:\n" without value
        let content = std::fs::read_to_string(path).unwrap();
        assert!(
            content.contains("  description:\n"),
            "empty value should serialize as field line only"
        );
        assert_entities_eq(&entity2, &back);
    });
    // Also test empty via fixture
    let empty_rel = entity_from_fixture("empty_relations.pmap");
    with_temp_path(|path| {
        write(path, &empty_rel).unwrap();
        let back = read(path).unwrap();
        // empty_relations fixture has 0 relations, writer omits relations field, reader should still have 0 relations
        assert_eq!(back.relations.len(), 0);
    });
}

// ---------------------------------------------------------------------------
// 25. unknown fields round-trip
// ---------------------------------------------------------------------------

#[test]
fn unknown_fields_round_trip() {
    // Build entity with unknown field, verify serialize preserves it via bytes,
    // then parse without validation and ensure field still present.
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1"), ("custom_field", "hello")],
        vec![],
    );
    let bytes = serialize(&entity).unwrap();
    assert!(
        bytes.contains("custom_field"),
        "serialize should contain custom field"
    );
    // Parse bytes directly (without validation) to verify preservation
    let parsed = aimt::parser::parse(&bytes).expect("serialized bytes should parse");
    let reparsed = AimtEntity::from_parsed(parsed);
    assert!(
        reparsed.has_field("custom_field"),
        "reparsed should have custom field"
    );
    assert_eq!(reparsed.field("custom_field").unwrap().value, "hello");

    // Also verify via write -> read yields Validation error that preserves field name (since Reader validates)
    with_temp_path(|path| {
        write(path, &entity).unwrap();
        let err = read(path).unwrap_err();
        match err {
            aimt::reader::ReaderError::Validation(vec) => {
                assert!(
                    vec.iter()
                        .any(|e| e.field.as_deref() == Some("custom_field")),
                    "validation should mention custom_field"
                );
            }
            other => panic!(
                "expected Validation error for custom_field, got {:?}",
                other
            ),
        }
        // Ensure file bytes still contain custom field
        let file_content = std::fs::read_to_string(path).unwrap();
        assert!(file_content.contains("custom_field"));
    });

    // Unknown field in relation round-trip via parser
    let entity2 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![relation(vec![
            ("type", "a"),
            ("from", "n1"),
            ("to", "n2"),
            ("custom_rel", "preserved"),
        ])],
    );
    let bytes2 = serialize(&entity2).unwrap();
    let reparsed2 = AimtEntity::from_parsed(aimt::parser::parse(&bytes2).unwrap());
    assert_eq!(
        reparsed2.relations[0].get_value("custom_rel"),
        Some("preserved")
    );
}

// ---------------------------------------------------------------------------
// 26. relation ordering round-trip
// ---------------------------------------------------------------------------

#[test]
fn relation_ordering_round_trip() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![
            relation(vec![("type", "depends_on"), ("from", "n1"), ("to", "n2")]),
            relation(vec![("type", "uses"), ("from", "n1"), ("to", "n3")]),
        ],
    );
    with_temp_path(|path| {
        write(path, &entity).unwrap();
        let back = read(path).unwrap();
        assert_eq!(back.relations.len(), 2);
        assert_eq!(back.relations[0].get_value("type"), Some("depends_on"));
        assert_eq!(back.relations[1].get_value("type"), Some("uses"));
        // Ensure ordering preserved exactly
        assert_entities_eq(&entity, &back);
    });
    // Also check bytes order
    let out = serialize(&entity).unwrap();
    assert!(out.find("depends_on").unwrap() < out.find("uses").unwrap());
}

// ---------------------------------------------------------------------------
// 27. Writer does not perform semantic validation
// ---------------------------------------------------------------------------

#[test]
fn writer_does_not_perform_semantic_validation() {
    // Missing required field version on @aimt should still serialize Ok
    let entity = make_entity(
        Level::Aimt,
        vec![("id", "a1")],
        vec![("title", "T")],
        vec![],
    );
    let out = serialize(&entity).expect("writer should not validate missing version");
    assert!(out.contains("@aimt"), "should still serialize @aimt");
    assert!(!out.contains("version"), "version absent but writer ok");
    // Forbidden field: parent on @aimt should still serialize Ok (validation deferred)
    let entity2 = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0"), ("parent", "x")],
        vec![("title", "T")],
        vec![],
    );
    let out2 = serialize(&entity2).expect("writer should not reject forbidden field");
    assert!(
        out2.contains("parent"),
        "forbidden parent should be serialized"
    );
    // Verify that Reader then fails validation for these entities
    with_temp_path(|path| {
        write(path, &entity).unwrap();
        let err = read(path).unwrap_err();
        assert!(
            matches!(err, aimt::reader::ReaderError::Validation(_)),
            "reader should surface validation error for missing version"
        );
    });
    with_temp_path(|path| {
        write(path, &entity2).unwrap();
        let err = read(path).unwrap_err();
        assert!(
            matches!(err, aimt::reader::ReaderError::Validation(_)),
            "reader should surface forbidden field error"
        );
    });
    // Empty required field also not rejected by writer
    let entity3 = make_entity(
        Level::Node,
        vec![("id", ""), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    let out3 = serialize(&entity3).expect("empty required should still serialize");
    assert!(
        out3.contains("  id:\n"),
        "empty id serialized as empty block"
    );
}

// ---------------------------------------------------------------------------
// 28. Writer does not resolve references
// ---------------------------------------------------------------------------

#[test]
fn writer_does_not_resolve_references() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "does_not_exist")],
        vec![],
    );
    let out = serialize(&entity).expect("should serialize unknown parent");
    assert!(out.contains("does_not_exist"));
    with_temp_path(|path| {
        write(path, &entity).unwrap();
        let back = read(path).unwrap();
        assert_eq!(back.parent_ref().unwrap().as_str(), "does_not_exist");
    });
    // Frame file reference does not exist
    let frame = make_entity(
        Level::Frame,
        vec![("id", "f1"), ("title", "T")],
        vec![
            ("file", "missing_file"),
            ("target", "Foo"),
            ("type", "class"),
        ],
        vec![],
    );
    let out2 = serialize(&frame).unwrap();
    assert!(out2.contains("missing_file"));
    // Relation from/to unknown
    let entity3 = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
        vec![relation(vec![
            ("type", "a"),
            ("from", "unknown"),
            ("to", "also_unknown"),
        ])],
    );
    let out3 = serialize(&entity3).unwrap();
    assert!(out3.contains("unknown"));
    with_temp_path(|path| {
        write(path, &entity3).unwrap();
        let back = read(path).unwrap();
        assert_eq!(back.relations[0].get_value("from"), Some("unknown"));
    });
}

// ---------------------------------------------------------------------------
// 29. Writer does not infer ID from filename
// ---------------------------------------------------------------------------

#[test]
fn writer_does_not_infer_id_from_filename() {
    let entity = make_entity(
        Level::Node,
        vec![("id", "actual_id"), ("title", "N")],
        vec![("parent", "d1")],
        vec![],
    );
    let dir = temp_dir();
    let weird_path = dir.join("weird_name.pmap");
    write(&weird_path, &entity).unwrap();
    let back = read(&weird_path).unwrap();
    assert_eq!(
        back.id().unwrap().as_str(),
        "actual_id",
        "id should be from field, not filename"
    );
    assert_ne!(back.id().unwrap().as_str(), "weird_name");
    // Also test file with id different from filename and ensure writer writes exactly supplied path
    let file_entity = entity_from_fixture("file_no_parent.pmap");
    let custom = dir.join("custom_file.pmap");
    write(&custom, &file_entity).unwrap();
    let back2 = read(&custom).unwrap();
    assert_eq!(back2.id().unwrap().as_str(), "file_001");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Additional: writer does not use read_dir / walkdir / create_dir_all / graph
// ---------------------------------------------------------------------------

#[test]
fn writer_source_does_not_use_forbidden_apis() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/storage/writer.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(!src.contains("read_dir"), "writer should not use read_dir");
    assert!(!src.contains("walkdir"), "writer should not use walkdir");
    assert!(
        !src.contains("create_dir_all"),
        "writer should not use create_dir_all"
    );
    assert!(!src.contains("glob"), "writer should not use glob");
    assert!(
        !src.contains("validation::validate"),
        "writer should not call validation::validate"
    );
    assert!(
        src.contains("std::fs::write"),
        "writer should use std::fs::write"
    );
}

// Ensure serialization uses syntax constants (indirect via indent checks already, but also check marker constants)
#[test]
fn serialization_uses_canonical_markers() {
    let entity = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
        vec![],
    );
    let out = serialize(&entity).unwrap();
    assert!(out.contains("#header"), "should contain #header");
    assert!(out.contains("#body"), "should contain #body");
    // Exactly one blank line between @level and #header
    assert!(
        out.starts_with("@aimt\n\n#header\n"),
        "canonical blank line between @level and #header"
    );
    assert!(
        out.contains("#header\n  id:\n    a1\n"),
        "field at correct indent"
    );
}
