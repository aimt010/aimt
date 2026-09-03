use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_antigravity_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Antigravity has no JSON config to merge for AIMT (skill is sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with opencode/claude/codex and reserves a boundary for future
    // Antigravity-native settings without duplicating shared logic.
    Ok(())
}
