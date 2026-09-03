use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_kilo_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Kilo Code has no JSON config to merge for AIMT (skill is sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with opencode/claude/codex/antigravity and reserves a boundary for future
    // Kilo-native settings without duplicating shared logic.
    Ok(())
}
