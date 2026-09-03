#[test]
fn workflow_format_is_decided_and_not_markdown_duplication() {
    // This test documents the architectural decision, not code:
    // - If workflows were Markdown, every plugin would copy `skills/aimt-*.md` per host (forbidden).
    // - Choosing Rust means `src/workflows/` is single source, plugins are thin translators.
    let choice = aimt::workflows::format_choice(); // not yet existent
    assert_eq!(choice, "rust-engine-not-markdown-per-host");
}
