/// Returns the locked-in workflow representation choice.
///
/// Decision record: "Workflows will be Rust structs/enums (not Markdown/skills)
/// because Core is Rust and determinism/tests require `cargo test`;
/// Markdown would be duplication per host." If evaluation had chosen Markdown,
/// this task would instead create `workflows/*.md` and justify.
///
/// Single source `src/workflows/` — plugins are thin translators.
pub fn format_choice() -> &'static str {
    "rust-engine-not-markdown-per-host"
}
