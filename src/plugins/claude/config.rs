use std::path::Path;

use crate::plugins::installer::filesystem::io_error;
use crate::plugins::installer::report::{InstallError, InstallReport};

use super::template::CLAUDE_HOOK_ENTRY;

pub fn ensure_claude_config(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    let cfg_path = project_dir.join(".claude").join("settings.json");
    if !cfg_path.exists() {
        if let Some(parent) = cfg_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        }
        // Create minimal settings.json with AIMT hook
        let content = format!(
            "{{\n  \"hooks\": {{\n    \"PreToolUse\": [\n      {}\n    ]\n  }}\n}}\n",
            CLAUDE_HOOK_ENTRY
        );
        std::fs::write(&cfg_path, &content).map_err(|e| io_error(&cfg_path, e))?;
        report.created.push(cfg_path);
        return Ok(());
    }
    let existing = std::fs::read_to_string(&cfg_path).map_err(|e| io_error(&cfg_path, e))?;
    // Idempotency: if already contains AIMT hook marker, skip
    if existing.contains("aimt")
        && existing.contains("PreToolUse")
        && existing.contains("CLAUDE.md")
    {
        // More precise check: contains our hook entry substring
        if existing.contains("AIMT: .aimt knowledge") {
            report.skipped.push(cfg_path);
            return Ok(());
        }
    }
    // If existing already contains aimt hook entry, skip
    if existing.contains(CLAUDE_HOOK_ENTRY) || existing.contains("AIMT: .aimt knowledge") {
        report.skipped.push(cfg_path);
        return Ok(());
    }
    let trimmed = existing.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        let content = format!(
            "{{\n  \"hooks\": {{\n    \"PreToolUse\": [\n      {}\n    ]\n  }}\n}}\n",
            CLAUDE_HOOK_ENTRY
        );
        std::fs::write(&cfg_path, &content).map_err(|e| io_error(&cfg_path, e))?;
        report.updated.push(cfg_path);
        return Ok(());
    }
    // Try to preserve existing config: if it contains "PreToolUse", insert hook into array
    if existing.contains("\"PreToolUse\"") {
        // Find PreToolUse array and insert hook before closing ]
        if let Some(pos) = existing.find("\"PreToolUse\"")
            && let Some(bracket_open_rel) = existing[pos..].find('[')
        {
            let bracket_open = pos + bracket_open_rel;
            // Find matching closing bracket for PreToolUse array (simplified: find next "]" after open)
            if let Some(bracket_close_rel) = existing[bracket_open..].find(']') {
                let bracket_close = bracket_open + bracket_close_rel;
                let inner = &existing[bracket_open + 1..bracket_close];
                let needs_comma = !inner.trim().is_empty();
                let new_inner = if needs_comma {
                    format!("{}, {}", inner.trim_end(), CLAUDE_HOOK_ENTRY)
                } else {
                    CLAUDE_HOOK_ENTRY.to_string()
                };
                let new_content = format!(
                    "{}{}{}",
                    &existing[..bracket_open + 1],
                    new_inner,
                    &existing[bracket_close..]
                );
                std::fs::write(&cfg_path, &new_content).map_err(|e| io_error(&cfg_path, e))?;
                report.updated.push(cfg_path);
                return Ok(());
            }
        }
    }
    // If contains "hooks" but not PreToolUse, add PreToolUse
    if existing.contains("\"hooks\"") {
        // Insert PreToolUse hook inside hooks object
        if let Some(pos) = existing.find("\"hooks\"")
            && let Some(brace_open_rel) = existing[pos..].find('{')
        {
            let brace_open = pos + brace_open_rel;
            // Find closing } for hooks (simplified: find next matching } - use rfind for outer)
            // Instead, insert before the closing } of hooks object
            // Find the first "{" after hooks and then insert after it
            let new_content = format!(
                "{}\"PreToolUse\": [{}], {}",
                &existing[..brace_open + 1],
                CLAUDE_HOOK_ENTRY,
                existing[brace_open + 1..].trim_start()
            );
            // Ensure we don't duplicate comma handling
            let cleaned = new_content.replace("\"PreToolUse\": [{", "\"PreToolUse\": [\n      {");
            std::fs::write(&cfg_path, &cleaned).map_err(|e| io_error(&cfg_path, e))?;
            report.updated.push(cfg_path);
            return Ok(());
        }
    }
    // Fallback: wrap existing content and add hooks
    // Preserve existing by inserting hooks before final }
    let trimmed_end = existing.trim_end();
    if trimmed_end.ends_with('}') {
        let without_closing = trimmed_end.strip_suffix('}').unwrap_or(trimmed_end);
        let needs_comma =
            !without_closing.trim().ends_with('{') && !without_closing.trim().is_empty();
        let new_content = if needs_comma {
            format!(
                "{},\n  \"hooks\": {{\n    \"PreToolUse\": [\n      {}\n    ]\n  }}\n}}\n",
                without_closing.trim_end(),
                CLAUDE_HOOK_ENTRY
            )
        } else {
            format!(
                "{}\n  \"hooks\": {{\n    \"PreToolUse\": [\n      {}\n    ]\n  }}\n}}\n",
                without_closing.trim_end(),
                CLAUDE_HOOK_ENTRY
            )
        };
        std::fs::write(&cfg_path, &new_content).map_err(|e| io_error(&cfg_path, e))?;
        report.updated.push(cfg_path);
        return Ok(());
    }
    // Last resort: overwrite with minimal
    let content = format!(
        "{{\n  \"hooks\": {{\n    \"PreToolUse\": [\n      {}\n    ]\n  }}\n}}\n",
        CLAUDE_HOOK_ENTRY
    );
    std::fs::write(&cfg_path, &content).map_err(|e| io_error(&cfg_path, e))?;
    report.updated.push(cfg_path);
    Ok(())
}
