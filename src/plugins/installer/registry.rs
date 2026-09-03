use std::path::Path;

use super::report::{InstallError, InstallReport};

pub trait PluginInstaller {
    fn tool(&self) -> &'static str;
    fn install(&self, project_dir: &Path) -> Result<InstallReport, InstallError>;
}

pub fn installer_for(tool: &str) -> Option<&'static dyn PluginInstaller> {
    match tool {
        "opencode" => Some(crate::plugins::opencode::opencode_installer()),
        "claude" => Some(crate::plugins::claude::claude_installer()),
        "codex" => Some(crate::plugins::codex::codex_installer()),
        "antigravity" => Some(crate::plugins::antigravity::antigravity_installer()),
        "kilo" => Some(crate::plugins::kilo::kilo_installer()),
        "copilot" => Some(crate::plugins::copilot::copilot_installer()),
        "aider" => Some(crate::plugins::aider::aider_installer()),
        "cursor" => Some(crate::plugins::cursor::cursor_installer()),
        "gemini" => Some(crate::plugins::gemini::gemini_installer()),
        "kimi" => Some(crate::plugins::kimi::kimi_installer()),
        _ => None,
    }
}
