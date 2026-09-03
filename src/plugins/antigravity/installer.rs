use std::path::Path;

use super::config::ensure_antigravity_config;
use super::template::ANTIGRAVITY_SKILL_MD;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_aimt_prompts;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct AntigravityInstaller;

impl PluginInstaller for AntigravityInstaller {
    fn tool(&self) -> &'static str {
        "antigravity"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "antigravity".into(),
            ..Default::default()
        };
        ensure_aimt_prompts(project_dir, &mut report)?;
        let skill_path = project_dir
            .join(".agents")
            .join("skills")
            .join("aimt")
            .join("SKILL.md");
        write_if_changed(&skill_path, ANTIGRAVITY_SKILL_MD, &mut report)?;
        ensure_antigravity_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: AntigravityInstaller = AntigravityInstaller;

pub fn antigravity_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
