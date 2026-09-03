use std::path::Path;

use super::config::ensure_cursor_config;
use super::template::CURSOR_RULE_MDC;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_aimt_prompts;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct CursorInstaller;

impl PluginInstaller for CursorInstaller {
    fn tool(&self) -> &'static str {
        "cursor"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "cursor".into(),
            ..Default::default()
        };
        ensure_aimt_prompts(project_dir, &mut report)?;
        let rule_path = project_dir.join(".cursor").join("rules").join("aimt.mdc");
        write_if_changed(&rule_path, CURSOR_RULE_MDC, &mut report)?;
        ensure_cursor_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: CursorInstaller = CursorInstaller;

pub fn cursor_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
