use super::error::{ErrorKind, ParseError};
use super::line::LineInfo;
use super::types::{ParsedField, ParsedPmap, ParsedRelation};
use crate::syntax::{
    BODY_MARKER, HEADER_MARKER, INDENT_WIDTH, RELATION_MARKER, RELATIONS_FIELD, indent_for_depth,
    is_lowercase_name, is_supported_level, is_supported_level_marker, is_tab_present,
    is_valid_indent,
};
use crate::syntax::{Level as ParsedLevel, Span};

// Public API
// ---------------------------------------------------------------------------

/// Parse `&str` `.pmap` text. Validates UTF-8, BOM, newlines, tabs.
pub fn parse(input: &str) -> Result<ParsedPmap, ParseError> {
    parse_normalized(input, false)
}

/// Parse `&[u8]` as UTF-8 `.pmap` text.
pub fn parse_bytes(input: &[u8]) -> Result<ParsedPmap, ParseError> {
    let s = std::str::from_utf8(input).map_err(|_| {
        ParseError::new(ErrorKind::InvalidUtf8, Span::single(1, 1), "invalid UTF-8")
    })?;
    parse_normalized(s, false)
}

fn parse_normalized(input: &str, _is_bytes: bool) -> Result<ParsedPmap, ParseError> {
    // BOM
    if input.starts_with('\u{FEFF}') {
        return Err(ParseError::new(
            ErrorKind::BomPresent,
            Span::single(1, 1),
            "UTF-8 BOM not allowed",
        ));
    }
    // lone CR and CRLF handling
    if input.contains('\r') {
        // check each \r
        let chars: Vec<char> = input.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '\r' && (i + 1 >= chars.len() || chars[i + 1] != '\n') {
                // find line/col for this \r
                let prefix: String = chars[..i].iter().collect();
                let line = prefix.matches('\n').count() + 1;
                let col = prefix
                    .rsplit('\n')
                    .next()
                    .map(|s| s.chars().count() + 1)
                    .unwrap_or(1);
                return Err(ParseError::new(
                    ErrorKind::LoneCr,
                    Span::single(line, col),
                    "lone CR not allowed",
                ));
            }
        }
    }
    // tab present anywhere -> report first occurrence
    if is_tab_present(input) {
        // find first tab line/col
        let mut line = 1usize;
        let mut col = 1usize;
        for ch in input.chars() {
            if ch == '\t' {
                return Err(ParseError::new(
                    ErrorKind::TabPresent,
                    Span::single(line, col),
                    "tab not allowed",
                ));
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
                if ch == '\r' {
                    // will be normalized
                }
            }
        }
    }
    // normalize CRLF -> LF
    let normalized = input.replace("\r\n", "\n");
    // trailing newline check: must end with single LF
    if normalized.is_empty() {
        return Err(ParseError::new(
            ErrorKind::MissingLevel,
            Span::single(1, 1),
            "missing @level",
        ));
    }
    if !normalized.ends_with('\n') {
        let lines = normalized.matches('\n').count() + 1;
        return Err(ParseError::new(
            ErrorKind::MissingTrailingNewline,
            Span::single(lines, 1),
            "missing trailing newline",
        ));
    }

    // Split into lines, preserving blank handling.
    // normalized ends with \n, so split will have trailing "" element.
    let raw_lines: Vec<&str> = normalized.split('\n').collect();
    // raw_lines last element is "" after final \n
    let mut lines: Vec<LineInfo> = Vec::new();
    for (idx, raw) in raw_lines.iter().enumerate() {
        let line_no = idx + 1;
        // raw is without \n
        let trimmed_trailing = raw.trim_end_matches(' ');
        let has_trailing_space = *raw != trimmed_trailing;
        let indent = trimmed_trailing.chars().take_while(|c| *c == ' ').count();
        let content = trimmed_trailing[indent..].to_string();
        let is_blank = content.is_empty();
        // Validate indent is multiple of 2 for non-blank lines
        if !is_blank && !is_valid_indent(indent) {
            return Err(ParseError::new(
                ErrorKind::InvalidIndent,
                Span::single(line_no, 1),
                format!("indent {indent} not multiple of {}", INDENT_WIDTH),
            ));
        }
        lines.push(LineInfo {
            line_no,
            indent,
            content,
            is_blank,
            has_trailing_space,
        });
        // Stop after processing the line that was the final ""? That "" corresponds to line after trailing \n.
        // We want lines to include that final empty line as trailing blank.
        // Actually raw_lines includes extra "" at end; we already handle.
    }

    // TooManyTrailingBlankLines: more than one trailing blank line at EOF (beyond one allowed)
    // raw_lines: e.g., "a\n" => ["a",""], one trailing blank (the "" after \n) is not considered blank for this check?
    // But spec says file must end with single LF, and up to one trailing blank line allowed.
    // A file ending with "\n" has one trailing blank entry "" that is just EOF marker, not a blank line.
    // A file ending with "\n\n" has raw_lines ["...", "", ""] -> two trailing empty entries => one blank line at EOF.
    // A file ending with "\n\n\n" => ["...","","",""] => two blank lines at EOF -> error.
    // Let's count consecutive blank lines at end (excluding the final EOF ""?).
    // Simplify: Count blank lines at end of lines vector before the final EOF marker.
    // Our lines vector includes the final "" as a blank line at last line_no.
    // For normalized ending with "\n", lines.last() is blank (the "" after). For "\n\n", lines has two trailing blanks.
    let mut trailing_blanks = 0usize;
    for li in lines.iter().rev() {
        if li.is_blank {
            trailing_blanks += 1;
        } else {
            break;
        }
    }
    // One trailing blank is the mandatory EOF empty after final \n, so allowed trailing_blanks is 1 or 2?
    // "\n" => trailing_blanks =1 (the EOF ""), allowed.
    // "\n\n" => trailing_blanks =2 (one blank line + EOF ""), allowed per 10.5.
    // "\n\n\n" => trailing_blanks =3 -> error.
    if trailing_blanks > 2 {
        return Err(ParseError::new(
            ErrorKind::TooManyTrailingBlankLines,
            Span::single(lines.len() - trailing_blanks + 1, 1),
            "too many trailing blank lines",
        ));
    }

    // Leading blank lines: up to one allowed before @level
    let mut leading_blanks = 0usize;
    for li in &lines {
        if li.is_blank {
            leading_blanks += 1;
        } else {
            break;
        }
    }
    if leading_blanks > 1 {
        return Err(ParseError::new(
            ErrorKind::TooManyLeadingBlankLines,
            Span::single(1, 1),
            "too many leading blank lines",
        ));
    }

    // Now parse structure
    parse_structure(lines, normalized)
}

fn parse_structure(
    mut lines: Vec<LineInfo>,
    _normalized: String,
) -> Result<ParsedPmap, ParseError> {
    // Remove the final EOF blank (the last "" after trailing \n) from consideration for structure,
    // but keep one potential trailing blank line inside bounds.
    // Our lines includes final "" as blank. For structure, we want to treat EOF as not a line.
    // If trailing_blanks ==2, the second last is the allowed trailing blank, last is EOF.
    // We'll pop the final EOF empty line.
    if let Some(last) = lines.last()
        && last.is_blank
        && last.line_no == lines.len()
    {
        // This is the EOF marker from split; remove it for parsing
        lines.pop();
    }

    // Now lines may end with one blank line (allowed trailing). Keep it for blank handling but it will be ignored.

    // Find first non-blank line index
    let mut idx = 0usize;
    // skip leading blank (0 or 1)
    if idx < lines.len() && lines[idx].is_blank {
        idx += 1;
    }
    if idx >= lines.len() {
        return Err(ParseError::new(
            ErrorKind::MissingLevel,
            Span::single(1, 1),
            "missing @level",
        ));
    }

    // Level line must be at indent 0
    let level_line = &lines[idx];
    if level_line.indent != 0 {
        return Err(ParseError::new(
            ErrorKind::InvalidLevelIndent,
            Span::single(level_line.line_no, 1),
            "level must be at column 0",
        ));
    }
    if level_line.has_trailing_space {
        return Err(ParseError::new(
            ErrorKind::TrailingContentAfterLevel,
            Span::single(level_line.line_no, 1),
            "trailing content after level",
        ));
    }
    // Must start with @
    if !level_line.content.starts_with('@') {
        return Err(ParseError::new(
            ErrorKind::MissingLevel,
            Span::single(level_line.line_no, 1),
            "missing @level",
        ));
    }
    // Check for trailing content after marker: content must be exactly marker
    // Validate shape: bare after @ must be lowercase name and supported
    let bare = &level_line.content[1..];
    // Check if content contains space or extra after bare
    if bare.contains(' ') || bare.contains('\t') {
        return Err(ParseError::new(
            ErrorKind::TrailingContentAfterLevel,
            Span::single(level_line.line_no, 1),
            "trailing content after level",
        ));
    }
    // Check lowercase shape first
    if !is_lowercase_name(bare) {
        // Determine if it's case issue vs unknown
        if bare.to_ascii_lowercase() != bare {
            return Err(ParseError::new(
                ErrorKind::InvalidLevelCase,
                Span::single(level_line.line_no, 1),
                "level must be lowercase",
            ));
        } else {
            return Err(ParseError::new(
                ErrorKind::UnknownLevel,
                Span::single(level_line.line_no, 1),
                format!("unknown level {bare}"),
            ));
        }
    }
    if !is_supported_level(bare) {
        return Err(ParseError::new(
            ErrorKind::UnknownLevel,
            Span::single(level_line.line_no, 1),
            format!("unknown level {bare}"),
        ));
    }
    // Also validate via syntax helper for marker
    if !is_supported_level_marker(&level_line.content) {
        return Err(ParseError::new(
            ErrorKind::UnknownLevel,
            Span::single(level_line.line_no, 1),
            "unknown level marker",
        ));
    }

    let level = ParsedLevel::from_bare(bare).unwrap();
    let level_marker = level_line.content.clone();
    idx += 1;

    // Helper to count blank lines between sections
    let mut blank_between_level_header = 0usize;
    while idx < lines.len() && lines[idx].is_blank {
        blank_between_level_header += 1;
        idx += 1;
    }
    if blank_between_level_header > 1 {
        return Err(ParseError::new(
            ErrorKind::TooManyBlankLinesBetweenSections,
            Span::single(lines[idx - 1].line_no, 1),
            "too many blank lines between level and #header",
        ));
    }

    if idx >= lines.len() {
        return Err(ParseError::new(
            ErrorKind::MissingSection,
            Span::single(level_line.line_no, 1),
            "missing #header",
        ));
    }
    // Expect #header
    let hdr_line = &lines[idx];
    if hdr_line.has_trailing_space {
        return Err(ParseError::new(
            ErrorKind::InvalidSection,
            Span::single(hdr_line.line_no, 1),
            "trailing content after section marker",
        ));
    }
    if hdr_line.content != HEADER_MARKER {
        if hdr_line.content == BODY_MARKER {
            return Err(ParseError::new(
                ErrorKind::WrongSectionOrder,
                Span::single(hdr_line.line_no, 1),
                "expected #header before #body",
            ));
        }
        if hdr_line.content.starts_with('#') {
            return Err(ParseError::new(
                ErrorKind::InvalidSection,
                Span::single(hdr_line.line_no, 1),
                format!("invalid section {}", hdr_line.content),
            ));
        }
        if hdr_line.content.starts_with('@') {
            // second level
            return Err(ParseError::new(
                ErrorKind::MultipleLevels,
                Span::single(hdr_line.line_no, 1),
                "multiple @level not allowed",
            ));
        }
        return Err(ParseError::new(
            ErrorKind::MissingSection,
            Span::single(hdr_line.line_no, 1),
            "missing #header",
        ));
    }
    if hdr_line.indent != 0 {
        return Err(ParseError::new(
            ErrorKind::InvalidSection,
            Span::single(hdr_line.line_no, 1),
            "section marker must be at column 0",
        ));
    }
    let header_start = idx;
    idx += 1;

    let mut header_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut found_body = false;

    // Scan for #body marker with blank handling
    let mut scan_idx = idx;
    let mut blank_count = 0usize;
    let mut header_block_end = scan_idx;
    while scan_idx < lines.len() {
        let li = &lines[scan_idx];
        if li.is_blank {
            blank_count += 1;
            if blank_count > 1 {
                // This is inside header/body blank handling; but for section separation, >1 blank between blocks is error
                // We need to know if we are between sections (after header fields) and before body
                // For now, we will detect later
            }
            scan_idx += 1;
            continue;
        }
        if li.has_trailing_space && (li.content == HEADER_MARKER || li.content == BODY_MARKER) {
            return Err(ParseError::new(
                ErrorKind::InvalidSection,
                Span::single(li.line_no, 1),
                "trailing content after section marker",
            ));
        }
        if li.indent == 0 && li.content == BODY_MARKER {
            header_block_end = scan_idx; // index of #body
            // check blank_count between header and body
            if blank_count > 1 {
                return Err(ParseError::new(
                    ErrorKind::TooManyBlankLinesBetweenSections,
                    Span::single(li.line_no - 1, 1),
                    "too many blank lines between #header and #body",
                ));
            }
            found_body = true;
            break;
        }
        if li.indent == 0 && li.content == HEADER_MARKER {
            // duplicate header
            return Err(ParseError::new(
                ErrorKind::DuplicateSection,
                Span::single(li.line_no, 1),
                "duplicate #header",
            ));
        }
        if li.indent == 0 && li.content.starts_with('@') {
            return Err(ParseError::new(
                ErrorKind::MultipleLevels,
                Span::single(li.line_no, 1),
                "multiple @level not allowed",
            ));
        }
        if li.indent == 0 && li.content.starts_with('#') {
            // "# comment" etc should be UnexpectedNode (comments not allowed), not InvalidSection
            if li.content != HEADER_MARKER && li.content != BODY_MARKER {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedNode,
                    Span::single(li.line_no, 1),
                    format!("unexpected content at column 0: {}", li.content),
                ));
            }
            return Err(ParseError::new(
                ErrorKind::InvalidSection,
                Span::single(li.line_no, 1),
                format!("invalid section {}", li.content),
            ));
        }
        // Field at wrong indent (e.g., "id:" at 0) should be InvalidIndent, not UnexpectedNode — defer to field parser
        if li.indent == 0 && li.content.contains(':') {
            let name_part = li.content.split(':').next().unwrap_or("");
            if !name_part.is_empty() && is_lowercase_name(name_part) {
                // treat as field with wrong indent; let parse_fields report InvalidIndent
                blank_count = 0;
                scan_idx += 1;
                continue;
            }
        }
        // If we encounter a line at indent 0 that is not body/header/@level, it's unexpected node (e.g., bare text at 0)
        if li.indent == 0 {
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                format!("unexpected content at column 0: {}", li.content),
            ));
        }
        // otherwise it's a field line at indent 2 (or blank)
        // For header block, we expect indent 2 for field names
        if li.indent != indent_for_depth(1) && !li.is_blank {
            // Could be value line at 4 inside header field value block, but we are scanning for body marker boundaries,
            // So we should not treat value lines as block end. We need to distinguish: we are scanning for #body marker, but header fields' value blocks are at 4.
            // So we should just continue scanning; any line at indent 4 is part of previous field's value, not a new block.
            // So we need to not treat indent 4 as start of new field for section boundary detection.
            // Continue.
        }
        blank_count = 0;
        scan_idx += 1;
    }

    if !found_body {
        return Err(ParseError::new(
            ErrorKind::MissingSection,
            Span::single(header_start + 1, 1),
            "missing #body",
        ));
    }

    let body_idx = header_block_end;
    let body_line = &lines[body_idx];
    // body at 0 already checked

    // Now parse header fields: slice lines[header_start+1 .. body_idx)
    let header_slice = &lines[header_start + 1..body_idx];
    let header_fields = parse_fields(header_slice, 1, &mut header_seen, true)?;

    // Check for duplicate body etc after body
    // Now parse body fields: slice lines[body_idx+1 .. end)
    let body_slice = &lines[body_idx + 1..];
    // Need to check for extra sections after body: duplicate #header/#body or second level
    for li in body_slice {
        if li.is_blank {
            continue;
        }
        if li.has_trailing_space && (li.content == HEADER_MARKER || li.content == BODY_MARKER) {
            return Err(ParseError::new(
                ErrorKind::InvalidSection,
                Span::single(li.line_no, 1),
                "trailing content after section marker",
            ));
        }
        if li.indent == 0 {
            if li.content == HEADER_MARKER || li.content == BODY_MARKER {
                return Err(ParseError::new(
                    ErrorKind::DuplicateSection,
                    Span::single(li.line_no, 1),
                    format!("duplicate section {}", li.content),
                ));
            }
            if li.content.starts_with('@') {
                return Err(ParseError::new(
                    ErrorKind::MultipleLevels,
                    Span::single(li.line_no, 1),
                    "multiple @level not allowed",
                ));
            }
            if li.content.starts_with('#') {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedNode,
                    Span::single(li.line_no, 1),
                    format!("unexpected content at column 0: {}", li.content),
                ));
            }
            if li.content.starts_with("//") || li.content.starts_with('/') {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedNode,
                    Span::single(li.line_no, 1),
                    "comments not allowed",
                ));
            }
            // Field at wrong indent (e.g., "id:" at 0) should be InvalidIndent, not UnexpectedNode
            if li.content.contains(':') {
                let name_part = li.content.split(':').next().unwrap_or("");
                if !name_part.is_empty() && is_lowercase_name(name_part) {
                    // defer to field parser for precise InvalidIndent
                } else {
                    return Err(ParseError::new(
                        ErrorKind::UnexpectedNode,
                        Span::single(li.line_no, 1),
                        format!("unexpected content at column 0: {}", li.content),
                    ));
                }
            } else {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedNode,
                    Span::single(li.line_no, 1),
                    format!("unexpected content at column 0: {}", li.content),
                ));
            }
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                format!("unexpected content at column 0: {}", li.content),
            ));
        }
        // comments check: any line starting with # at indent !=0? For now, we will handle in field parsing: if field name starts with #?
        if li.content.starts_with('#') {
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                "comments not allowed",
            ));
        }
        if li.content.starts_with("//") {
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                "comments not allowed",
            ));
        }
    }

    let mut body_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let body_fields = parse_fields(body_slice, 1, &mut body_seen, false)?;

    // After body, check for trailing blanks already handled; no more content

    // Build spans
    let total_start = level_line.line_no;
    let total_end = if body_fields.is_empty() && header_fields.is_empty() {
        body_line.line_no
    } else if !body_fields.is_empty() {
        body_fields.last().unwrap().span.end_line
    } else if !header_fields.is_empty() {
        // if body empty, span to body marker
        body_line.line_no
    } else {
        body_line.line_no
    };

    let parsed = ParsedPmap {
        level,
        level_marker,
        header: header_fields,
        body: body_fields,
        span: Span::range(total_start, 1, total_end, 1),
    };
    Ok(parsed)
}

fn parse_fields(
    slice: &[LineInfo],
    base_depth: usize,
    seen: &mut std::collections::HashSet<String>,
    _is_header: bool,
) -> Result<Vec<ParsedField>, ParseError> {
    let mut fields: Vec<ParsedField> = Vec::new();
    let mut i = 0usize;
    let field_indent = indent_for_depth(base_depth); // 2
    let value_indent = indent_for_depth(base_depth + 1); // 4
    let relation_indent = indent_for_depth(2); // 4 for @relation

    while i < slice.len() {
        let li = &slice[i];
        if li.is_blank {
            // Blank lines between fields: they should not appear as isolated between fields outside value blocks.
            // But blank lines inside value blocks are handled inside field value collection.
            // If we encounter a blank line here at top level between fields, it means either trailing blank after previous field's value block that wasn't consumed,
            // or an unexpected blank line. We already handle blank lines inside value block collection, so a blank here that is not inside any field's block
            // would be an extra blank line between fields. Spec says no extra blank line required between fields, but blank lines inside values are allowed.
            // An isolated blank line at field level (indent 0 or blank) that is not part of a value block should be ignored? But spec says blank lines inside value-block are preserved, not between fields.
            // For simplicity, treat isolated blank lines at this level as not allowed unless they are part of previous field's trailing?
            // However our field value collection already consumed blank lines inside blocks.
            // So if we reach here and li is blank, it means we have a blank line not inside any field's value block -> it's between fields.
            // Spec does not forbid blank lines between fields? It says blank lines inside value-block are preserved, but blank lines between sections are limited.
            // Between fields, a blank line would be ambiguous. We will treat a solitary blank line between fields as not allowed? But examples have no blank lines between fields.
            // For leniency, we'll just skip blank lines between fields, but if we see two consecutive blanks at this level, it's ConsecutiveBlankLines?
            // Let's just skip single blank lines between fields for now.
            // Check for consecutive blanks at field level
            if i + 1 < slice.len() && slice[i + 1].is_blank {
                return Err(ParseError::new(
                    ErrorKind::ConsecutiveBlankLines,
                    Span::single(li.line_no, 1),
                    "consecutive blank lines",
                ));
            }
            i += 1;
            continue;
        }

        // Must be field line at field_indent
        if li.indent != field_indent {
            // Check if it's a value line at wrong indent for this field level
            if li.indent == value_indent {
                // This would be a value line without a preceding field -> unexpected
                return Err(ParseError::new(
                    ErrorKind::UnexpectedNode,
                    Span::single(li.line_no, 1),
                    format!("unexpected value line without field: {}", li.content),
                ));
            }
            return Err(ParseError::new(
                ErrorKind::InvalidIndent,
                Span::single(li.line_no, 1),
                format!("field must be at indent {field_indent}"),
            ));
        }

        // Check for unexpected @relation at field level
        if li.content == RELATION_MARKER {
            return Err(ParseError::new(
                ErrorKind::InvalidRelationPlacement,
                Span::single(li.line_no, 1),
                "@relation outside relations",
            ));
        }
        if li.content.starts_with('@') {
            return Err(ParseError::new(
                ErrorKind::InvalidRelationPlacement,
                Span::single(li.line_no, 1),
                "unexpected @ marker",
            ));
        }

        // Field lines must not have trailing spaces after colon
        if li.has_trailing_space {
            return Err(ParseError::new(
                ErrorKind::TrailingContentAfterColon,
                Span::single(li.line_no, 1),
                "trailing content after colon",
            ));
        }

        // Check for comment-like: # at start
        if li.content.starts_with('#') {
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                "comments not allowed",
            ));
        }
        if li.content.starts_with("//") {
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                "comments not allowed",
            ));
        }

        // Field line must end with ':' and have no trailing content after colon
        if !li.content.ends_with(':') {
            // Check if it contains ':' but with inline value
            if li.content.contains(':') {
                // contains colon but not at end -> inline value
                if li.content.contains(": ") || li.content.contains(':') {
                    // Check if after colon there is content
                    let colon_pos = li.content.find(':').unwrap();
                    let after = &li.content[colon_pos + 1..];
                    if !after.trim().is_empty() {
                        return Err(ParseError::new(
                            ErrorKind::InlineValueNotAllowed,
                            Span::single(li.line_no, 1),
                            "inline value not allowed",
                        ));
                    }
                    // If after is empty but colon not at end due to space?
                    return Err(ParseError::new(
                        ErrorKind::TrailingContentAfterColon,
                        Span::single(li.line_no, 1),
                        "trailing content after colon",
                    ));
                }
                return Err(ParseError::new(
                    ErrorKind::MissingColon,
                    Span::single(li.line_no, 1),
                    "missing colon",
                ));
            } else {
                return Err(ParseError::new(
                    ErrorKind::MissingColon,
                    Span::single(li.line_no, 1),
                    "missing colon",
                ));
            }
        }
        // Now content ends with ':', extract name
        let name = &li.content[..li.content.len() - 1];
        if name.is_empty() {
            return Err(ParseError::new(
                ErrorKind::InvalidFieldName,
                Span::single(li.line_no, 1),
                "empty field name",
            ));
        }
        if !is_lowercase_name(name) {
            return Err(ParseError::new(
                ErrorKind::InvalidFieldName,
                Span::single(li.line_no, 1),
                format!("invalid field name {name}"),
            ));
        }

        // Check duplicate
        if seen.contains(name) {
            return Err(ParseError::new(
                ErrorKind::DuplicateField,
                Span::single(li.line_no, 1),
                format!("duplicate field {name}"),
            ));
        }
        seen.insert(name.to_string());

        // Collect value block: consecutive lines with indent > field_indent or blank lines
        let field_line_no = li.line_no;
        let mut block: Vec<LineInfo> = Vec::new();
        let mut j = i + 1;
        while j < slice.len() {
            let nxt = &slice[j];
            if nxt.is_blank {
                block.push(nxt.clone());
                j += 1;
                continue;
            }
            if nxt.indent == field_indent && nxt.content.ends_with(':') {
                // next sibling field
                break;
            }
            if nxt.indent > field_indent {
                // value line or relation marker
                // Validate indent is valid and not jump
                // For plain value, expected indent is value_indent (4) or deeper if extra spaces preserved?
                // Spec says after stripping required indent, remainder preserved. So indent could be >4 for extra spaces.
                // But we should ensure indent is at least value_indent and is multiple of 2, and not jumping from field 2 to 8 without 4?
                // Simple: allow any indent > field_indent that is multiple of 2, but check jump from previous value?
                // We'll allow any indent >= value_indent and multiple of 2.
                if !is_valid_indent(nxt.indent) {
                    return Err(ParseError::new(
                        ErrorKind::InvalidIndent,
                        Span::single(nxt.line_no, 1),
                        "invalid indent",
                    ));
                }
                // Check for unexpected @relation outside relations
                if nxt.content == RELATION_MARKER {
                    if nxt.has_trailing_space {
                        return Err(ParseError::new(
                            ErrorKind::InvalidRelationPlacement,
                            Span::single(nxt.line_no, 1),
                            "trailing content after @relation",
                        ));
                    }
                    if name != RELATIONS_FIELD {
                        return Err(ParseError::new(
                            ErrorKind::RelationOutsideRelations,
                            Span::single(nxt.line_no, 1),
                            "@relation outside relations field",
                        ));
                    }
                    if nxt.indent != relation_indent {
                        return Err(ParseError::new(
                            ErrorKind::InvalidRelationPlacement,
                            Span::single(nxt.line_no, 1),
                            format!("@relation must be at indent {relation_indent}"),
                        ));
                    }
                } else if nxt.content.starts_with('@') && nxt.indent == relation_indent {
                    return Err(ParseError::new(
                        ErrorKind::InvalidRelationPlacement,
                        Span::single(nxt.line_no, 1),
                        "invalid @ marker",
                    ));
                } else if nxt.content.starts_with('@') {
                    // @ in plain value field at value indent -> unexpected
                    if name != RELATIONS_FIELD {
                        return Err(ParseError::new(
                            ErrorKind::UnexpectedAtMarkerInPlainText,
                            Span::single(nxt.line_no, 1),
                            "unexpected @ in plain field",
                        ));
                    }
                }
                // Check indent jump: if previous value indent was value_indent and current is relation_value_indent (8) without intermediate, that's allowed only inside relation?
                // For plain field, value lines should be at value_indent (4) or deeper for extra spaces, but jump to 8 without being inside relation is invalid.
                // If name != relations and indent == relation_value_indent (8), that's jump from 2 to 8 -> invalid.
                if name != RELATIONS_FIELD && nxt.indent > value_indent {
                    // Allow extra spaces beyond 4? Spec says after stripping required indent, extra spaces preserved.
                    // So indent 5? But indent must be multiple of 2, so 6 would be extra 2 spaces beyond required.
                    // That could be intentional prose indent. Should we allow?
                    // For plain field, indent could be 4 plus extra. That would be 6, 8 etc. But spec says value lines at 4.
                    // To be lenient, allow indent >=4 for plain field, but ensure is_valid.
                    // However indent jump from 2 to 8 is larger than needed but still valid as extra spaces.
                    // We'll not error on jump for plain field for now.
                }
                // Also detect indent jump from field 2 directly to 8 as invalid? Let's enforce that for plain fields, indent must be exactly value_indent (4) unless intentional extra.
                // For now, allow any >= value_indent.
                block.push(nxt.clone());
                j += 1;
                continue;
            }
            break;
        }

        // Now process block to produce ParsedField
        let field_span_end_line = if block.is_empty() {
            field_line_no
        } else {
            // find last non-blank in block, or field line if all blank
            let mut end = field_line_no;
            for b in &block {
                if !b.is_blank {
                    end = b.line_no;
                }
            }
            // If block ends with blank lines, trim them for span? But we keep span to last value line
            // If all blank, end remains field_line_no
            // Check trailing blank not needed for span
            end
        };
        let span = Span::range(field_line_no, 1, field_span_end_line, 1);

        // Handle relations field specially
        if name == RELATIONS_FIELD {
            // Check for singular relation field? This name is already relations, singular would be "relation" which is different name; would be caught as field name "relation" with @relation inside -> RelationOutsideRelations?
            // But spec says singular "relation:" is invalid field name? Actually it's still lowercase shape valid, but wrapper must be "relations". So "relation:" field containing @relation should error RelationOutsideRelations.
            // Since we are inside relations field, parse relations
            let relations = parse_relations_block(&block, field_line_no)?;
            // For relations, raw_value is empty
            let pf = ParsedField {
                name: name.to_string(),
                raw_value: String::new(),
                relations,
                span,
            };
            fields.push(pf);
            i = j;
            continue;
        } else {
            // For other fields, block must not contain @relation
            for b in &block {
                if !b.is_blank && b.content == RELATION_MARKER {
                    return Err(ParseError::new(
                        ErrorKind::RelationOutsideRelations,
                        Span::single(b.line_no, 1),
                        "@relation outside relations",
                    ));
                }
            }
            // Check consecutive blank lines inside block
            let mut consecutive_blanks = 0usize;
            let mut has_value_line = false;
            for b in &block {
                if b.is_blank {
                    consecutive_blanks += 1;
                    if consecutive_blanks > 1 {
                        return Err(ParseError::new(
                            ErrorKind::ConsecutiveBlankLines,
                            Span::single(b.line_no, 1),
                            "consecutive blank lines",
                        ));
                    }
                } else {
                    consecutive_blanks = 0;
                    has_value_line = true;
                    // Check @ in plain text at value indent
                    let stripped = &b.content;
                    if stripped.starts_with('@') {
                        return Err(ParseError::new(
                            ErrorKind::UnexpectedAtMarkerInPlainText,
                            Span::single(b.line_no, 1),
                            "unexpected @ in plain text",
                        ));
                    }
                }
            }
            // Build raw_value: strip indent value_indent (4) from each value line, join
            let mut raw_parts: Vec<String> = Vec::new();
            let mut pending_blanks: usize = 0;
            for b in &block {
                if b.is_blank {
                    pending_blanks += 1;
                } else {
                    // handle blank(s) before this value line
                    if has_value_line && pending_blanks > 0 {
                        // one blank -> insert empty string to represent blank line
                        for _ in 0..pending_blanks {
                            raw_parts.push(String::new());
                        }
                    }
                    pending_blanks = 0;
                    // b.content is after stripping leading indent; extra beyond required 4 is preserved via indent diff
                    let extra = if b.indent > value_indent {
                        " ".repeat(b.indent - value_indent)
                    } else {
                        String::new()
                    };
                    let val = format!("{}{}", extra, b.content);
                    raw_parts.push(val);
                }
            }
            // Trim trailing blanks already not added because pending_blanks at end not flushed
            let raw_value = raw_parts.join("\n");
            let pf = ParsedField {
                name: name.to_string(),
                raw_value,
                relations: Vec::new(),
                span,
            };
            fields.push(pf);
            i = j;
            continue;
        }
    }

    Ok(fields)
}

fn parse_relations_block(
    block: &[LineInfo],
    _field_line_no: usize,
) -> Result<Vec<ParsedRelation>, ParseError> {
    let mut relations: Vec<ParsedRelation> = Vec::new();
    // Empty block is zero relations
    if block.iter().all(|b| b.is_blank) {
        return Ok(relations);
    }
    let relation_indent = indent_for_depth(2); // 4
    let relation_field_indent = indent_for_depth(3); // 6
    let relation_value_indent = indent_for_depth(4); // 8

    let mut i = 0usize;
    while i < block.len() {
        let li = &block[i];
        if li.is_blank {
            // blank lines between relations? Not allowed? But we can skip single blank?
            if i + 1 < block.len() && block[i + 1].is_blank {
                return Err(ParseError::new(
                    ErrorKind::ConsecutiveBlankLines,
                    Span::single(li.line_no, 1),
                    "consecutive blank lines in relations",
                ));
            }
            i += 1;
            continue;
        }
        if li.indent != relation_indent || li.content != RELATION_MARKER {
            // Could be value line at wrong indent for relations?
            if li.content == RELATION_MARKER && li.indent != relation_indent {
                return Err(ParseError::new(
                    ErrorKind::InvalidRelationPlacement,
                    Span::single(li.line_no, 1),
                    format!("@relation must be at indent {relation_indent}"),
                ));
            }
            return Err(ParseError::new(
                ErrorKind::UnexpectedNode,
                Span::single(li.line_no, 1),
                format!(
                    "expected @relation at indent {relation_indent}, got {}",
                    li.content
                ),
            ));
        }
        let rel_start = li.line_no;
        let mut rel_fields: Vec<ParsedField> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut j = i + 1;
        let mut rel_end = rel_start;
        while j < block.len() {
            let nxt = &block[j];
            if nxt.is_blank {
                j += 1;
                continue;
            }
            if nxt.indent == relation_indent && nxt.content == RELATION_MARKER {
                // next relation
                break;
            }
            if nxt.has_trailing_space {
                return Err(ParseError::new(
                    ErrorKind::TrailingContentAfterColon,
                    Span::single(nxt.line_no, 1),
                    "trailing content after colon",
                ));
            }
            if nxt.indent != relation_field_indent {
                return Err(ParseError::new(
                    ErrorKind::InvalidIndent,
                    Span::single(nxt.line_no, 1),
                    format!("relation field must be at indent {relation_field_indent}"),
                ));
            }
            if nxt.content == RELATION_MARKER {
                return Err(ParseError::new(
                    ErrorKind::InvalidRelationPlacement,
                    Span::single(nxt.line_no, 1),
                    "@relation inside relation",
                ));
            }
            if !nxt.content.ends_with(':') {
                if nxt.content.contains(':') {
                    return Err(ParseError::new(
                        ErrorKind::InlineValueNotAllowed,
                        Span::single(nxt.line_no, 1),
                        "inline value not allowed",
                    ));
                }
                return Err(ParseError::new(
                    ErrorKind::MissingColon,
                    Span::single(nxt.line_no, 1),
                    "missing colon in relation field",
                ));
            }
            let name = &nxt.content[..nxt.content.len() - 1];
            if !is_lowercase_name(name) {
                return Err(ParseError::new(
                    ErrorKind::InvalidFieldName,
                    Span::single(nxt.line_no, 1),
                    format!("invalid field name {name}"),
                ));
            }
            if seen.contains(name) {
                return Err(ParseError::new(
                    ErrorKind::DuplicateField,
                    Span::single(nxt.line_no, 1),
                    format!("duplicate field {name} in relation"),
                ));
            }
            seen.insert(name.to_string());
            let field_line_no = nxt.line_no;
            // collect value block for this relation field: lines with indent > relation_field_indent (i.e., 8)
            let mut block2: Vec<LineInfo> = Vec::new();
            let mut k = j + 1;
            while k < block.len() {
                let nb = &block[k];
                if nb.is_blank {
                    block2.push(nb.clone());
                    k += 1;
                    continue;
                }
                if nb.indent == relation_field_indent && nb.content.ends_with(':') {
                    break;
                }
                if nb.indent == relation_indent && nb.content == RELATION_MARKER {
                    break;
                }
                if nb.indent > relation_field_indent {
                    if !is_valid_indent(nb.indent) {
                        return Err(ParseError::new(
                            ErrorKind::InvalidIndent,
                            Span::single(nb.line_no, 1),
                            "invalid indent in relation value",
                        ));
                    }
                    if nb.indent != relation_value_indent && nb.indent > relation_value_indent {
                        // extra indent beyond 8 allowed as extra spaces?
                    }
                    if nb.content.starts_with('@') {
                        return Err(ParseError::new(
                            ErrorKind::UnexpectedAtMarkerInPlainText,
                            Span::single(nb.line_no, 1),
                            "unexpected @ in relation value",
                        ));
                    }
                    block2.push(nb.clone());
                    k += 1;
                    continue;
                }
                return Err(ParseError::new(
                    ErrorKind::InvalidIndent,
                    Span::single(nb.line_no, 1),
                    "invalid indent in relation",
                ));
            }
            // check consecutive blanks
            let mut consec = 0usize;
            for b in &block2 {
                if b.is_blank {
                    consec += 1;
                    if consec > 1 {
                        return Err(ParseError::new(
                            ErrorKind::ConsecutiveBlankLines,
                            Span::single(b.line_no, 1),
                            "consecutive blank lines in relation field",
                        ));
                    }
                } else {
                    consec = 0;
                }
            }
            // build raw_value for relation field
            let mut raw_parts: Vec<String> = Vec::new();
            let mut pending = 0usize;
            for b in &block2 {
                if b.is_blank {
                    pending += 1;
                } else {
                    if pending > 0 {
                        for _ in 0..pending {
                            raw_parts.push(String::new());
                        }
                        pending = 0;
                    }
                    let extra = if b.indent > relation_value_indent {
                        " ".repeat(b.indent - relation_value_indent)
                    } else {
                        String::new()
                    };
                    raw_parts.push(format!("{}{}", extra, b.content));
                }
            }
            let raw_value = raw_parts.join("\n");
            if raw_value.is_empty() && block2.iter().any(|b| !b.is_blank) {
                // value present but empty? Actually raw_value empty but had blanks? Not needed
            }
            // Check relations outside? Already handled
            let field_span_end = if block2.is_empty() {
                field_line_no
            } else {
                block2
                    .iter()
                    .rfind(|b| !b.is_blank)
                    .map(|b| b.line_no)
                    .unwrap_or(field_line_no)
            };
            let span = Span::range(field_line_no, 1, field_span_end, 1);
            rel_fields.push(ParsedField {
                name: name.to_string(),
                raw_value,
                relations: Vec::new(),
                span,
            });
            rel_end = field_span_end;
            j = k;
        }
        let span = Span::range(rel_start, 1, rel_end, 1);
        relations.push(ParsedRelation {
            fields: rel_fields,
            span,
        });
        i = j;
    }

    Ok(relations)
}
