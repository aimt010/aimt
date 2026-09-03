use std::path::Path;

use super::config::ensure_kimi_config;
use super::template::KIMI_SKILL_MD;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_aimt_prompts;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct KimiInstaller;

impl PluginInstaller for KimiInstaller {
    fn tool(&self) -> &'static str {
        "kimi"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "kimi".into(),
            ..Default::default()
        };
        ensure_aimt_prompts(project_dir, &mut report)?;
        let skill_path = project_dir
            .join(".kimi")
            .join("skills")
            .join("aimt")
            .join("SKILL.md");
        write_if_changed(&skill_path, KIMI_SKILL_MD, &mut report)?;
        ensure_kimi_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: KimiInstaller = KimiInstaller;

pub fn kimi_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
