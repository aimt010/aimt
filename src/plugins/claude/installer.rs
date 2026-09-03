use std::path::Path;

use super::config::ensure_claude_config;
use crate::plugins::installer::filesystem::validate_project_dir;
use crate::plugins::installer::guide::ensure_claude_guide;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct ClaudeInstaller;

impl PluginInstaller for ClaudeInstaller {
    fn tool(&self) -> &'static str {
        "claude"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "claude".into(),
            ..Default::default()
        };
        ensure_claude_guide(project_dir, &mut report)?;
        ensure_claude_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: ClaudeInstaller = ClaudeInstaller;

pub fn claude_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
