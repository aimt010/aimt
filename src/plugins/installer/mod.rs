pub mod aimt;
pub mod config;
pub mod filesystem;
pub mod guide;
pub mod registry;
pub mod report;
pub mod template;

pub use filesystem::write_if_changed;
pub use guide::{ensure_claude_guide, ensure_copilot_guide, ensure_gemini_guide, ensure_guide};
pub use registry::{PluginInstaller, installer_for};
pub use report::{InstallError, InstallReport, IoErrorDetails};
