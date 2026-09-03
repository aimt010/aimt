/// Indentation unit in spaces. `pmap.md` §6 requires exactly 2.
pub const INDENT_WIDTH: usize = 2;

/// Section markers at column 0. `pmap.md` §5.
pub const HEADER_MARKER: &str = "#header";
pub const BODY_MARKER: &str = "#body";

/// Supported sections in required order: `#header` then `#body`.
pub const SECTION_MARKERS: &[&str] = &[HEADER_MARKER, BODY_MARKER];

/// Canonical wrapper field for nested relations (plural).
pub const RELATIONS_FIELD: &str = "relations";

/// Nested relation marker, not a level. `pmap.md` §9.
pub const RELATION_MARKER: &str = "@relation";

/// Returns true if `marker` is a supported section marker.
pub fn is_supported_section(marker: &str) -> bool {
    SECTION_MARKERS.contains(&marker)
}

/// Returns true if `indent` is divisible by the 2-space unit (`INDENT_WIDTH`).
pub fn is_valid_indent(indent: usize) -> bool {
    indent.is_multiple_of(INDENT_WIDTH)
}

/// Indent in spaces for a logical depth. `pmap.md` §6.2:
/// depth 0 → 0, 1 → 2, 2 → 4, 3 → 6, 4 → 8.
pub fn indent_for_depth(depth: usize) -> usize {
    depth * INDENT_WIDTH
}

/// Returns true if `s` contains a tab character, which is invalid anywhere
/// in a `.pmap` file. `pmap.md` §6.1 and §10.1.
pub fn is_tab_present(s: &str) -> bool {
    s.contains('\t')
}

/// Returns true if `s` matches the generic lowercase name shape
/// (`[a-z][a-z0-9_]*`). This is a shape helper only; it does NOT
/// determine whether a field or level is actually supported — use
/// `is_supported_level` / `is_supported_level_marker` for that.
/// `pmap.md` §7.1 and `fields.md` §2.
pub fn is_lowercase_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => (),
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
