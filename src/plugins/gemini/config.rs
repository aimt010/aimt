use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_gemini_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Gemini CLI has no JSON config to merge for AIMT (GEMINI.md is sufficient).
    // This placeholder preserves the 4-file structure (config/installer/mod/template)
    // consistent with other plugins and reserves a boundary for future Gemini-native
    // settings without duplicating shared logic.
    Ok(())
}
