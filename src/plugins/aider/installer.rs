use std::path::Path;

use super::config::ensure_aider_config;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_aimt_prompts;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};
use crate::plugins::installer::template::AIMT_HOST_GUIDE_MD;

struct AiderInstaller;

impl PluginInstaller for AiderInstaller {
    fn tool(&self) -> &'static str {
        "aider"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "aider".into(),
            ..Default::default()
        };
        ensure_aimt_prompts(project_dir, &mut report)?;
        let guide_path = project_dir.join(".aider").join("aimt.md");
        write_if_changed(&guide_path, AIMT_HOST_GUIDE_MD, &mut report)?;
        ensure_aider_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: AiderInstaller = AiderInstaller;

pub fn aider_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
