use aimt::model::AimtEntity;
use aimt::validation::{ValidationErrorKind, validate};

fn entity(src: &str) -> AimtEntity {
    AimtEntity::parse_and_convert(src).unwrap_or_else(|e| {
        panic!(
            "parse failed {} at {}:{}",
            e.kind.as_str(),
            e.span.start_line,
            e.span.start_col
        )
    })
}

fn assert_ok(src: &str) {
    let e = entity(src);
    let res = validate(&e);
    assert!(
        res.is_ok(),
        "expected ok, got errors {:?} for src:\n{}",
        res.unwrap_err(),
        src
    );
}

fn assert_err_kind(src: &str, kind: ValidationErrorKind, field: &str) {
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    assert!(
        errs.iter()
            .any(|err| err.kind == kind && err.field.as_deref() == Some(field)),
        "expected error kind {:?} for field {:?}, got {:?} for src:\n{}",
        kind,
        field,
        errs,
        src
    );
}

// ---------------------------------------------------------------------------
// Level tests
// ---------------------------------------------------------------------------

#[test]
fn all_seven_valid_levels() {
    let valid = [
        (
            "@aimt\n\n#header\n  id:\n    aimt1\n  version:\n    0.1.0\n#body\n  title:\n    T\n",
            aimt::syntax::Level::Aimt,
        ),
        (
            "@map\n\n#header\n  id:\n    map1\n  title:\n    M\n#body\n  description:\n    d\n",
            aimt::syntax::Level::Map,
        ),
        (
            "@domain\n\n#header\n  id:\n    d1\n  title:\n    D\n#body\n  description:\n    hi\n",
            aimt::syntax::Level::Domain,
        ),
        (
            "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    d1\n",
            aimt::syntax::Level::Region,
        ),
        (
            "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n",
            aimt::syntax::Level::Node,
        ),
        (
            "@file\n\n#header\n  id:\n    file1\n  path:\n    src/main.rs\n#body\n  hash:\n    abc\n",
            aimt::syntax::Level::File,
        ),
        (
            "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    file1\n  target:\n    Foo\n  type:\n    class\n",
            aimt::syntax::Level::Frame,
        ),
    ];
    for (src, lvl) in valid {
        let e = entity(src);
        assert_eq!(e.level, lvl);
        assert_ok(src);
    }
}

#[test]
fn level_specific_allowed_fields() {
    // @aimt with hash passes (Optional)
    assert_ok(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  hash:\n    abc\n#body\n  title:\n    T\n",
    );
    // @aimt with summary fails ForbiddenField (summary NotAllowed on @aimt)
    assert_err_kind(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  summary:\n    s\n#body\n  title:\n    T\n",
        ValidationErrorKind::ForbiddenField,
        "summary",
    );
    // @map with summary also forbidden
    assert_err_kind(
        "@map\n\n#header\n  id:\n    m1\n  title:\n    M\n  summary:\n    s\n#body\n  description:\n    d\n",
        ValidationErrorKind::ForbiddenField,
        "summary",
    );
}

// ---------------------------------------------------------------------------
// Required fields
// ---------------------------------------------------------------------------

#[test]
fn every_required_field_present() {
    // One passing entity per level with Required set populated
    assert_ok("@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n#body\n  title:\n    T\n");
    assert_ok("@map\n\n#header\n  id:\n    m1\n  title:\n    M\n#body\n  description:\n    d\n");
    assert_ok(
        "@domain\n\n#header\n  id:\n    d1\n  title:\n    D\n#body\n  description:\n    hi\n",
    );
    assert_ok("@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    d1\n");
    assert_ok("@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n");
    assert_ok("@file\n\n#header\n  id:\n    file1\n  path:\n    src/x.rs\n#body\n  hash:\n    h\n");
    assert_ok(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    file1\n  target:\n    Foo\n  type:\n    class\n",
    );
    // relation required fields
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n",
    );
}

#[test]
fn every_required_field_missing() {
    // @aimt missing version
    assert_err_kind(
        "@aimt\n\n#header\n  id:\n    a1\n#body\n  title:\n    T\n",
        ValidationErrorKind::MissingRequiredField,
        "version",
    );
    // @region missing parent
    assert_err_kind(
        "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  description:\n    hi\n",
        ValidationErrorKind::MissingRequiredField,
        "parent",
    );
    // @file missing path
    assert_err_kind(
        "@file\n\n#header\n  id:\n    f1\n#body\n  hash:\n    h\n",
        ValidationErrorKind::MissingRequiredField,
        "path",
    );
    // @frame missing file
    assert_err_kind(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  target:\n    Foo\n  type:\n    class\n",
        ValidationErrorKind::MissingRequiredField,
        "file",
    );
    // relation missing type
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      from:\n        n1\n      to:\n        n2\n",
        ValidationErrorKind::MissingRequiredField,
        "relations[0].type",
    );
}

#[test]
fn required_field_with_empty_value() {
    assert_err_kind(
        "@aimt\n\n#header\n  id:\n    \n  version:\n    0.1.0\n#body\n  title:\n    T\n",
        ValidationErrorKind::EmptyRequiredField,
        "id",
    );
    assert_err_kind(
        "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    \n",
        ValidationErrorKind::EmptyRequiredField,
        "parent",
    );
    assert_err_kind(
        "@file\n\n#header\n  id:\n    f1\n  path:\n    \n#body\n  hash:\n    h\n",
        ValidationErrorKind::EmptyRequiredField,
        "path",
    );
}

// ---------------------------------------------------------------------------
// Optional fields
// ---------------------------------------------------------------------------

#[test]
fn optional_absent_populated_empty() {
    // absent optional passes
    assert_ok("@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n");
    // populated optional passes
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  summary:\n    hello\n",
    );
    // empty optional where allowed passes
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  summary:\n",
    );
}

// ---------------------------------------------------------------------------
// Forbidden fields
// ---------------------------------------------------------------------------

#[test]
fn forbidden_fields() {
    // @aimt with parent
    assert_err_kind(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  parent:\n    x\n#body\n  title:\n    T\n",
        ValidationErrorKind::ForbiddenField,
        "parent",
    );
    // @file with target
    assert_err_kind(
        "@file\n\n#header\n  id:\n    f1\n  path:\n    src/x.rs\n  target:\n    Foo\n#body\n  hash:\n    h\n",
        ValidationErrorKind::ForbiddenField,
        "target",
    );
    // @map with summary
    assert_err_kind(
        "@map\n\n#header\n  id:\n    m1\n  title:\n    M\n  summary:\n    s\n#body\n  description:\n    d\n",
        ValidationErrorKind::ForbiddenField,
        "summary",
    );
    // context on @file not allowed
    assert_err_kind(
        "@file\n\n#header\n  id:\n    f1\n  path:\n    src/x.rs\n  context:\n    ctx\n#body\n  hash:\n    h\n",
        ValidationErrorKind::ForbiddenField,
        "context",
    );
    // from at top-level not allowed
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  from:\n    x\n",
        ValidationErrorKind::ForbiddenField,
        "from",
    );
}

// ---------------------------------------------------------------------------
// Unknown fields
// ---------------------------------------------------------------------------

#[test]
fn unknown_fields_preserved_and_rejected() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  custom_field:\n    hello\n";
    let e = entity(src);
    assert!(e.has_field("custom_field"));
    let errs = validate(&e).unwrap_err();
    assert!(
        errs.iter()
            .any(|err| err.kind == ValidationErrorKind::UnknownField
                && err.field.as_deref() == Some("custom_field"))
    );
    // span should equal field span
    let field = e.field("custom_field").unwrap();
    let err = errs
        .iter()
        .find(|err| err.field.as_deref() == Some("custom_field"))
        .unwrap();
    assert_eq!(err.span.start_line, field.span.start_line);
}

#[test]
fn unknown_field_in_relation_rejected() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n      custom_rel:\n        x\n";
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    assert!(
        errs.iter()
            .any(|err| err.kind == ValidationErrorKind::UnknownField
                && err.field.as_deref() == Some("relations[0].custom_rel"))
    );
}

// ---------------------------------------------------------------------------
// ID / reference shape
// ---------------------------------------------------------------------------

#[test]
fn valid_reference_values() {
    assert_ok(
        "@node\n\n#header\n  id:\n    node_001\n  title:\n    N\n#body\n  parent:\n    domain_001\n",
    );
    assert_ok(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    file_001\n  target:\n    Foo\n  type:\n    class\n",
    );
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n",
    );
}

#[test]
fn invalid_reference_values() {
    // empty parent where Required -> EmptyRequiredField
    assert_err_kind(
        "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    \n",
        ValidationErrorKind::EmptyRequiredField,
        "parent",
    );
    // invalid ID shape with space
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    Has Space\n",
        ValidationErrorKind::InvalidFieldValue,
        "parent",
    );
    // UPPER case
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    UPPER\n",
        ValidationErrorKind::InvalidFieldValue,
        "parent",
    );
    // from with invalid shape
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        Has Space\n      to:\n        n2\n",
        ValidationErrorKind::InvalidFieldValue,
        "relations[0].from",
    );
}

#[test]
fn unresolved_references_remain_unresolved() {
    // parent/file/from/to with unknown IDs should still pass (no existence check)
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    does_not_exist\n",
    );
    assert_ok(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    missing_file\n  target:\n    T\n  type:\n    class\n",
    );
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        unknown\n      to:\n        also_unknown\n",
    );
}

#[test]
fn id_shape_invalid() {
    assert_err_kind(
        "@node\n\n#header\n  id:\n    Has Space\n  title:\n    N\n#body\n  parent:\n    d1\n",
        ValidationErrorKind::InvalidFieldValue,
        "id",
    );
    assert_err_kind(
        "@node\n\n#header\n  id:\n    UPPER\n  title:\n    N\n#body\n  parent:\n    d1\n",
        ValidationErrorKind::InvalidFieldValue,
        "id",
    );
    // empty id
    assert_err_kind(
        "@node\n\n#header\n  id:\n    \n  title:\n    N\n#body\n  parent:\n    d1\n",
        ValidationErrorKind::EmptyRequiredField,
        "id",
    );
}

// ---------------------------------------------------------------------------
// Relations
// ---------------------------------------------------------------------------

#[test]
fn relations_zero() {
    assert_ok("@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n");
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n",
    );
}

#[test]
fn relations_valid() {
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        depends_on\n      from:\n        n1\n      to:\n        n2\n      evidence:\n        src/x.rs:1\n",
    );
}

#[test]
fn missing_relation_type_from_to() {
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      from:\n        n1\n      to:\n        n2\n",
        ValidationErrorKind::MissingRequiredField,
        "relations[0].type",
    );
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      to:\n        n2\n",
        ValidationErrorKind::MissingRequiredField,
        "relations[0].from",
    );
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n",
        ValidationErrorKind::MissingRequiredField,
        "relations[0].to",
    );
}

#[test]
fn invalid_relation_field() {
    assert_err_kind(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n      parent:\n        x\n",
        ValidationErrorKind::ForbiddenField,
        "relations[0].parent",
    );
}

#[test]
fn relation_outside_allowed_level() {
    assert_err_kind(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n#body\n  title:\n    T\n  relations:\n    @relation\n      type:\n        a\n      from:\n        x\n      to:\n        y\n",
        ValidationErrorKind::ForbiddenField,
        "relations",
    );
}

// ---------------------------------------------------------------------------
// Value-format boundary / deferred formats
// ---------------------------------------------------------------------------

#[test]
fn deferred_formats_not_validated() {
    // version, hash, created, location, evidence, source, context, title, etc. with arbitrary values should pass
    assert_ok(
        "@aimt\n\n#header\n  id:\n    a1\n  version:\n    not-semver\n#body\n  title:\n    T\n  hash:\n    not-a-hash-but-allowed\n",
    );
    assert_ok(
        "@file\n\n#header\n  id:\n    f1\n  path:\n    src/x.rs\n#body\n  location:\n    weird location 123\n  created:\n    not-iso\n",
    );
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n      evidence:\n        any evidence string\n",
    );
}

// ---------------------------------------------------------------------------
// Source spans
// ---------------------------------------------------------------------------

#[test]
fn source_spans_present() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    Has Space\n";
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    let err = errs
        .iter()
        .find(|e| e.field.as_deref() == Some("parent"))
        .unwrap();
    let field = e.field("parent").unwrap();
    assert_eq!(err.span.start_line, field.span.start_line);
}

#[test]
fn missing_required_uses_entity_span() {
    let src = "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  description:\n    hi\n";
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    let err = errs
        .iter()
        .find(|e| e.field.as_deref() == Some("parent"))
        .unwrap();
    assert_eq!(err.kind, ValidationErrorKind::MissingRequiredField);
    assert_eq!(err.span.start_line, e.span.start_line);
}

#[test]
fn relation_missing_uses_relation_span() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      from:\n        n1\n      to:\n        n2\n";
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    let err = errs
        .iter()
        .find(|e| e.field.as_deref() == Some("relations[0].type"))
        .unwrap();
    assert_eq!(err.span.start_line, e.relations[0].span.start_line);
}

// ---------------------------------------------------------------------------
// Deterministic error ordering
// ---------------------------------------------------------------------------

#[test]
fn deterministic_error_ordering() {
    // Create entity with multiple errors: forbidden + missing + invalid value
    // Use @aimt with forbidden parent and missing version? Actually @aimt requires version, so missing version + forbidden parent
    let src = "@aimt\n\n#header\n  id:\n    a1\n  parent:\n    x\n#body\n  title:\n    T\n";
    let e = entity(src);
    let errs1 = validate(&e).unwrap_err();
    let errs2 = validate(&e).unwrap_err();
    assert_eq!(errs1, errs2, "validation must be deterministic");
    // Check ordering is by span
    for window in errs1.windows(2) {
        let a = &window[0];
        let b = &window[1];
        assert!(
            (a.span.start_line, a.span.start_col) <= (b.span.start_line, b.span.start_col),
            "errors should be sorted by span"
        );
    }
}

#[test]
fn all_errors_collected_not_first() {
    let src = "@aimt\n\n#header\n  id:\n    \n  parent:\n    x\n#body\n  title:\n    T\n";
    // id empty (EmptyRequiredField) + parent forbidden + version missing
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    assert!(errs.len() >= 3, "should collect all errors, got {:?}", errs);
    assert!(
        errs.iter()
            .any(|e| e.kind == ValidationErrorKind::EmptyRequiredField
                && e.field.as_deref() == Some("id"))
    );
    assert!(
        errs.iter()
            .any(|e| e.kind == ValidationErrorKind::ForbiddenField
                && e.field.as_deref() == Some("parent"))
    );
    assert!(
        errs.iter()
            .any(|e| e.kind == ValidationErrorKind::MissingRequiredField
                && e.field.as_deref() == Some("version"))
    );
}

// ---------------------------------------------------------------------------
// No filesystem / reference resolution
// ---------------------------------------------------------------------------

#[test]
fn no_filesystem_resolution() {
    // Even though parent/file point to non-existent IDs, validation must not try to read filesystem
    // This test would fail if validation tried to check existence
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    no_such_parent\n",
    );
    assert_ok(
        "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    no_such_file\n  target:\n    Foo\n  type:\n    class\n",
    );
    assert_ok(
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        nonexistent\n      to:\n        also_nonexistent\n",
    );
}

#[test]
fn boundary_no_graph_built() {
    // Validation should not build graph or check that from/to point to same entity
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n1\n";
    assert_ok(src);
}

// ---------------------------------------------------------------------------
// Unknown fields already tested, but ensure they are distinct from forbidden
// ---------------------------------------------------------------------------

#[test]
fn unknown_vs_forbidden_distinction() {
    // "unknown_field" not in vocab -> UnknownField
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  unknown_field:\n    x\n";
    let e = entity(src);
    let errs = validate(&e).unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.kind == ValidationErrorKind::UnknownField)
    );

    // "summary" on @aimt is in vocab but NotAllowed -> ForbiddenField, not UnknownField
    let src2 = "@aimt\n\n#header\n  id:\n    a1\n  version:\n    0.1.0\n  summary:\n    s\n#body\n  title:\n    T\n";
    let e2 = entity(src2);
    let errs2 = validate(&e2).unwrap_err();
    assert!(
        errs2
            .iter()
            .any(|e| e.kind == ValidationErrorKind::ForbiddenField
                && e.field.as_deref() == Some("summary"))
    );
    assert!(
        !errs2
            .iter()
            .any(|e| e.kind == ValidationErrorKind::UnknownField
                && e.field.as_deref() == Some("summary"))
    );
}
