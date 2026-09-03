use std::path::{Path, PathBuf};

#[test]
fn help_lists_install_command() {
    let out = std::process::Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("install"),
        "help should list install: got {}",
        stdout
    );
    assert!(stdout.contains("opencode"), "help should mention opencode");
    assert!(stdout.contains("claude"), "help should mention claude");
    assert!(stdout.contains("codex"), "help should mention codex");
    assert!(
        stdout.contains("antigravity"),
        "help should mention antigravity"
    );
    assert!(stdout.contains("kilo"), "help should mention kilo");
    assert!(stdout.contains("copilot"), "help should mention copilot");
    assert!(stdout.contains("aider"), "help should mention aider");
    assert!(stdout.contains("cursor"), "help should mention cursor");
    assert!(stdout.contains("gemini"), "help should mention gemini");
    assert!(stdout.contains("kimi"), "help should mention kimi");
    assert!(stdout.contains("kilo"), "help should mention kilo");
}

#[test]
fn install_unknown_tool_errors() {
    let dir = tempfile_dir();
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "unknown_tool_xyz",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "unknown tool should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown tool") || stderr.contains("unknown"),
        "stderr should mention unknown tool: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn tempfile_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let p = base.join(format!(
        "aimt_install_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn install_creates_opencode_plugin_file() {
    let dir = tempfile_dir();
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let plugin = dir.join(".opencode").join("plugins").join("aimt.js");
    assert!(
        plugin.is_file(),
        "plugin file should exist at {}",
        plugin.display()
    );
    let content = std::fs::read_to_string(&plugin).unwrap();
    assert!(
        content.contains("AimPlugin") || content.contains("AIMT"),
        "plugin should contain AIMT integration, got: {}",
        &content[..200.min(content.len())]
    );
    assert!(
        content.contains("aimt") && content.contains("tool"),
        "plugin should expose aimt tool"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_creates_directories_automatically() {
    let base = std::env::temp_dir().join(format!(
        "aimt_install_dir_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let proj = base.join("nested").join("project");
    // Do not create proj — installer must handle missing parents via create_dir_all on .opencode/plugins
    // But spec says parent does not exist case should error for init; for install we create .opencode inside project, so project itself must exist?
    // Instead test: create project dir but not .opencode
    std::fs::create_dir_all(&proj).unwrap();
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            proj.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "install should create .opencode dirs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(proj.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn install_registers_plugin_in_opencode_json() {
    let dir = tempfile_dir();
    let out = run_install(&dir);
    assert!(out.status.success());
    let cfg = dir.join(".opencode").join("opencode.json");
    assert!(cfg.is_file(), "opencode.json should exist");
    let content = std::fs::read_to_string(&cfg).unwrap();
    assert!(
        content.contains("aimt.js"),
        "config should register aimt.js: {}",
        content
    );
    // Should have valid JSON structure
    assert!(content.contains("plugin"), "config should have plugin key");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_preserves_existing_graphify_plugin() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".opencode").join("opencode.json");
    std::fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg_path,
        r#"{"plugin": [".opencode/plugins/graphify.js"]}"#,
    )
    .unwrap();
    let out = run_install(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(
        content.contains("graphify.js"),
        "should preserve graphify: {}",
        content
    );
    assert!(content.contains("aimt.js"), "should add aimt: {}", content);
    // Count occurrences — aimt.js should appear exactly once
    assert_eq!(content.matches("aimt.js").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_does_not_overwrite_unrelated_config_keys() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".opencode").join("opencode.json");
    std::fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg_path,
        r#"{"$schema": "https://opencode.ai/config.json", "plugin": [], "custom": {"keep": true}}"#,
    )
    .unwrap();
    let out = run_install(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(
        content.contains("keep") || content.contains("custom"),
        "should preserve custom key: {}",
        content
    );
    assert!(
        content.contains("\"$schema\"") || content.contains("$schema"),
        "should preserve $schema: {}",
        content
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_creates_aimt_guide() {
    let dir = tempfile_dir();
    let out = run_install(&dir);
    assert!(
        out.status.success(),
        "install should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let guide = dir.join("AGENTS.md");
    assert!(
        guide.is_file(),
        "AGENTS.md should exist at {}",
        guide.display()
    );
    let content = std::fs::read_to_string(&guide).unwrap();
    // Minimal host guide — conceptual relationship only, not verbose gate (Tasks 6/7)
    assert!(
        content.contains("## aimt"),
        "guide should contain marker ## aimt"
    );
    assert!(
        content.contains(".agents/aimt/index.md"),
        "guide should point to .agents/aimt/index.md"
    );
    assert!(content.contains(".aimt"), "guide should mention .aimt file");
    assert!(
        content.contains("primary knowledge map")
            || content.contains("knowledge map when available"),
        "guide should state AIMT is primary knowledge map when available"
    );
    assert!(
        content.contains("evidence"),
        "guide should state source is evidence"
    );
    assert!(
        content.contains("missing")
            || content.contains("stale")
            || content.contains("insufficient"),
        "guide should mention missing/stale handling"
    );
    assert!(
        content.contains("Do not create")
            || content.contains("Do NOT")
            || content.contains("no separate"),
        "guide should forbid separate project dir"
    );
    assert!(
        !content.contains("before answering") && !content.contains("before reading source"),
        "guide should not be mandatory gate"
    );
    assert!(
        content.lines().count() < 20,
        "guide should be minimal, got {} lines",
        content.lines().count()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_preserves_existing_agents_content() {
    let dir = tempfile_dir();
    let agents = dir.join("AGENTS.md");
    std::fs::write(&agents, "# My Project\n\nExisting content here.\n").unwrap();
    let out = run_install(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&agents).unwrap();
    assert!(
        content.contains("My Project"),
        "should preserve existing AGENTS.md content"
    );
    assert!(
        content.contains("Existing content"),
        "should preserve existing body"
    );
    assert!(content.contains("## aimt"), "should add aimt section");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_guide_contains_workflow_section() {
    let dir = tempfile_dir();
    let out = run_install(&dir);
    assert!(out.status.success());
    let host_content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        host_content.contains(".agents/aimt/index.md"),
        "host guide should reference .agents/aimt/index.md"
    );
    // Minimal 5-file architecture: index table + operations covers lifecycle (Task 6)
    let aimt_dir = dir.join(".agents/aimt");
    let mut names: Vec<String> = std::fs::read_dir(&aimt_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "core.md",
            "index.md",
            "mapping.md",
            "operations.md",
            "update.md"
        ],
        "should be exactly 5 minimal files, got {:?}",
        names
    );
    // operations.md covers full lifecycle OPEN/READ/follow/WRITE/VALIDATE/CLOSE
    let ops = std::fs::read_to_string(aimt_dir.join("operations.md")).unwrap();
    assert!(ops.contains("OPEN"), "operations should mention OPEN");
    assert!(ops.contains("READ"), "operations should mention READ");
    assert!(
        ops.contains("VALIDATE"),
        "operations should mention VALIDATE"
    );
    assert!(ops.contains("CLOSE"), "operations should mention CLOSE");
    assert!(
        ops.contains("follow") || ops.contains("Follow"),
        "operations should mention follow"
    );
    let index = std::fs::read_to_string(aimt_dir.join("index.md")).unwrap();
    assert!(
        index.contains("| File | Responsibility |"),
        "index should be protocol table"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn run_install(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn repeated_installation_is_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install(&dir);
    assert!(out1.status.success());
    let content1_plugin = std::fs::read_to_string(dir.join(".opencode/plugins/aimt.js")).unwrap();
    let content1_agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let content1_config = std::fs::read_to_string(dir.join(".opencode/opencode.json")).unwrap();
    let out2 = run_install(&dir);
    assert!(
        out2.status.success(),
        "second install should succeed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    // Installer output is concise "AIMT installed for OpenCode."; idempotency is proven via file equality
    assert!(
        stdout2.to_lowercase().contains("aimt installed for")
            || stdout2.contains("skipped")
            || stdout2.contains("already"),
        "second run should report installed or skipped: {}",
        stdout2
    );
    assert_eq!(
        content1_plugin,
        std::fs::read_to_string(dir.join(".opencode/plugins/aimt.js")).unwrap()
    );
    assert_eq!(
        content1_agents,
        std::fs::read_to_string(dir.join("AGENTS.md")).unwrap()
    );
    assert_eq!(
        content1_config,
        std::fs::read_to_string(dir.join(".opencode/opencode.json")).unwrap()
    );
    // Ensure plugin entry appears exactly once
    assert_eq!(content1_config.matches("aimt.js").count(), 1);
    // Ensure AGENTS.md marker appears exactly once
    assert_eq!(content1_agents.matches("## aimt").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let keep = dir.join("src").join("keep.txt");
    std::fs::create_dir_all(keep.parent().unwrap()).unwrap();
    std::fs::write(&keep, "do not touch").unwrap();
    // Also create a pre-existing graphify plugin that must be preserved
    let graphify = dir.join(".opencode").join("plugins").join("graphify.js");
    std::fs::create_dir_all(graphify.parent().unwrap()).unwrap();
    std::fs::write(&graphify, "// graphify original").unwrap();
    let out = run_install(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    assert_eq!(std::fs::read_to_string(&keep).unwrap(), "do not touch");
    assert_eq!(
        std::fs::read_to_string(&graphify).unwrap(),
        "// graphify original"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_reports_what_was_installed() {
    let dir = tempfile_dir();
    let out = run_install(&dir);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.to_lowercase().contains("aimt installed for"),
        "should report installation: {}",
        stdout
    );
    // Verify actual artifacts were created regardless of stdout verbosity (installer is concise)
    assert!(
        dir.join(".opencode/plugins/aimt.js").is_file(),
        "plugin file should exist"
    );
    assert!(
        dir.join(".agents/aimt/index.md").is_file(),
        "index.md should exist"
    );
    assert!(dir.join("AGENTS.md").is_file(), "AGENTS.md should exist");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_errors_on_nonexistent_project_dir() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_nonexistent_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!dir.exists());
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should fail for missing project dir");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("invalid project"),
        "should explain: {}",
        stderr
    );
}

#[test]
fn install_errors_when_path_is_file() {
    let dir = tempfile_dir();
    let file = dir.join("not_a_dir");
    std::fs::write(&file, "hello").unwrap();
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not a directory") || stderr.contains("invalid"),
        "stderr: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_errors_on_invalid_opencode_json_and_preserves() {
    let dir = tempfile_dir();
    let cfg = dir.join(".opencode").join("opencode.json");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(&cfg, "not json {").unwrap();
    let out = run_install(&dir);
    assert!(!out.status.success(), "should fail on invalid json");
    // File should not be corrupted to empty; should still be the original invalid content or unchanged
    let content = std::fs::read_to_string(&cfg).unwrap();
    assert_eq!(content, "not json {");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn future_tool_is_reserved_not_panicking() {
    let dir = tempfile_dir();
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "unknown_future_tool_xyz",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    // Should not panic; should report unknown/unsupported cleanly
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stderr.contains("unknown tool")
            || stdout.contains("unknown tool")
            || stderr.contains("Supported"),
        "should mention supported tools: stderr={}, stdout={}",
        stderr,
        stdout
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn run_install_claude(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "claude",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_claude_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_claude(&dir);
    assert!(
        out.status.success(),
        "install claude should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_creates_claude_md() {
    let dir = tempfile_dir();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    let claude_md = dir.join("CLAUDE.md");
    assert!(
        claude_md.is_file(),
        "CLAUDE.md should exist at {}",
        claude_md.display()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    let host_content = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert!(
        host_content.contains("## aimt"),
        "should contain marker ## aimt"
    );
    assert!(
        host_content.contains(".agents/aimt/index.md"),
        "host file should reference .agents/aimt/index.md"
    );
    // Minimal 5-file architecture — verify exact 5 files and key prompts (Tasks 6/7)
    let aimt_dir = dir.join(".agents/aimt");
    let mut names: Vec<String> = std::fs::read_dir(&aimt_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "core.md",
            "index.md",
            "mapping.md",
            "operations.md",
            "update.md"
        ],
        "should be exactly 5 minimal files, got {:?}",
        names
    );
    // index is operating protocol/router, not verbose guidance
    let index = std::fs::read_to_string(aimt_dir.join("index.md")).unwrap();
    assert!(
        index.contains("| File | Responsibility |"),
        "index should be protocol table"
    );
    assert!(
        index.contains(".aimt") || index.contains("AIMT"),
        "index should reference AIMT"
    );
    // core preserves ownership and 18 fields
    let core = std::fs::read_to_string(aimt_dir.join("core.md")).unwrap();
    assert!(
        core.contains("One fact, one owner, many references."),
        "core should preserve ownership verbatim"
    );
    assert!(
        core.contains("18 fields") || core.contains("18-field"),
        "core should state 18 fields"
    );
    // mapping / update have deterministic triggers
    let mapping = std::fs::read_to_string(aimt_dir.join("mapping.md")).unwrap();
    assert!(
        mapping.contains("/aimt ."),
        "mapping should trigger on /aimt ."
    );
    let update = std::fs::read_to_string(aimt_dir.join("update.md")).unwrap();
    assert!(
        update.contains("/aimt update"),
        "update should trigger on /aimt update"
    );
    let ops = std::fs::read_to_string(aimt_dir.join("operations.md")).unwrap();
    assert!(
        ops.contains("OPEN") && ops.contains("CLOSE"),
        "operations should cover lifecycle"
    );
    assert!(
        !aimt_dir.join("authorization.md").exists(),
        "fragmented authorization.md should not exist in minimal arch"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_preserves_existing_claude_md() {
    let dir = tempfile_dir();
    let claude_md = dir.join("CLAUDE.md");
    std::fs::write(
        &claude_md,
        "# My Claude Project\n\nExisting instructions.\n",
    )
    .unwrap();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&claude_md).unwrap();
    assert!(
        content.contains("My Claude Project"),
        "should preserve existing CLAUDE.md header"
    );
    assert!(
        content.contains("Existing instructions"),
        "should preserve existing body"
    );
    assert!(content.contains("## aimt"), "should add aimt section");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_claude(&dir);
    assert!(out1.status.success());
    let content1 = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    let out2 = run_install_claude(&dir);
    assert!(
        out2.status.success(),
        "second install should succeed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let content2 = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    assert_eq!(
        content1, content2,
        "content should be identical after second install"
    );
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.to_lowercase().contains("aimt installed for")
            || stdout2.contains("skipped")
            || stdout2.contains("already"),
        "second run should report installed or skipped: {}",
        stdout2
    );
    assert_eq!(
        content2.matches("## aimt").count(),
        1,
        "marker should appear exactly once"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_no_duplicate_content() {
    let dir = tempfile_dir();
    let out1 = run_install_claude(&dir);
    assert!(out1.status.success());
    let out2 = run_install_claude(&dir);
    assert!(out2.status.success());
    let content = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
    // Ensure guide content not duplicated
    assert_eq!(
        content.matches("## aimt").count(),
        1,
        "should not duplicate aimt section"
    );
    // Ensure plugin entry not duplicated (for claude there is no plugin file, but check no duplicate guide)
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_claude_nonexistent_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!dir.exists());
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "claude",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "should fail for missing project dir");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("invalid project"),
        "should explain: {}",
        stderr
    );
}

#[test]
fn install_claude_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    // Ensure opencode files are not touched when installing claude
    assert!(
        !dir.join(".opencode").exists(),
        "claude install should not create .opencode"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_does_not_affect_opencode() {
    let dir = tempfile_dir();
    // First install opencode
    let out_opencode = run_install(&dir);
    assert!(out_opencode.status.success());
    let opencode_claude_content_before =
        std::fs::read_to_string(dir.join("AGENTS.md")).unwrap_or_default();
    // Then install claude
    let out_claude = run_install_claude(&dir);
    assert!(out_claude.status.success());
    // Opencode files should still exist and be unchanged (except AGENTS.md vs CLAUDE.md are separate)
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    assert!(dir.join(".opencode/opencode.json").is_file());
    // CLAUDE.md should be created, AGENTS.md should be preserved (not overwritten by claude)
    assert!(dir.join("CLAUDE.md").is_file());
    // If AGENTS.md existed before, it should still contain aimt marker (from opencode)
    if !opencode_claude_content_before.is_empty() {
        let agents_content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents_content.contains("## aimt"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_creates_settings_json() {
    let dir = tempfile_dir();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    let cfg = dir.join(".claude").join("settings.json");
    assert!(
        cfg.is_file(),
        "settings.json should exist at {}",
        cfg.display()
    );
    let content = std::fs::read_to_string(&cfg).unwrap();
    assert!(
        content.contains("PreToolUse"),
        "should contain PreToolUse hook"
    );
    assert!(content.contains("aimt"), "should contain aimt hook");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_settings_hook_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_claude(&dir);
    assert!(out1.status.success());
    let content1 = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    let out2 = run_install_claude(&dir);
    assert!(out2.status.success());
    let content2 = std::fs::read_to_string(dir.join(".claude/settings.json")).unwrap();
    assert_eq!(
        content1, content2,
        "settings.json should be identical after second install"
    );
    assert_eq!(
        content2.matches("AIMT: .aimt knowledge").count(),
        1,
        "hook should appear exactly once"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_claude_preserves_existing_settings_json() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".claude").join("settings.json");
    std::fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cfg_path,
        r#"{"permissions":{"allow":["Bash"]},"extra":"keep"}"#,
    )
    .unwrap();
    let out = run_install_claude(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(
        content.contains("keep"),
        "should preserve extra key: {}",
        content
    );
    assert!(
        content.contains("allow") || content.contains("permissions"),
        "should preserve permissions: {}",
        content
    );
    assert!(content.contains("aimt"), "should add aimt hook");
    // Should not duplicate hook
    let out2 = run_install_claude(&dir);
    assert!(out2.status.success());
    let content2 = std::fs::read_to_string(&cfg_path).unwrap();
    assert_eq!(
        content2.matches("AIMT: .aimt knowledge").count(),
        1,
        "should not duplicate hook"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn run_install_codex(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "codex",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_codex_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_codex(&dir);
    assert!(
        out.status.success(),
        "install codex should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_creates_integration_files() {
    let dir = tempfile_dir();
    let out = run_install_codex(&dir);
    assert!(out.status.success());
    assert!(dir.join("AGENTS.md").is_file(), "AGENTS.md should exist");
    assert!(
        dir.join(".agents/skills/aimt/SKILL.md").is_file(),
        "Codex skill should exist at .agents/skills/aimt/SKILL.md"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_codex(&dir);
    assert!(out.status.success());
    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("## aimt"),
        "AGENTS.md should contain marker"
    );
    assert!(agents.contains(".aimt"), "should mention .aimt");
    assert!(
        agents.to_lowercase().contains("when") && agents.contains("AIMT"),
        "should mention when to use"
    );
    let skill = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    assert!(skill.contains("aimt"), "skill should mention aimt");
    assert!(skill.contains(".aimt"), "skill should mention .aimt file");
    assert!(
        skill.to_lowercase().contains("discover") || skill.contains("Discover"),
        "skill should mention discover"
    );
    assert!(
        skill.contains("search") && skill.contains("read"),
        "skill should mention search/read"
    );
    assert!(
        skill.contains("follow_parent") || skill.contains("follow parent"),
        "skill should mention follow parent"
    );
    assert!(
        skill.contains("follow_file") || skill.contains("follow file"),
        "skill should mention follow file"
    );
    assert!(skill.contains("relation"), "skill should mention relation");
    assert!(skill.contains("validate"), "skill should mention validate");
    assert!(
        skill.to_lowercase().contains("hosted") && skill.to_lowercase().contains("install"),
        "skill should explain Hosted vs Install"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_preserves_existing_agents_md() {
    let dir = tempfile_dir();
    let agents = dir.join("AGENTS.md");
    std::fs::write(&agents, "# My Project\n\nExisting content.\n").unwrap();
    let out = run_install_codex(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&agents).unwrap();
    assert!(content.contains("My Project"), "should preserve header");
    assert!(content.contains("Existing content"), "should preserve body");
    assert!(content.contains("## aimt"), "should add aimt section");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_preserves_existing_skill_files() {
    let dir = tempfile_dir();
    let other_skill = dir.join(".agents/skills/other/SKILL.md");
    std::fs::create_dir_all(other_skill.parent().unwrap()).unwrap();
    std::fs::write(&other_skill, "# other skill\n").unwrap();
    let out = run_install_codex(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_skill).unwrap(),
        "# other skill\n",
        "should preserve other skill"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_codex(&dir);
    assert!(out1.status.success());
    let agents1 = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let skill1 = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    let out2 = run_install_codex(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let agents2 = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    let skill2 = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    assert_eq!(agents1, agents2, "AGENTS.md should be identical");
    assert_eq!(skill1, skill2, "skill should be identical");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.to_lowercase().contains("aimt installed for")
            || stdout2.contains("skipped")
            || stdout2.contains("already"),
        "should report installed or skipped: {}",
        stdout2
    );
    assert_eq!(agents2.matches("## aimt").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_no_duplicate_content() {
    let dir = tempfile_dir();
    let out1 = run_install_codex(&dir);
    assert!(out1.status.success());
    let out2 = run_install_codex(&dir);
    assert!(out2.status.success());
    let agents = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert_eq!(agents.matches("## aimt").count(), 1, "should not duplicate");
    let skill = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    // Skill should not be duplicated - just check file still single and contains marker once
    assert!(skill.contains("aimt"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_codex_nonexistent_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!dir.exists());
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "codex",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("invalid project"),
        "should explain: {}",
        stderr
    );
}

#[test]
fn install_codex_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_codex(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_codex = run_install_codex(&dir);
    assert!(out_codex.status.success());
    // Codex should create AGENTS.md and skill, but not claude/opencode specific files
    assert!(dir.join("AGENTS.md").is_file());
    assert!(dir.join(".agents/skills/aimt/SKILL.md").is_file());
    // Should not create .claude or .opencode by default (those are for other tools)
    // But AGENTS.md is shared with opencode, so installing codex then opencode should preserve both
    let out_opencode = run_install(&dir);
    assert!(out_opencode.status.success());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    // Both should coexist
    let agents_content = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(agents_content.contains("## aimt"));
    assert!(dir.join(".agents/skills/aimt/SKILL.md").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

fn run_install_antigravity(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "antigravity",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_antigravity_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_antigravity(&dir);
    assert!(
        out.status.success(),
        "install antigravity should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_creates_skill() {
    let dir = tempfile_dir();
    let out = run_install_antigravity(&dir);
    assert!(out.status.success());
    let skill = dir.join(".agents/skills/aimt/SKILL.md");
    assert!(skill.is_file(), "skill should exist at {}", skill.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_skill_has_frontmatter() {
    let dir = tempfile_dir();
    let out = run_install_antigravity(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    assert!(content.starts_with("---"), "should start with frontmatter");
    assert!(content.contains("name: aimt"), "should contain name: aimt");
    assert!(
        content.contains("description:"),
        "should contain description"
    );
    // Ensure frontmatter is valid YAML delimiters
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    assert!(parts.len() >= 3, "should have closing --- for frontmatter");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_antigravity(&dir);
    assert!(out.status.success());
    let skill = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    assert!(
        skill.contains("##") || skill.contains("AIMT"),
        "should contain AIMT header"
    );
    assert!(skill.contains(".aimt"), "should mention .aimt");
    assert!(
        skill.to_lowercase().contains("when") && skill.contains("AIMT"),
        "should mention when to use"
    );
    assert!(
        skill.contains("discover") || skill.contains("Discover"),
        "should mention discover"
    );
    assert!(
        skill.contains("search") && skill.contains("read"),
        "should mention search/read"
    );
    assert!(
        skill.contains("follow_parent") || skill.contains("follow parent"),
        "should mention follow parent"
    );
    assert!(
        skill.contains("follow_file") || skill.contains("follow file"),
        "should mention follow file"
    );
    assert!(skill.contains("relation"), "should mention relation");
    assert!(skill.contains("validate"), "should mention validate");
    assert!(
        skill.to_lowercase().contains("update") || skill.to_lowercase().contains("permission"),
        "should mention update/permissions"
    );
    assert!(
        skill.to_lowercase().contains("hosted") && skill.to_lowercase().contains("install"),
        "should explain Hosted vs Install"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_preserves_existing_skills() {
    let dir = tempfile_dir();
    let other_skill = dir.join(".agents/skills/other/SKILL.md");
    std::fs::create_dir_all(other_skill.parent().unwrap()).unwrap();
    std::fs::write(&other_skill, "# other skill\n").unwrap();
    let out = run_install_antigravity(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_skill).unwrap(),
        "# other skill\n",
        "should preserve other skill"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_preserves_existing_rules() {
    let dir = tempfile_dir();
    let rule = dir.join(".agents/rules/custom.md");
    std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
    std::fs::write(&rule, "# custom rule\n").unwrap();
    let out = run_install_antigravity(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&rule).unwrap(),
        "# custom rule\n",
        "should preserve existing rules"
    );
    // Should not create .agents/rules/aimt.md by default (skill preferred)
    assert!(
        !dir.join(".agents/rules/aimt.md").exists(),
        "should not create rules file by default"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_antigravity(&dir);
    assert!(out1.status.success());
    let skill1 = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    let out2 = run_install_antigravity(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let skill2 = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    assert_eq!(skill1, skill2, "skill should be identical");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.to_lowercase().contains("aimt installed for")
            || stdout2.contains("skipped")
            || stdout2.contains("already"),
        "should report installed or skipped: {}",
        stdout2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_no_duplicate_content() {
    let dir = tempfile_dir();
    let out1 = run_install_antigravity(&dir);
    assert!(out1.status.success());
    let out2 = run_install_antigravity(&dir);
    assert!(out2.status.success());
    let skill = std::fs::read_to_string(dir.join(".agents/skills/aimt/SKILL.md")).unwrap();
    // Should contain aimt marker only once logically (skill file not duplicated)
    assert!(skill.contains("aimt"));
    // File should still be single, not appended twice
    assert_eq!(
        skill.matches("name: aimt").count(),
        1,
        "should not duplicate frontmatter"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_antigravity_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_antigravity_nonexistent_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!dir.exists());
    let out = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "antigravity",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("invalid project"),
        "should explain: {}",
        stderr
    );
}

#[test]
fn install_antigravity_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_anti = run_install_antigravity(&dir);
    assert!(out_anti.status.success());
    assert!(dir.join(".agents/skills/aimt/SKILL.md").is_file());
    // Should not create opencode or claude specific files
    assert!(
        !dir.join("CLAUDE.md").exists()
            || std::fs::read_to_string(dir.join("CLAUDE.md"))
                .unwrap_or_default()
                .is_empty()
            || !dir.join(".claude").exists(),
        "antigravity should not create CLAUDE.md by default"
    );
    // Install opencode afterwards should preserve antigravity skill
    let out_opencode = run_install(&dir);
    assert!(out_opencode.status.success());
    assert!(dir.join(".agents/skills/aimt/SKILL.md").is_file());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn existing_adapters_remain_unchanged() {
    // Verify GenericAdapter still works for all tools — proves we didn't break cross-tool portability
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    let claude = aimt::plugins::claude::ClaudeAdapter;
    let codex = aimt::plugins::codex::CodexAdapter;
    let antigravity = aimt::plugins::antigravity::AntigravityAdapter;
    use aimt::plugins::adapter::WorkflowAdapter;
    assert_eq!(opencode.id(), "opencode");
    assert_eq!(claude.id(), "claude");
    assert_eq!(codex.id(), "codex");
    assert_eq!(antigravity.id(), "antigravity");
    // All translate via GenericAdapter — ensure no drift
    let cap = aimt::workflows::capabilities::Capability::Search;
    let sug = aimt::workflows::engine::Suggestion::Execute(cap);
    // We can't clone Suggestion, so test via string matching on HostAction display
    let action = opencode.translate(sug);
    assert!(action.to_string().contains("ToolCall"));
}

#[test]
fn shared_guide_contains_authorization_rule() {
    let guide = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/plugins/installer/guide_body.md"
    ))
    .unwrap();
    let lower = guide.to_lowercase();
    // Core authorization flow must be defined once in shared guide
    assert!(
        guide.contains("Authorization before writes"),
        "shared guide must define Authorization before writes"
    );
    assert!(
        guide.contains("Check AIMT authorization") || guide.contains("Check authorization"),
        "guide must tell AI to check authorization"
    );
    assert!(
        lower.contains("not authorized") && lower.contains("authorized"),
        "guide must distinguish Authorized vs Not authorized"
    );
}

#[test]
fn guide_authorization_covers_required_behaviors() {
    let guide = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/plugins/installer/guide_body.md"
    ))
    .unwrap();
    let lower = guide.to_lowercase();
    // Unauthenticated must stop before CRUD
    assert!(
        guide.contains("STOP") || lower.contains("stop the write"),
        "guide must tell AI to STOP when not authorized"
    );
    assert!(
        guide.contains("insert")
            && guide.contains("update")
            && guide.contains("remove")
            && guide.contains("persist"),
        "guide must list insert/update/remove/persist as blocked when not authorized"
    );
    // Must tell user to authorize/login
    assert!(
        lower.contains("please authorize") || lower.contains("aimt login"),
        "guide must ask user to authorize/login"
    );
    // Must never request private key
    assert!(
        lower.contains("do not ask") && lower.contains("private key"),
        "guide must forbid asking for private key"
    );
    assert!(
        lower.contains("do not request") && lower.contains("private key"),
        "guide must forbid requesting private key through AI"
    );
    // Must forbid modifying security metadata
    assert!(
        guide.contains("owner_public_key") && guide.contains("open_to_read"),
        "guide must forbid modifying owner_public_key/open_to_read"
    );
    // Read/search remains allowed
    assert!(
        lower.contains("read access is public")
            || (guide.contains("discover")
                && guide.contains("search")
                && guide.contains("read")
                && guide.contains("follow")
                && guide.contains("validate")),
        "guide must state read/search/follow/validate remain allowed"
    );
    // Authorized does not bypass workflow permissions
    assert!(
        lower.contains("does not bypass") || lower.contains("authorization does not bypass"),
        "guide must state authorization does not bypass workflow permissions"
    );
    assert!(
        guide.contains("HostedEngine") && lower.contains("read-only"),
        "guide must state HostedEngine remains read-only"
    );
    // Changing open_to_read never grants write
    assert!(
        lower.contains("open_to_read") && lower.contains("never grant"),
        "guide must state open_to_read change never grants write"
    );
    // Must be concise, not cryptographic details
    assert!(
        !lower.contains("ed25519") && !lower.contains("private key cryptography"),
        "guide should not expose cryptographic implementation details"
    );
}

#[test]
fn all_plugins_include_authorization_rule() {
    // Updated for minimal 5-file architecture (Tasks 6/7): authorization.md fragmented file removed;
    // verify exactly 5 files (index, core, mapping, operations, update) and host references .agents/aimt/
    let tools = [
        "opencode",
        "claude",
        "codex",
        "antigravity",
        "kilo",
        "kimi",
        "cursor",
        "copilot",
        "aider",
        "gemini",
    ];
    for tool in tools {
        let dir = tempfile_dir();
        let out = std::process::Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--bin",
                "aimt",
                "--",
                "install",
                tool,
                "--path",
                dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "install {} should succeed: stderr={}",
            tool,
            String::from_utf8_lossy(&out.stderr)
        );
        let aimt_dir = dir.join(".agents/aimt");
        assert!(
            aimt_dir.is_dir(),
            "tool {} should create .agents/aimt/",
            tool
        );
        let mut names: Vec<String> = std::fs::read_dir(&aimt_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "core.md",
                "index.md",
                "mapping.md",
                "operations.md",
                "update.md"
            ],
            "tool {} should have exactly 5 minimal files, got {:?}",
            tool,
            names
        );
        assert!(
            !aimt_dir.join("authorization.md").exists(),
            "tool {} should not create fragmented authorization.md in minimal arch",
            tool
        );
        // Ensure none of the 5 expose raw private key secrets
        for name in &names {
            let content = std::fs::read_to_string(aimt_dir.join(name)).unwrap();
            assert!(
                !content.to_lowercase().contains("private key:")
                    || content.contains("Do NOT ask")
                    || content.contains("Do not"),
                "tool {} file {} must not expose private key",
                tool,
                name
            );
        }
        // Host files should be minimal and reference the new 5-file system
        let candidates = [
            dir.join("AGENTS.md"),
            dir.join("CLAUDE.md"),
            dir.join(".agents/skills/aimt/SKILL.md"),
            dir.join(".kilo/skills/aimt/SKILL.md"),
            dir.join(".kimi/skills/aimt/SKILL.md"),
            dir.join(".cursor/rules/aimt.mdc"),
            dir.join(".github/copilot-instructions.md"),
            dir.join(".aider/aimt.md"),
            dir.join("GEMINI.md"),
        ];
        let mut host_found = false;
        for p in candidates {
            if p.is_file() {
                let content = std::fs::read_to_string(&p).unwrap();
                if content.contains(".agents/aimt/") {
                    host_found = true;
                    break;
                }
            }
        }
        assert!(
            host_found,
            "tool {} should generate host file referencing .agents/aimt/",
            tool
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn guide_does_not_expose_secrets() {
    // Minimal 5-file architecture (Tasks 6/7): check only existing prompts + legacy guide_body
    let mut paths = vec![
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/plugins/installer/guide_body.md"
        )
        .to_string(),
    ];
    for name in [
        "index.md",
        "core.md",
        "mapping.md",
        "operations.md",
        "update.md",
    ] {
        paths.push(format!(
            "{}/src/plugins/installer/aimt/{}",
            env!("CARGO_MANIFEST_DIR"),
            name
        ));
    }
    for p in paths {
        let guide = std::fs::read_to_string(&p).unwrap();
        assert!(
            !guide.contains("private_key") || guide.contains("Do NOT ask"),
            "guide {} may mention private_key only to forbid it",
            p
        );
        let hex_tokens: Vec<&str> = guide
            .split(|c: char| !c.is_ascii_hexdigit())
            .filter(|s| s.len() == 64)
            .collect();
        assert!(
            hex_tokens.is_empty(),
            "guide {} must not contain 64-char hex, found {:?}",
            p,
            hex_tokens
        );
    }
}

#[test]
fn installer_index_is_sorted_table() {
    let idx = aimt::plugins::installer::aimt::INDEX_MD;
    // New protocol: index is operating protocol/router, not merely file index
    assert!(
        idx.contains("# AIMT Operating Protocol"),
        "index should be operating protocol"
    );
    assert!(idx.contains("## Purpose"), "index should have Purpose");
    assert!(
        idx.contains("## Operating Modes"),
        "index should have Operating Modes"
    );
    assert!(
        idx.contains("## Mode Routing"),
        "index should have Mode Routing"
    );
    assert!(
        idx.contains("## Normal Project Work"),
        "index should have Normal Project Work"
    );
    assert!(
        idx.contains("## Instruction Files"),
        "index should have Instruction Files"
    );
    assert!(
        idx.contains("## Operating Rules"),
        "index should have Operating Rules"
    );
    // Three operating modes
    assert!(idx.contains("/aimt ."), "index should define /aimt . mode");
    assert!(
        idx.contains("/aimt update"),
        "index should define /aimt update mode"
    );
    assert!(
        idx.to_lowercase().contains("normal project work"),
        "index should define normal project work mode"
    );
    // Instruction routing
    assert!(
        idx.contains("mapping.md"),
        "index should route to mapping.md"
    );
    assert!(idx.contains("update.md"), "index should route to update.md");
    assert!(
        idx.contains("operations.md"),
        "index should route to operations.md"
    );
    assert!(idx.contains("core.md"), "index should reference core.md");
    // Instruction Files table
    assert!(
        idx.contains("| File | Responsibility |"),
        "index should have File|Responsibility table"
    );
    // No dev dependency
    assert!(
        !idx.contains("docs/specification"),
        "index should not depend on dev repository"
    );
    // Normal work conditional
    assert!(
        idx.contains("Do not update AIMT merely because a chat or task ended")
            || idx.contains("Do not update AIMT merely because a chat"),
        "index should state AIMT updated only when knowledge changes"
    );
}

#[test]
fn installer_operations_covers_lifecycle_without_hardcoded_assumptions() {
    let ops = aimt::plugins::installer::aimt::OPERATIONS_MD;
    assert!(ops.contains("OPEN"), "operations should cover OPEN");
    assert!(ops.contains("READ"), "operations should cover READ");
    assert!(
        ops.contains("follow") || ops.contains("follow_"),
        "operations should cover follow/navigation"
    );
    assert!(ops.contains("WRITE"), "operations should cover WRITE");
    assert!(ops.contains("VALIDATE"), "operations should cover VALIDATE");
    assert!(ops.contains("CLOSE"), "operations should cover CLOSE");
    // Must not require search before read as fundamental
    assert!(
        !ops.contains("Search before read"),
        "operations should not require search before read"
    );
    assert!(
        !ops.contains("search before read"),
        "operations should not require search before read (case insensitive)"
    );
    // Must not hardcode unsupported specifics
    assert!(
        !ops.contains("Store::open_mut"),
        "operations should not hardcode Store::open_mut"
    );
    assert!(
        !ops.contains("aimt init"),
        "operations should not hardcode aimt init unless supported"
    );
    assert!(
        !ops.contains("aimt login"),
        "operations should not hardcode aimt login unless supported"
    );
    assert!(
        !ops.contains("^[a-z]"),
        "operations should not hardcode ID regex"
    );
    // Normal operation: determines relevance, conditional AIMT usage
    assert!(
        ops.contains("Determine AIMT relevance")
            || ops.contains("determine whether the current user request"),
        "operations should start with determining AIMT relevance"
    );
    assert!(
        ops.contains("existing .aimt") || ops.contains("existing `.aimt`"),
        "operations should distinguish update opening existing .aimt"
    );
    assert!(
        ops.lines().count() < 350,
        "operations should be focused, got {} lines",
        ops.lines().count()
    );
    // Verify 7 steps are present
    assert!(
        ops.matches("### Step").count() == 7,
        "operations should have exactly 7 steps, got {}",
        ops.matches("### Step").count()
    );
    // Verify conditional behavior and no-update guard
    assert!(
        ops.contains("Do not update AIMT merely because a conversation or task ended"),
        "operations should guard against updating merely because task ended"
    );
}

#[test]
fn installer_core_md_is_minimal_and_complete() {
    let core = aimt::plugins::installer::aimt::CORE_MD;
    assert!(
        core.contains("@aimt") && core.contains("@map") && core.contains("@domain"),
        "core should list levels"
    );
    assert!(
        core.contains("@region")
            && core.contains("@node")
            && core.contains("@file")
            && core.contains("@frame"),
        "core should list all 7 levels"
    );
    assert!(
        core.contains("One fact, one owner, many references."),
        "core should preserve ownership principle verbatim"
    );
    assert!(
        core.contains("18 fields") || core.contains("18-field"),
        "core should state 18-field vocabulary"
    );
    for f in [
        "id",
        "title",
        "description",
        "summary",
        "parent",
        "type",
        "source",
        "context",
        "path",
        "target",
        "location",
        "hash",
        "from",
        "to",
        "evidence",
        "version",
        "created",
        "updated",
    ] {
        assert!(
            core.contains(f),
            "core should mention field {} got {}",
            f,
            &core[..500.min(core.len())]
        );
    }
    // No invented fields
    assert!(
        !core.contains("22 fields") && !core.contains("20 fields"),
        "core must not claim 22/20 fields"
    );
    // Must not hardcode unsupported rules
    assert!(
        !core.contains("^[a-z]"),
        "core should not hardcode ID regex"
    );
    assert!(
        !core.contains("class|function|method|variable")
            && !core.contains("class/function/method/variable"),
        "core should not hardcode frame type enumeration unless spec cites it"
    );
    // Must describe canonical vs evidence vs instructions
    assert!(
        core.contains(".aimt") && (core.contains("canonical") || core.contains("source of truth")),
        "core should state .aimt canonical"
    );
    assert!(core.contains("evidence"), "core should mention evidence");
    assert!(
        core.contains("instructions") || core.contains(".agents/aimt"),
        "core should distinguish instructions layer"
    );
    // Populating applicable optional fields
    assert!(
        core.contains("applicable") || core.contains("optional"),
        "core should instruct to populate applicable optional fields"
    );
    assert!(
        core.lines().count() < 400,
        "core.md should be focused, got {} lines",
        core.lines().count()
    );
}

#[test]
fn installer_mapping_prompt_covers_initial_aimt() {
    let m = aimt::plugins::installer::aimt::MAPPING_MD;
    assert!(
        m.contains("/aimt .") || m.contains("`/aimt .`"),
        "mapping should trigger on /aimt ."
    );
    // Must not broaden to arbitrary phrases
    assert!(
        !m.contains("map my project") && !m.contains("initialize AIMT"),
        "mapping trigger should be deterministic, not broad phrases"
    );
    assert!(
        m.contains("understand") || m.contains("Understand"),
        "should instruct to understand the project"
    );
    assert!(
        m.contains("connected") || m.contains("relations") || m.contains("relation"),
        "should require connected knowledge via relations"
    );
    assert!(m.contains("evidence"), "should require evidence");
    assert!(
        m.contains("validate") || m.contains("VALIDATE"),
        "should validate"
    );
    assert!(
        m.contains("persist") || m.contains("PERSIST"),
        "should persist"
    );
    assert!(
        m.contains("One fact, one owner, many references."),
        "should preserve ownership principle verbatim"
    );
    // Applicable fields including optional
    assert!(
        m.contains("applicable") && (m.contains("optional") || m.contains("Optional")),
        "mapping should instruct to populate all applicable fields including optional"
    );
    for f in ["description", "evidence", "hash", "location"] {
        assert!(
            m.contains(f),
            "mapping should mention applicable field {}",
            f
        );
    }
    // Must forbid creating development directory, distinguish layers
    assert!(
        m.contains(".aimt") && (m.contains("canonical") || m.contains("source of truth")),
        "should state .aimt canonical"
    );
    assert!(
        m.contains("Do NOT") || m.contains("Do not"),
        "should forbid separate development directory"
    );
    // Must not hardcode unsupported API specifics
    assert!(
        !m.contains("Store::open_mut")
            && !m.contains("aimt login")
            && !m.contains("aimt init")
            && !m.contains("^[a-z]"),
        "mapping should not hardcode unsupported implementation specifics"
    );
    // Should list hierarchy at high level without hardcoding type enumerations
    assert!(
        m.contains("@domain") && m.contains("@file") && m.contains("@frame"),
        "mapping should mention hierarchy levels at high level"
    );
}

#[test]
fn installer_update_prompt_covers_incremental() {
    let u = aimt::plugins::installer::aimt::UPDATE_MD;
    assert!(u.contains("/aimt update"), "should trigger on /aimt update");
    // Deterministic only
    assert!(
        !u.contains("map my project"),
        "update trigger should be deterministic"
    );
    assert!(
        u.contains("open") || u.contains("OPEN"),
        "should open existing .aimt"
    );
    assert!(
        u.contains("existing") || u.contains("Existing"),
        "should instruct to understand existing knowledge first"
    );
    assert!(
        u.contains("preserve") || u.contains("stable"),
        "should preserve stable IDs"
    );
    assert!(
        u.contains("affected") || u.contains("incremental") || u.contains("only"),
        "should update only affected knowledge"
    );
    assert!(
        u.contains("relations") || u.contains("relation"),
        "should handle relations"
    );
    assert!(
        u.contains("evidence") && u.contains("hash"),
        "should handle evidence/hashes"
    );
    assert!(
        u.contains("validate") || u.contains("VALIDATE"),
        "should validate"
    );
    assert!(
        u.contains("persist") || u.contains("PERSIST"),
        "should persist"
    );
    // Architecture unchanged
    assert!(u.contains(".aimt"), "should mention .aimt canonical");
    // Must not hardcode unsupported APIs
    assert!(
        !u.contains("Store::open_mut") && !u.contains("aimt login") && !u.contains("aimt init"),
        "update should not hardcode unsupported specifics"
    );
    // Must not rebuild from zero
    assert!(
        u.contains("Do not rebuild")
            || u.contains("not rebuild")
            || u.contains("maintain")
            || u.contains("incremental"),
        "should forbid full rebuild"
    );
}

#[test]
fn installer_modules_are_exactly_five() {
    let modules = aimt::plugins::installer::aimt::MODULES;
    assert_eq!(
        modules.len(),
        5,
        "minimal modules should be exactly 5, got {:?}",
        modules.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
    let names: Vec<_> = modules.iter().map(|(n, _)| *n).collect();
    assert!(names.contains(&"index.md"), "must contain index.md");
    assert!(names.contains(&"core.md"), "must contain core.md");
    assert!(
        names.contains(&"operations.md"),
        "must contain operations.md"
    );
    assert!(names.contains(&"mapping.md"), "must contain mapping.md");
    assert!(names.contains(&"update.md"), "must contain update.md");
    for frag in [
        "search.md",
        "read.md",
        "follow.md",
        "validate.md",
        "fields.md",
        "levels.md",
        "project.md",
        "authorization.md",
        "errors.md",
        "knowledge.md",
        "schema.md",
        "security.md",
        "write.md",
        "examples.md",
        "README.md",
    ] {
        assert!(
            !names.contains(&frag),
            "minimal should not contain fragmented {}",
            frag
        );
    }
}

#[test]
fn host_guide_is_minimal_entry_point_without_mandatory_gate() {
    let guide = aimt::plugins::installer::template::AIMT_HOST_GUIDE_MD;
    assert!(guide.contains("## aimt"), "should contain marker");
    assert!(
        guide.contains(".agents/aimt/index.md"),
        "should point to .agents/aimt/index.md"
    );
    assert!(
        guide.lines().count() < 15,
        "host guide should be small, got {} lines: {}",
        guide.lines().count(),
        guide
    );
    assert!(
        guide.len() < 900,
        "host guide should be concise, got {} chars",
        guide.len()
    );
    assert!(
        guide.contains(".aimt")
            && (guide.contains("canonical") || guide.contains("source of truth")),
        "should state .aimt is canonical"
    );
    // Must NOT be mandatory gate before source
    assert!(
        !guide.contains("before answering") && !guide.contains("before reading source"),
        "host guide should not mandate reading AIMT before source"
    );
    // Must state conceptual relationship and stale/missing handling
    assert!(
        guide.contains("primary knowledge map") || guide.contains("knowledge map when available"),
        "should state AIMT is primary knowledge map when available"
    );
    assert!(
        guide.contains("evidence"),
        "should state source is evidence"
    );
    assert!(
        guide.contains("missing") || guide.contains("stale") || guide.contains("insufficient"),
        "should mention inspecting source when AIMT missing/stale/insufficient"
    );
    // Must NOT contain complete list of every prompt file
    assert!(
        !guide.contains("| Title | File |"),
        "host guide should not duplicate index table"
    );
    assert!(
        guide.contains("Do not create")
            || guide.contains("Do NOT")
            || guide.contains("no separate"),
        "should forbid separate project dir at high level"
    );
}

#[test]
fn opencode_aimt_command_dispatches_strictly_and_reads_files() {
    let cmd = aimt::plugins::opencode::template::OPENCODE_AIMT_COMMAND_MD;
    assert!(
        cmd.contains("$ARGUMENTS"),
        "command template should use $ARGUMENTS"
    );
    // Strict dispatch
    assert!(
        cmd.contains("\".\"")
            || cmd.contains("'.'")
            || cmd.contains("` . `")
            || cmd.contains("is \".\""),
        "should handle \".\" argument"
    );
    assert!(cmd.contains("update"), "should handle \"update\" argument");
    // Must explicitly instruct to read files, not just mention path
    assert!(
        cmd.contains("Read `.agents/aimt/index.md`")
            || cmd.contains("read `.agents/aimt/index.md`")
            || cmd.to_lowercase().contains("read") && cmd.contains("index.md"),
        "should instruct to read index.md"
    );
    assert!(
        cmd.contains("read `.agents/aimt/mapping.md`")
            || cmd.to_lowercase().contains("read") && cmd.contains("mapping.md"),
        "should instruct to read mapping.md"
    );
    assert!(
        cmd.contains("read `.agents/aimt/update.md`")
            || cmd.to_lowercase().contains("read") && cmd.contains("update.md"),
        "should instruct to read update.md"
    );
    // Dispatch destinations
    assert!(
        cmd.contains(".agents/aimt/mapping.md"),
        "should dispatch \".\" to mapping.md"
    );
    assert!(
        cmd.contains(".agents/aimt/update.md"),
        "should dispatch \"update\" to update.md"
    );
    // Strict unsupported handling
    assert!(
        cmd.contains("unsupported") || cmd.contains("only supports"),
        "should report unsupported for anything else"
    );
    // Must not create separate command names or broad synonyms
    assert!(
        !cmd.contains("/aimt-mapping") && !cmd.contains("/aimt-update"),
        "should be one /aimt command, not separate names"
    );
    // Must not suggest empty/other fallback to ask user
    assert!(
        !cmd.to_lowercase()
            .contains("ask the user whether they want"),
        "should not fallback to asking user for empty/unknown"
    );
    // Must contain follow instructions language
    assert!(
        cmd.contains("follow") || cmd.contains("Follow"),
        "should instruct to follow prompt file instructions"
    );
}

#[test]
fn install_produces_minimal_agents_structure_with_command() {
    let dir = tempfile_dir();
    let proj = &dir;
    std::fs::create_dir_all(proj).unwrap();
    let mut report = aimt::plugins::installer::report::InstallReport {
        tool: "opencode".into(),
        ..Default::default()
    };
    aimt::plugins::installer::guide::ensure_aimt_prompts(proj, &mut report).unwrap();
    let aimt_dir = proj.join(".agents").join("aimt");
    assert!(aimt_dir.is_dir(), ".agents/aimt should exist");
    let mut entries: Vec<String> = std::fs::read_dir(&aimt_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    entries.sort();
    assert_eq!(
        entries.len(),
        5,
        "installed .agents/aimt should have exactly 5 files, got {:?}",
        entries
    );
    assert!(entries.contains(&"index.md".to_string()));
    assert!(entries.contains(&"core.md".to_string()));
    assert!(entries.contains(&"mapping.md".to_string()));
    assert!(entries.contains(&"update.md".to_string()));
    assert!(entries.contains(&"operations.md".to_string()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn e2e_minimal_structure_and_chat_prompts() {
    let dir = tempfile::tempdir().unwrap();
    let proj = dir.path();
    let installer = aimt::plugins::installer::installer_for("opencode")
        .expect("opencode installer should exist");
    let _report = installer.install(proj).expect("install should succeed");
    let aimt_dir = proj.join(".agents").join("aimt");
    let mut names: Vec<String> = std::fs::read_dir(&aimt_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "core.md",
            "index.md",
            "mapping.md",
            "operations.md",
            "update.md"
        ],
        "should be exactly 5 minimal files, got {:?}",
        names
    );

    let index = std::fs::read_to_string(aimt_dir.join("index.md")).unwrap();
    assert!(
        index.contains("# AIMT Operating Protocol"),
        "index should be operating protocol: {}",
        &index[..200.min(index.len())]
    );
    assert!(
        index.contains("| File | Responsibility |"),
        "index should have File|Responsibility table"
    );
    assert!(
        index.contains("/aimt .") && index.contains("/aimt update"),
        "index should define three modes"
    );
    assert!(
        index.contains("## Mode Routing"),
        "index should have Mode Routing"
    );

    let mapping = std::fs::read_to_string(aimt_dir.join("mapping.md")).unwrap();
    assert!(
        mapping.contains("/aimt ."),
        "mapping should be /aimt . prompt"
    );
    assert!(
        mapping.contains("One fact, one owner, many references."),
        "mapping should preserve ownership verbatim"
    );
    let update = std::fs::read_to_string(aimt_dir.join("update.md")).unwrap();
    assert!(
        update.contains("/aimt update"),
        "update should be /aimt update prompt"
    );
    assert!(
        update.contains("One fact, one owner, many references.") || update.contains("One fact"),
        "update should preserve ownership"
    );

    // core.md 18 fields
    let core = std::fs::read_to_string(aimt_dir.join("core.md")).unwrap();
    assert!(core.contains("18 fields") || core.contains("18-field"));

    // operations.md normal operation lifecycle
    let ops = std::fs::read_to_string(aimt_dir.join("operations.md")).unwrap();
    assert!(
        ops.contains("Determine AIMT relevance")
            || ops.contains("determine whether the current user request"),
        "operations should start with determining AIMT relevance"
    );

    let agents = std::fs::read_to_string(proj.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("## aimt"),
        "AGENTS.md should contain marker"
    );
    assert!(
        agents.contains(".agents/aimt/index.md"),
        "AGENTS.md should point to index.md"
    );
    assert!(
        !agents.contains("before answering") && !agents.contains("before reading source"),
        "AGENTS.md should not be mandatory gate"
    );
    assert!(
        agents.contains("primary knowledge map") || agents.contains("knowledge map when available"),
        "AGENTS.md should state conceptual relationship"
    );
    assert!(
        agents.lines().count() < 20,
        "AGENTS.md should be minimal, got {}",
        agents.lines().count()
    );

    // OpenCode command strict dispatch that reads files
    let cmd = proj.join(".opencode").join("commands").join("aimt.md");
    assert!(cmd.is_file(), "should install .opencode/commands/aimt.md");
    let cmd_content = std::fs::read_to_string(&cmd).unwrap();
    assert!(
        cmd_content.contains("$ARGUMENTS"),
        "command should dispatch on $ARGUMENTS"
    );
    assert!(
        cmd_content.contains("Read `.agents/aimt/index.md`")
            || cmd_content.to_lowercase().contains("read") && cmd_content.contains("index.md"),
        "command should read index.md"
    );
    assert!(
        cmd_content.contains("read `.agents/aimt/mapping.md`")
            || cmd_content.contains(".agents/aimt/mapping.md"),
        "command should contain mapping.md"
    );
    assert!(
        cmd_content.contains("read `.agents/aimt/update.md`")
            || cmd_content.contains(".agents/aimt/update.md"),
        "command should contain update.md"
    );
    assert!(
        cmd_content.contains("unsupported") || cmd_content.contains("only supports"),
        "command should report unsupported for anything else"
    );
    assert!(
        !cmd_content.contains("/aimt-mapping") && !cmd_content.contains("/aimt-update"),
        "command should be single /aimt"
    );

    // Second install idempotent
    let report2 = installer
        .install(proj)
        .expect("second install should succeed");
    assert!(
        report2.skipped.len() >= 5,
        "second install should skip at least 5, got {:?}",
        report2.skipped
    );
}

#[test]
fn plan_does_not_invent_fields_or_hardcode_unsupported_rules() {
    let modules = aimt::plugins::installer::aimt::MODULES;
    for (_, content) in modules {
        assert!(
            !content.contains("22 fields"),
            "no prompt should invent 22 fields"
        );
        assert!(
            !content.contains("Store::open_mut"),
            "no prompt should hardcode Store::open_mut"
        );
        assert!(
            !content.contains("aimt init"),
            "no prompt should hardcode aimt init unless supported"
        );
        assert!(
            !content.contains("^[a-z]"),
            "no prompt should hardcode ID regex"
        );
    }
    let core = aimt::plugins::installer::aimt::CORE_MD;
    assert!(
        !core.contains("class|function|method|variable"),
        "core should not hardcode frame type list"
    );
}
