use std::path::Path;

use super::config::ensure_kilo_config;
use super::template::KILO_SKILL_MD;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_aimt_prompts;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct KiloInstaller;

impl PluginInstaller for KiloInstaller {
    fn tool(&self) -> &'static str {
        "kilo"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "kilo".into(),
            ..Default::default()
        };
        ensure_aimt_prompts(project_dir, &mut report)?;
        let skill_path = project_dir
            .join(".kilo")
            .join("skills")
            .join("aimt")
            .join("SKILL.md");
        write_if_changed(&skill_path, KILO_SKILL_MD, &mut report)?;
        ensure_kilo_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: KiloInstaller = KiloInstaller;

pub fn kilo_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
