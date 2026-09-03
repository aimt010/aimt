use std::path::Path;

use super::config::ensure_copilot_config;
use crate::plugins::installer::filesystem::validate_project_dir;
use crate::plugins::installer::guide::ensure_copilot_guide;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct CopilotInstaller;

impl PluginInstaller for CopilotInstaller {
    fn tool(&self) -> &'static str {
        "copilot"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "copilot".into(),
            ..Default::default()
        };
        ensure_copilot_guide(project_dir, &mut report)?;
        ensure_copilot_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: CopilotInstaller = CopilotInstaller;

pub fn copilot_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
