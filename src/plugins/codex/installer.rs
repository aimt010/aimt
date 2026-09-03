use std::path::Path;

use super::config::ensure_codex_config;
use super::template::CODEX_SKILL_MD;
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_guide;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct CodexInstaller;

impl PluginInstaller for CodexInstaller {
    fn tool(&self) -> &'static str {
        "codex"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "codex".into(),
            ..Default::default()
        };
        ensure_guide(project_dir, &mut report)?;
        let skill_path = project_dir
            .join(".agents")
            .join("skills")
            .join("aimt")
            .join("SKILL.md");
        write_if_changed(&skill_path, CODEX_SKILL_MD, &mut report)?;
        ensure_codex_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: CodexInstaller = CodexInstaller;

pub fn codex_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
