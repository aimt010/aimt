use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_copilot_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Copilot CLI has no JSON config to merge for AIMT (instructions file is sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with other plugins and reserves a boundary for future Copilot-native
    // settings without duplicating shared logic.
    Ok(())
}
