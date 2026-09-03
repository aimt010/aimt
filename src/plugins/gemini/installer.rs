use std::path::Path;

use super::config::ensure_gemini_config;
use crate::plugins::installer::filesystem::validate_project_dir;
use crate::plugins::installer::guide::ensure_gemini_guide;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct GeminiInstaller;

impl PluginInstaller for GeminiInstaller {
    fn tool(&self) -> &'static str {
        "gemini"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "gemini".into(),
            ..Default::default()
        };
        ensure_gemini_guide(project_dir, &mut report)?;
        ensure_gemini_config(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: GeminiInstaller = GeminiInstaller;

pub fn gemini_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
