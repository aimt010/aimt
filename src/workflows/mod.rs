//! Workflow Engine — agentic AIMT behavior, host-agnostic.
//! Format decision: Rust structs (not Markdown per host) — see Task 1 doc.
//! Workflows will be Rust structs/enums (not Markdown/skills) because Core is Rust
//! and determinism/tests require `cargo test`; Markdown would be duplication per host.

pub mod capabilities;
pub mod context;
pub mod definition;
pub mod engine;
pub mod format;
pub mod lifecycle;
pub mod permissions;
pub mod read;
pub mod update;

pub use format::format_choice;
