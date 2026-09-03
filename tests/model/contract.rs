use aimt::model::{AimtEntity, Level};
use aimt::parser::parse;

fn parse_to_model(src: &str) -> AimtEntity {
    let parsed = parse(src).unwrap_or_else(|e| {
        panic!(
            "parse failed {} at {}:{}",
            e.kind.as_str(),
            e.span.start_line,
            e.span.start_col
        )
    });
    AimtEntity::from_parsed(parsed)
}

// ---------------------------------------------------------------------------
// All seven levels
// ---------------------------------------------------------------------------

#[test]
fn all_seven_levels_convert() {
    for (level_str, level_enum) in [
        ("aimt", Level::Aimt),
        ("map", Level::Map),
        ("domain", Level::Domain),
        ("region", Level::Region),
        ("node", Level::Node),
        ("file", Level::File),
        ("frame", Level::Frame),
    ] {
        let src = format!(
            "@{level_str}\n\n#header\n  id:\n    id_{level_str}\n  title:\n    T\n#body\n  parent:\n    p1\n"
        );
        // For file/frame we need path/file etc, but model should still convert without validation
        let src_file_frame = match level_str {
            "file" => "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/main.rs\n#body\n  hash:\n    abc\n"
                .to_string(),
            "frame" => "@frame\n\n#header\n  id:\n    frame_001\n  title:\n    T\n#body\n  file:\n    file_001\n  target:\n    Foo\n  type:\n    class\n"
                .to_string(),
            _ => src,
        };
        let entity = parse_to_model(&src_file_frame);
        assert_eq!(entity.level, level_enum, "level {level_str} should map");
        assert_eq!(entity.level.as_str(), level_str);
        assert_eq!(entity.level.marker(), format!("@{level_str}"));
    }
}

// ---------------------------------------------------------------------------
// ParsedPmap → model conversion
// ---------------------------------------------------------------------------

#[test]
fn conversion_from_parsed_pmap() {
    let src = "@node\n\n#header\n  id:\n    node_001\n  title:\n    N\n#body\n  parent:\n    domain_001\n  type:\n    service\n";
    let parsed = parse(src).unwrap();
    let entity = AimtEntity::from_parsed(parsed);
    assert_eq!(entity.level, Level::Node);
    assert!(entity.field("id").is_some());
    assert!(entity.field("parent").is_some());
}

#[test]
fn conversion_via_parse_and_convert_helper() {
    let src =
        "@domain\n\n#header\n  id:\n    d1\n  title:\n    D\n#body\n  description:\n    hello\n";
    let entity = AimtEntity::parse_and_convert(src).unwrap();
    assert_eq!(entity.level, Level::Domain);
}

// ---------------------------------------------------------------------------
// Common fields
// ---------------------------------------------------------------------------

#[test]
fn common_fields_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    MyNode\n#body\n  description:\n    A description\n  summary:\n    short\n";
    let entity = parse_to_model(src);
    assert_eq!(entity.field("id").unwrap().value, "n1");
    assert_eq!(entity.field("title").unwrap().value, "MyNode");
    assert_eq!(entity.field("description").unwrap().value, "A description");
    assert_eq!(entity.field("summary").unwrap().value, "short");
}

// ---------------------------------------------------------------------------
// Optional fields
// ---------------------------------------------------------------------------

#[test]
fn optional_fields_absent_vs_present() {
    let src_missing =
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n";
    let entity_missing = parse_to_model(src_missing);
    assert!(entity_missing.field("summary").is_none());
    assert!(entity_missing.field("context").is_none());

    let src_present = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  context:\n    ctx\n  summary:\n    s\n";
    let entity_present = parse_to_model(src_present);
    assert_eq!(entity_present.field("summary").unwrap().value, "s");
    assert_eq!(entity_present.field("context").unwrap().value, "ctx");
}

// ---------------------------------------------------------------------------
// Empty values
// ---------------------------------------------------------------------------

#[test]
fn empty_values_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  description:\n  type:\n    service\n";
    let entity = parse_to_model(src);
    let desc = entity.field("description").unwrap();
    assert_eq!(
        desc.value, "",
        "empty description should be preserved as empty string"
    );
    assert!(desc.is_empty());
    // non-empty still works
    assert_eq!(entity.field("type").unwrap().value, "service");
}

#[test]
fn empty_vs_missing_distinct() {
    let src_empty =
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  summary:\n";
    let entity_empty = parse_to_model(src_empty);
    assert!(entity_empty.field("summary").is_some());
    assert_eq!(entity_empty.field("summary").unwrap().value, "");

    let src_missing =
        "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n";
    let entity_missing = parse_to_model(src_missing);
    assert!(entity_missing.field("summary").is_none());
}

// ---------------------------------------------------------------------------
// IDs
// ---------------------------------------------------------------------------

#[test]
fn ids_as_entity_id() {
    let src = "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/main.rs\n#body\n  hash:\n    abc\n";
    let entity = parse_to_model(src);
    let id = entity.id().unwrap();
    assert_eq!(id.as_str(), "file_001");
    // id field also preserved as generic field
    assert_eq!(entity.field("id").unwrap().value, "file_001");
}

// ---------------------------------------------------------------------------
// References — remain IDs, not resolved
// ---------------------------------------------------------------------------

#[test]
fn references_remain_ids() {
    let src =
        "@region\n\n#header\n  id:\n    r1\n  title:\n    R\n#body\n  parent:\n    domain_001\n";
    let entity = parse_to_model(src);
    let parent = entity.parent_ref().unwrap();
    assert_eq!(parent.as_str(), "domain_001");
    // Ensure it's stored as plain string, not resolved object
    assert_eq!(entity.field("parent").unwrap().value, "domain_001");
}

#[test]
fn references_are_plain_values() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    domain_001\n  description:\n    hello\n";
    let entity = parse_to_model(src);
    // parent is just a field value, not a resolved entity
    let field = entity.field("parent").unwrap();
    assert_eq!(field.value, "domain_001");
    assert_eq!(field.name, "parent");
}

// ---------------------------------------------------------------------------
// parent
// ---------------------------------------------------------------------------

#[test]
fn parent_reference() {
    let src = "@node\n\n#header\n  id:\n    node_001\n  title:\n    N\n#body\n  parent:\n    region_001\n  type:\n    service\n";
    let entity = parse_to_model(src);
    assert_eq!(entity.parent_ref().unwrap().as_str(), "region_001");
    assert_eq!(entity.field("parent").unwrap().value, "region_001");
}

#[test]
fn parent_optional() {
    let src = "@domain\n\n#header\n  id:\n    d1\n  title:\n    D\n#body\n  description:\n    hi\n";
    let entity = parse_to_model(src);
    assert!(entity.parent_ref().is_none());
}

// ---------------------------------------------------------------------------
// file
// ---------------------------------------------------------------------------

#[test]
fn file_reference() {
    let src = "@frame\n\n#header\n  id:\n    frame_001\n  title:\n    Foo\n#body\n  file:\n    file_001\n  target:\n    Foo\n  type:\n    class\n";
    let entity = parse_to_model(src);
    assert_eq!(entity.file_ref().unwrap().as_str(), "file_001");
    assert_eq!(entity.level, Level::Frame);
}

#[test]
fn file_reference_not_resolved() {
    let src = "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    does_not_exist\n  target:\n    T\n  type:\n    function\n";
    let entity = parse_to_model(src);
    // Should still preserve the reference even if target doesn't exist — no validation in Step 3
    assert_eq!(entity.file_ref().unwrap().as_str(), "does_not_exist");
}

// ---------------------------------------------------------------------------
// relations — zero / one / multiple
// ---------------------------------------------------------------------------

#[test]
fn relations_zero() {
    let src = include_str!("../fixtures/parser/valid/empty_relations.pmap");
    let entity = parse_to_model(src);
    assert_eq!(entity.relations.len(), 0);
}

#[test]
fn relations_one() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        depends_on\n      from:\n        n1\n      to:\n        n2\n";
    let entity = parse_to_model(src);
    assert_eq!(entity.relations.len(), 1);
    assert_eq!(entity.relations[0].get_value("type"), Some("depends_on"));
}

#[test]
fn relations_multiple() {
    let src = include_str!("../fixtures/parser/valid/node_with_relations.pmap");
    let entity = parse_to_model(src);
    assert_eq!(entity.relations.len(), 2);
    assert_eq!(entity.relations[0].get_value("type"), Some("depends_on"));
    assert_eq!(entity.relations[1].get_value("type"), Some("uses"));
}

// ---------------------------------------------------------------------------
// relation fields
// ---------------------------------------------------------------------------

#[test]
fn relation_fields_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        uses\n      from:\n        n1\n      to:\n        file_010\n      evidence:\n        src/pay.rs:88\n";
    let entity = parse_to_model(src);
    let rel = &entity.relations[0];
    assert_eq!(rel.get_value("type"), Some("uses"));
    assert_eq!(rel.get_value("from"), Some("n1"));
    assert_eq!(rel.get_value("to"), Some("file_010"));
    assert_eq!(rel.get_value("evidence"), Some("src/pay.rs:88"));
}

#[test]
fn relation_optional_fields() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        depends_on\n      from:\n        n1\n      to:\n        n2\n";
    let entity = parse_to_model(src);
    let rel = &entity.relations[0];
    // evidence is optional, should be None when absent
    assert!(rel.get("evidence").is_none());
    // required fields present
    assert!(rel.get("type").is_some());
    assert!(rel.get("from").is_some());
    assert!(rel.get("to").is_some());
}

#[test]
fn relation_from_to_remain_references() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        some_id\n      to:\n        other_id\n";
    let entity = parse_to_model(src);
    let rel = &entity.relations[0];
    let from = rel.from_ref().unwrap();
    let to = rel.to_ref().unwrap();
    assert_eq!(from.as_str(), "some_id");
    assert_eq!(to.as_str(), "other_id");
    // Not resolved to entities
    assert_eq!(rel.get_value("from"), Some("some_id"));
}

// ---------------------------------------------------------------------------
// Source spans / provenance
// ---------------------------------------------------------------------------

#[test]
fn source_spans_preserved() {
    let src = "@frame\n\n#header\n  id:\n    frame_001\n  title:\n    T\n\n#body\n  file:\n    file_001\n  target:\n    T\n  type:\n    class\n";
    let entity = parse_to_model(src);
    // entity span starts at line 1
    assert_eq!(entity.span.start_line, 1);
    assert_eq!(entity.level_span.start_line, 1);
    // header field spans
    let id_field = entity.header.iter().find(|f| f.name == "id").unwrap();
    assert_eq!(id_field.span.start_line, 4);
    // body field spans
    let file_field = entity.body.iter().find(|f| f.name == "file").unwrap();
    assert!(file_field.span.start_line >= 9);
    // relation spans
    let src2 = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n";
    let entity2 = parse_to_model(src2);
    assert_eq!(entity2.relations[0].span.start_line, 12);
}

#[test]
fn header_body_spans_distinct() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n";
    let entity = parse_to_model(src);
    let header_id = entity.header.iter().find(|f| f.name == "id").unwrap();
    let body_parent = entity.body.iter().find(|f| f.name == "parent").unwrap();
    assert!(header_id.span.start_line < body_parent.span.start_line);
}

// ---------------------------------------------------------------------------
// Unknown-field preservation
// ---------------------------------------------------------------------------

#[test]
fn unknown_fields_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  custom_field:\n    hello\n  another_unknown:\n    world\n";
    let entity = parse_to_model(src);
    assert!(entity.has_field("custom_field"));
    assert_eq!(entity.field("custom_field").unwrap().value, "hello");
    assert!(entity.has_field("another_unknown"));
    assert_eq!(entity.field("another_unknown").unwrap().value, "world");
}

#[test]
fn unknown_field_in_header_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n  custom_header:\n    foo\n#body\n  parent:\n    d1\n";
    let entity = parse_to_model(src);
    assert!(entity.header.iter().any(|f| f.name == "custom_header"));
    assert_eq!(
        entity
            .header
            .iter()
            .find(|f| f.name == "custom_header")
            .unwrap()
            .value,
        "foo"
    );
}

#[test]
fn unknown_field_in_relation_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n      custom_rel_field:\n        preserved\n";
    let entity = parse_to_model(src);
    let rel = &entity.relations[0];
    assert!(rel.get("custom_rel_field").is_some());
    assert_eq!(rel.get_value("custom_rel_field"), Some("preserved"));
}

#[test]
fn unknown_fields_not_discarded_for_any_level() {
    for level in ["domain", "region", "file"] {
        let src = match level {
            "file" => "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/x.rs\n#body\n  custom:\n    val\n"
                .to_string(),
            _ => format!(
                "@{level}\n\n#header\n  id:\n    id1\n  title:\n    T\n#body\n  parent:\n    p1\n  custom:\n    val\n"
            ),
        };
        let entity = parse_to_model(&src);
        assert!(
            entity.has_field("custom"),
            "unknown field should be preserved for level {level}"
        );
    }
}

// ---------------------------------------------------------------------------
// Entity ownership — one .pmap = one entity
// ---------------------------------------------------------------------------

#[test]
fn entity_ownership_one_pmap_one_entity() {
    let src = "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/main.rs\n#body\n  hash:\n    abc\n";
    let entity = parse_to_model(src);
    assert_eq!(entity.level, Level::File);
    assert_eq!(entity.id().unwrap().as_str(), "file_001");
    // entity represents exactly one level, not multiple
    assert!(matches!(entity.level, Level::File));
}

#[test]
fn file_owns_file_info_frame_owns_target() {
    let file_src =
        "@file\n\n#header\n  id:\n    file_001\n  path:\n    src/lib.rs\n#body\n  hash:\n    h1\n";
    let file_entity = parse_to_model(file_src);
    assert!(file_entity.field("path").is_some());
    assert_eq!(file_entity.field("path").unwrap().value, "src/lib.rs");

    let frame_src = "@frame\n\n#header\n  id:\n    frame_001\n  title:\n    Foo\n#body\n  file:\n    file_001\n  target:\n    Foo\n  type:\n    class\n";
    let frame_entity = parse_to_model(frame_src);
    assert!(frame_entity.field("target").is_some());
    assert_eq!(frame_entity.field("target").unwrap().value, "Foo");
    assert!(frame_entity.file_ref().is_some());
}

// ---------------------------------------------------------------------------
// No reference resolution during model conversion
// ---------------------------------------------------------------------------

#[test]
fn no_reference_resolution() {
    let src = "@frame\n\n#header\n  id:\n    f1\n  title:\n    T\n#body\n  file:\n    nonexistent_file\n  target:\n    T\n  type:\n    class\n  parent:\n    nonexistent_parent\n";
    let entity = parse_to_model(src);
    // References are kept as plain IDs, even if they don't exist
    assert_eq!(entity.file_ref().unwrap().as_str(), "nonexistent_file");
    assert_eq!(entity.parent_ref().unwrap().as_str(), "nonexistent_parent");
    // No panic, no validation error, just preserved
}

#[test]
fn relations_not_resolved_to_entities() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        depends_on\n      from:\n        unknown_id\n      to:\n        another_unknown\n";
    let entity = parse_to_model(src);
    let rel = &entity.relations[0];
    // from/to are still plain strings, not resolved objects
    assert_eq!(rel.from_ref().unwrap().as_str(), "unknown_id");
    assert_eq!(rel.to_ref().unwrap().as_str(), "another_unknown");
    // No graph built
    assert_eq!(entity.relations.len(), 1);
}

// ---------------------------------------------------------------------------
// Structural invariants
// ---------------------------------------------------------------------------

#[test]
fn structural_invariants_hold() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  relations:\n    @relation\n      type:\n        a\n      from:\n        n1\n      to:\n        n2\n";
    let entity = parse_to_model(src);
    assert!(entity.invariant_one_level_per_entity());
    assert!(entity.invariant_level_is_closed_set());
    assert!(entity.invariant_relations_not_levels());
    assert!(Level::all().contains(&entity.level.as_str()));
}

// ---------------------------------------------------------------------------
// Header/body separation
// ---------------------------------------------------------------------------

#[test]
fn header_body_separation_preserved() {
    let src = "@node\n\n#header\n  id:\n    n1\n  title:\n    N\n#body\n  parent:\n    d1\n  description:\n    hello\n";
    let entity = parse_to_model(src);
    assert!(entity.header.iter().any(|f| f.name == "id"));
    assert!(entity.header.iter().any(|f| f.name == "title"));
    assert!(entity.body.iter().any(|f| f.name == "parent"));
    assert!(entity.body.iter().any(|f| f.name == "description"));
    // header should not contain body fields and vice versa
    assert!(!entity.header.iter().any(|f| f.name == "parent"));
    assert!(!entity.body.iter().any(|f| f.name == "id"));
}

// ---------------------------------------------------------------------------
// Using fixtures — ensure parser fixtures convert cleanly
// ---------------------------------------------------------------------------

#[test]
fn fixtures_convert_to_model() {
    for name in [
        "minimal_aimt.pmap",
        "file_no_parent.pmap",
        "node_with_relations.pmap",
    ] {
        let path = format!(
            "{}/tests/fixtures/parser/valid/{}",
            env!("CARGO_MANIFEST_DIR"),
            name
        );
        let content = std::fs::read_to_string(&path).unwrap();
        let entity = AimtEntity::parse_and_convert(&content).unwrap();
        assert!(entity.id().is_some(), "fixture {name} should have id");
    }
}
