use aimt::syntax::{
    BODY_MARKER, HEADER_MARKER, INDENT_WIDTH, RELATION_MARKER, RELATIONS_FIELD, SECTION_MARKERS,
    SUPPORTED_LEVEL_MARKERS, SUPPORTED_LEVELS, indent_for_depth, is_lowercase_name,
    is_supported_level, is_supported_level_marker, is_supported_section, is_tab_present,
    is_valid_indent,
};

#[test]
fn supported_levels_count_is_seven() {
    assert_eq!(SUPPORTED_LEVELS.len(), 7);
    assert_eq!(SUPPORTED_LEVEL_MARKERS.len(), 7);
}

#[test]
fn supported_levels_exact_set() {
    let expected = ["aimt", "map", "domain", "region", "node", "file", "frame"];
    assert_eq!(SUPPORTED_LEVELS, &expected);
}

#[test]
fn supported_level_markers_exact_set() {
    let expected = [
        "@aimt", "@map", "@domain", "@region", "@node", "@file", "@frame",
    ];
    assert_eq!(SUPPORTED_LEVEL_MARKERS, &expected);
}

#[test]
fn lowercase_level_names() {
    for level in SUPPORTED_LEVELS {
        assert_eq!(
            *level,
            level.to_ascii_lowercase(),
            "level {level} must be lowercase"
        );
        assert!(
            is_lowercase_name(level),
            "level {level} must match lowercase name shape"
        );
    }
    for marker in SUPPORTED_LEVEL_MARKERS {
        let bare = marker.trim_start_matches('@');
        assert_eq!(bare, bare.to_ascii_lowercase());
    }
}

#[test]
fn no_uppercase_levels_accepted() {
    assert!(!is_supported_level("AIMT"));
    assert!(!is_supported_level("Map"));
    assert!(!is_supported_level("FRAME"));
    assert!(!is_supported_level_marker("@AIMT"));
    assert!(!is_supported_level_marker("@Frame"));
}

#[test]
fn header_and_body_markers() {
    assert_eq!(HEADER_MARKER, "#header");
    assert_eq!(BODY_MARKER, "#body");
}

#[test]
fn supported_sections_are_header_then_body() {
    assert_eq!(SECTION_MARKERS, &["#header", "#body"]);
    assert_eq!(SECTION_MARKERS[0], HEADER_MARKER);
    assert_eq!(SECTION_MARKERS[1], BODY_MARKER);
}

#[test]
fn required_section_order() {
    assert!(is_supported_section("#header"));
    assert!(is_supported_section("#body"));
    assert!(!is_supported_section("#Header"));
    assert!(!is_supported_section("#BODY"));
    assert!(!is_supported_section("# header"));
    let header_idx = SECTION_MARKERS
        .iter()
        .position(|s| *s == "#header")
        .unwrap();
    let body_idx = SECTION_MARKERS.iter().position(|s| *s == "#body").unwrap();
    assert!(header_idx < body_idx, "#header must precede #body");
}

#[test]
fn indent_width_is_two() {
    assert_eq!(INDENT_WIDTH, 2);
}

#[test]
fn indent_validation_multiples_of_two() {
    for valid in [0, 2, 4, 6, 8, 10] {
        assert!(is_valid_indent(valid), "{valid} should be valid");
    }
    for invalid in [1, 3, 5, 7, 9] {
        assert!(!is_valid_indent(invalid), "{invalid} should be invalid");
    }
    assert_eq!(indent_for_depth(0), 0);
    assert_eq!(indent_for_depth(1), 2);
    assert_eq!(indent_for_depth(2), 4);
    assert_eq!(indent_for_depth(3), 6);
    assert_eq!(indent_for_depth(4), 8);
}

#[test]
fn tabs_are_invalid() {
    assert!(is_tab_present("\t"));
    assert!(is_tab_present("  \t  "));
    assert!(is_tab_present("a\tb"));
    assert!(!is_tab_present("  "));
    assert!(!is_tab_present("    valid"));
    assert!(!is_tab_present("  id:"));
}

#[test]
fn canonical_relations_field() {
    assert_eq!(RELATIONS_FIELD, "relations");
    assert_ne!(RELATIONS_FIELD, "relation");
    assert!(is_lowercase_name(RELATIONS_FIELD));
}

#[test]
fn relation_marker() {
    assert_eq!(RELATION_MARKER, "@relation");
    assert!(!is_supported_level("relation"));
    assert!(!is_supported_level_marker("@relation"));
    assert!(!is_supported_level_marker(RELATION_MARKER));
}

#[test]
fn no_singular_relation_in_supported_constants() {
    assert!(!SUPPORTED_LEVELS.contains(&"relation"));
    assert!(!SUPPORTED_LEVEL_MARKERS.contains(&"@relation"));
    assert!(!SUPPORTED_LEVEL_MARKERS.contains(&"relation"));
    assert!(!SECTION_MARKERS.contains(&"relation"));
    assert!(!SECTION_MARKERS.contains(&"@relation"));
    assert_ne!(RELATIONS_FIELD, "relation");
    assert_ne!(RELATION_MARKER, "@relations");
    assert_ne!(RELATION_MARKER, "relation");
}

#[test]
fn no_unsupported_levels() {
    let unsupported = [
        "@relation",
        "@unknown",
        "@project",
        "@component",
        "aimt ",
        "@aimt extra",
        "",
        "@",
    ];
    for m in unsupported {
        assert!(
            !is_supported_level_marker(m),
            "marker {m:?} must not be accepted"
        );
    }
    let unsupported_bare = ["relation", "unknown", "project", "AIMT", "FILE", ""];
    for n in unsupported_bare {
        assert!(!is_supported_level(n), "level {n:?} must not be accepted");
    }
}

#[test]
fn canonical_syntax_constants_are_distinct() {
    assert_ne!(HEADER_MARKER, BODY_MARKER);
    assert_ne!(RELATIONS_FIELD, RELATION_MARKER);
    assert_ne!(HEADER_MARKER, RELATION_MARKER);
    assert!(!SUPPORTED_LEVELS.contains(&RELATIONS_FIELD));
    assert!(!SUPPORTED_LEVELS.contains(&HEADER_MARKER));
}

#[test]
fn lowercase_name_shape() {
    assert!(is_lowercase_name("id"));
    assert!(is_lowercase_name("frame_001"));
    assert!(is_lowercase_name("relations"));
    assert!(!is_lowercase_name("Relation"));
    assert!(!is_lowercase_name("relations-field"));
    assert!(!is_lowercase_name("_id"));
    assert!(!is_lowercase_name("1id"));
    assert!(!is_lowercase_name(""));
}

#[test]
fn all_seven_levels_individually_supported() {
    for level in ["aimt", "map", "domain", "region", "node", "file", "frame"] {
        assert!(is_supported_level(level), "{level} should be supported");
        let marker = format!("@{level}");
        assert!(
            is_supported_level_marker(&marker),
            "{marker} should be supported"
        );
    }
}

#[test]
fn marker_derives_from_canonical_levels() {
    // Behavior derives from SUPPORTED_LEVELS, not an independent list.
    for level in SUPPORTED_LEVELS {
        let marker = format!("@{level}");
        assert!(is_supported_level_marker(&marker));
        assert!(SUPPORTED_LEVEL_MARKERS.contains(&marker.as_str()));
    }
    // Marker without @ or with wrong case must not derive to supported.
    assert!(!is_supported_level_marker("aimt"));
    assert!(!is_supported_level_marker("@Aimt"));
    assert!(!is_supported_level_marker("@aimt "));
}

#[test]
fn indentation_boundaries_and_depth() {
    // pMap §6.2 depths 0..4
    assert_eq!(indent_for_depth(0), 0);
    assert_eq!(indent_for_depth(1), 2);
    assert_eq!(indent_for_depth(2), 4);
    assert_eq!(indent_for_depth(3), 6);
    assert_eq!(indent_for_depth(4), 8);
    assert_eq!(indent_for_depth(5), 10);
    // Unit check only — divisible by 2, not depth validation
    assert!(is_valid_indent(0));
    assert!(is_valid_indent(2));
    assert!(is_valid_indent(8));
    assert!(!is_valid_indent(1));
    assert!(!is_valid_indent(3));
    // is_valid_indent does not validate depth context
    assert!(is_valid_indent(12));
    assert!(is_valid_indent(100));
}

#[test]
fn tabs_and_lowercase_helpers_are_generic() {
    // is_tab_present is generic tab detection
    assert!(is_tab_present("\t"));
    assert!(is_tab_present("a\tb"));
    assert!(!is_tab_present("    "));
    // is_lowercase_name is shape-only, not support check
    assert!(is_lowercase_name("custom_field"));
    assert!(!is_supported_level("custom_field"));
    assert!(is_lowercase_name("aimt"));
    assert!(is_supported_level("aimt"));
    // Invalid name shapes
    for invalid in [
        "",
        "_id",
        "1id",
        "Relation",
        "relations-field",
        "id with space",
        "ID",
    ] {
        assert!(
            !is_lowercase_name(invalid),
            "{invalid:?} should be invalid shape"
        );
    }
    for valid in ["id", "title", "file_001", "relations", "a"] {
        assert!(is_lowercase_name(valid), "{valid:?} should be valid shape");
    }
}
