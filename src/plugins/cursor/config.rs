use std::path::Path;

use crate::plugins::installer::report::{InstallError, InstallReport};

pub fn ensure_cursor_config(
    _project_dir: &Path,
    _report: &mut InstallReport,
) -> Result<(), InstallError> {
    // Cursor has no JSON config to merge for AIMT (rule file is sufficient).
    // This placeholder preserves the 5-file structure (adapter/config/installer/mod/template)
    // consistent with other plugins and reserves a boundary for future Cursor-native
    // settings without duplicating shared logic.
    Ok(())
}
