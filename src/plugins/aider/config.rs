use std::path::Path;

use crate::plugins::installer::filesystem::io_error;
use crate::plugins::installer::report::{InstallError, InstallReport};

const AIMT_READ_ENTRY: &str = ".aider/aimt.md";

pub fn ensure_aider_config(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    let cfg_path = project_dir.join(".aider.conf.yml");
    if !cfg_path.exists() {
        if let Some(parent) = cfg_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        }
        let content = format!("read: [{}]\n", AIMT_READ_ENTRY);
        std::fs::write(&cfg_path, &content).map_err(|e| io_error(&cfg_path, e))?;
        report.created.push(cfg_path);
        return Ok(());
    }
    let existing = std::fs::read_to_string(&cfg_path).map_err(|e| io_error(&cfg_path, e))?;
    if existing.contains(AIMT_READ_ENTRY) {
        report.skipped.push(cfg_path);
        return Ok(());
    }
    // Parse existing read entries and merge
    let new_content = merge_read_entry(&existing, AIMT_READ_ENTRY, &cfg_path)?;
    if new_content == existing {
        report.skipped.push(cfg_path);
        return Ok(());
    }
    std::fs::write(&cfg_path, &new_content).map_err(|e| io_error(&cfg_path, e))?;
    report.updated.push(cfg_path);
    Ok(())
}

fn merge_read_entry(existing: &str, entry: &str, _cfg_path: &Path) -> Result<String, InstallError> {
    // Handle different YAML list formats for `read:`
    // Check if file contains `read:` key
    if let Some(read_pos) = existing.find("read:") {
        let after_read = &existing[read_pos + 5..];
        // Find end of read value (next non-indented key or EOF)
        // For simplicity, handle three cases:
        // 1. `read: [a, b]` - bracket list
        // 2. `read:\n  - a\n  - b` - bulleted list
        // 3. `read: single_file` - single value
        let read_line_end = after_read.find('\n').unwrap_or(after_read.len());
        let read_value = after_read[..read_line_end].trim();
        if read_value.starts_with('[') {
            // Bracket list: read: [a, b]
            if let Some(bracket_close) = after_read.find(']') {
                let inner_start = read_pos + 5 + after_read.find('[').unwrap() + 1;
                let inner_end = read_pos + 5 + bracket_close;
                let inner = &existing[inner_start..inner_end];
                let mut entries: Vec<String> = Vec::new();
                if !inner.trim().is_empty() {
                    for part in inner.split(',') {
                        let p = part.trim().trim_matches('"').trim_matches('\'').trim();
                        if !p.is_empty() {
                            entries.push(p.to_string());
                        }
                    }
                }
                if entries.contains(&entry.to_string()) {
                    return Ok(existing.to_string());
                }
                entries.push(entry.to_string());
                let new_inner = entries
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let new_content = format!(
                    "{}{}[{}]{}",
                    &existing[..inner_start],
                    "",
                    new_inner,
                    &existing[inner_end..]
                );
                // Fix up: ensure we have read: [new_inner]
                let fixed =
                    new_content.replace(&format!("[{}]", inner), &format!("[{}]", new_inner));
                // If inner was empty, the above will handle
                if fixed.contains(entry) {
                    return Ok(fixed);
                }
                // Fallback: reconstruct bracket list
                let before = &existing[..read_pos + 5];
                let after_bracket = &existing[read_pos + 5 + bracket_close + 1..];
                let new_bracket_content = entries.join(", ");
                return Ok(format!(
                    "{} [{}]{}",
                    before.trim_end(),
                    new_bracket_content,
                    after_bracket
                ));
            }
        } else if read_value.is_empty() {
            // Bulleted list: read:\n  - a\n  - b
            // Find all following indented list items
            let after_read_rest = &existing[read_pos + 5 + read_line_end..];
            let mut entries: Vec<String> = Vec::new();
            let end_pos = read_pos + 5 + read_line_end;
            let lines = after_read_rest.lines().peekable();
            let mut found_bullets = false;
            let mut bullet_end = end_pos;
            for line in lines {
                let trimmed = line.trim();
                if let Some(stripped) = trimmed.strip_prefix("- ") {
                    found_bullets = true;
                    let val = stripped.trim().trim_matches('"').trim_matches('\'');
                    if !val.is_empty() {
                        entries.push(val.to_string());
                    }
                    bullet_end += line.len() + 1; // +1 for \n
                } else if trimmed.is_empty() {
                    bullet_end += line.len() + 1;
                    continue;
                } else {
                    break;
                }
            }
            if found_bullets {
                if entries.contains(&entry.to_string()) {
                    return Ok(existing.to_string());
                }
                entries.push(entry.to_string());
                let bullet_list = entries
                    .iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                let before = &existing[..read_pos + 5];
                let after = &existing[bullet_end..];
                return Ok(format!(
                    "{}\n{}\n{}",
                    before.trim_end(),
                    bullet_list,
                    after.trim_start()
                ));
            } else {
                // Empty read: with no value, add bracket list
                let before = &existing[..read_pos + 5];
                let after = &existing[read_pos + 5 + read_line_end..];
                return Ok(format!("{} [{}]{}", before.trim_end(), entry, after));
            }
        } else {
            // Single value: read: somefile
            let single = read_value.trim().trim_matches('"').trim_matches('\'');
            if single == entry {
                return Ok(existing.to_string());
            }
            // Convert to bracket list with both entries
            let before = &existing[..read_pos];
            let after = &existing[read_pos + 5 + read_line_end..];
            return Ok(format!("{}read: [{}, {}]{}", before, single, entry, after));
        }
    }
    // No read key found, add it preserving existing content
    let trimmed = existing.trim_end();
    if trimmed.is_empty() {
        return Ok(format!("read: [{}]\n", entry));
    }
    // Append read entry at end, preserving existing content
    Ok(format!("{}\nread: [{}]\n", trimmed, entry))
}
