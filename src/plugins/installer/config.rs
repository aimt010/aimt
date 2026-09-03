use std::path::Path;

use super::report::InstallError;

pub fn merge_plugin_entry(
    existing: &str,
    plugin_entry: &str,
    cfg_path: &Path,
) -> Result<String, InstallError> {
    let trimmed = existing.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return Err(InstallError::InvalidProject(format!(
            "invalid opencode.json at {}: not a JSON object",
            cfg_path.display()
        )));
    }
    if existing.contains("\"plugin\"") {
        let plugin_pos = existing.find("\"plugin\"").ok_or_else(|| {
            InstallError::InvalidProject(format!(
                "invalid opencode.json at {}: not a JSON object",
                cfg_path.display()
            ))
        })?;
        let bracket_open_rel = existing[plugin_pos..].find('[').ok_or_else(|| {
            InstallError::InvalidProject(format!(
                "invalid opencode.json at {}: missing '[' for plugin array",
                cfg_path.display()
            ))
        })?;
        let bracket_open = plugin_pos + bracket_open_rel;
        let bracket_close_rel = existing[bracket_open..].find(']').ok_or_else(|| {
            InstallError::InvalidProject(format!(
                "invalid opencode.json at {}: missing ']' for plugin array",
                cfg_path.display()
            ))
        })?;
        let bracket_close = bracket_open + bracket_close_rel;
        let inner = &existing[bracket_open + 1..bracket_close];
        let mut plugins: Vec<String> = Vec::new();
        if !inner.trim().is_empty() {
            for part in inner.split(',') {
                let p = part.trim().trim_matches('"').trim_matches('\'').trim();
                if !p.is_empty() {
                    plugins.push(p.to_string());
                }
            }
        }
        plugins.push(plugin_entry.to_string());
        let new_inner = plugins
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "{}{}{}",
            &existing[..bracket_open + 1],
            new_inner,
            &existing[bracket_close..]
        ))
    } else {
        let start = existing.find('{').unwrap();
        let end = existing.rfind('}').unwrap();
        let inner_raw = &existing[start + 1..end];
        if inner_raw.trim().is_empty() {
            Ok(format!("{{\n  \"plugin\": [\"{}\"]\n}}\n", plugin_entry))
        } else {
            let prefix = existing[..end].trim_end();
            let needs_comma = !prefix.ends_with('{') && !prefix.ends_with(',');
            let mut out = String::new();
            out.push_str(prefix);
            if needs_comma {
                out.push(',');
            }
            out.push_str(&format!("\n  \"plugin\": [\"{}\"]\n}}\n", plugin_entry));
            Ok(out)
        }
    }
}
