use std::path::Path;

use crate::plugins::installer::config::merge_plugin_entry;
use crate::plugins::installer::filesystem::io_error;
use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_opencode_config(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    let cfg_path = project_dir.join(".opencode").join("opencode.json");
    let plugin_entry = ".opencode/plugins/aimt.js";
    if !cfg_path.exists() {
        if let Some(parent) = cfg_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        }
        let content = format!("{{\n  \"plugin\": [\"{}\"]\n}}\n", plugin_entry);
        std::fs::write(&cfg_path, &content).map_err(|e| io_error(&cfg_path, e))?;
        report.created.push(cfg_path);
        return Ok(());
    }
    let existing = std::fs::read_to_string(&cfg_path).map_err(|e| io_error(&cfg_path, e))?;
    if existing.contains(plugin_entry) {
        report.skipped.push(cfg_path);
        return Ok(());
    }
    let new_content = merge_plugin_entry(&existing, plugin_entry, &cfg_path)?;
    std::fs::write(&cfg_path, &new_content).map_err(|e| io_error(&cfg_path, e))?;
    report.updated.push(cfg_path);
    Ok(())
}
