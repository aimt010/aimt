use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_codex_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Codex has no JSON config to merge for AIMT (AGENTS.md + skill are sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with opencode/claude and reserves a boundary for future Codex-native
    // settings (e.g., .codex/config.toml) without duplicating shared logic.
    Ok(())
}
