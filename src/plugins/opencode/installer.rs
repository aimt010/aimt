use std::path::Path;

use super::config::ensure_opencode_config;
use super::template::{OPENCODE_AIMT_COMMAND_MD, OPENCODE_PLUGIN_JS};
use crate::plugins::installer::filesystem::{validate_project_dir, write_if_changed};
use crate::plugins::installer::guide::ensure_guide;
use crate::plugins::installer::registry::PluginInstaller;
use crate::plugins::installer::report::{InstallError, InstallReport};

struct OpenCodeInstaller;

impl PluginInstaller for OpenCodeInstaller {
    fn tool(&self) -> &'static str {
        "opencode"
    }

    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError> {
        validate_project_dir(project_dir)?;
        let mut report = InstallReport {
            tool: "opencode".into(),
            ..Default::default()
        };
        let plugin_path = project_dir
            .join(".opencode")
            .join("plugins")
            .join("aimt.js");
        write_if_changed(&plugin_path, OPENCODE_PLUGIN_JS, &mut report)?;
        let cmd_path = project_dir
            .join(".opencode")
            .join("commands")
            .join("aimt.md");
        write_if_changed(&cmd_path, OPENCODE_AIMT_COMMAND_MD, &mut report)?;
        ensure_opencode_config(project_dir, &mut report)?;
        ensure_guide(project_dir, &mut report)?;
        report.created.sort();
        report.updated.sort();
        report.skipped.sort();
        Ok(report)
    }
}

static INSTALLER: OpenCodeInstaller = OpenCodeInstaller;

pub fn opencode_installer() -> &'static dyn PluginInstaller {
    &INSTALLER
}
