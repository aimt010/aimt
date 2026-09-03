use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_kimi_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Kimi Code has no JSON config to merge for AIMT (skill is sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with other plugins and reserves a boundary for future Kimi-native
    // settings without duplicating shared logic.
    Ok(())
}
