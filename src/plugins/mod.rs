pub mod adapter;
pub mod aider;
pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod gemini;
pub mod installer;
pub mod kilo;
pub mod kimi;
pub mod opencode;
pub mod visualizer;

pub use installer::registry::{PluginInstaller, installer_for};
