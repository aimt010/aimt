use std::path::{Path, PathBuf};

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

fn run_install_copilot(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "copilot",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_copilot_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_copilot(&dir);
    assert!(
        out.status.success(),
        "install copilot should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_creates_instructions() {
    let dir = tempfile_dir();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    let instr = dir.join(".github").join("copilot-instructions.md");
    assert!(
        instr.is_file(),
        "copilot instructions should exist at {}",
        instr.display()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_instructions_have_frontmatter_or_marker() {
    let dir = tempfile_dir();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".github/copilot-instructions.md")).unwrap();
    assert!(content.contains("## aimt"), "should contain marker ## aimt");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".github/copilot-instructions.md")).unwrap();
    // Minimal host guide — conceptual relationship only (minimal 5-file architecture)
    assert!(content.contains("## aimt"), "should contain marker ## aimt");
    assert!(
        content.contains(".agents/aimt/index.md"),
        "host file should reference .agents/aimt/index.md"
    );
    assert!(content.contains(".aimt"), "should mention .aimt");
    assert!(
        content.contains("primary knowledge map")
            || content.contains("knowledge map when available"),
        "should state primary knowledge map"
    );
    assert!(
        content.contains("evidence"),
        "should state source is evidence"
    );
    assert!(
        content.contains("missing")
            || content.contains("stale")
            || content.contains("insufficient"),
        "should mention missing/stale handling"
    );
    assert!(
        content.contains("Do not create")
            || content.contains("Do NOT")
            || content.contains("no separate"),
        "should forbid separate project dir"
    );
    assert!(
        !content.contains("before answering") && !content.contains("before reading source"),
        "should not be mandatory gate"
    );
    assert!(
        content.lines().count() < 20,
        "host guide should be minimal, got {} lines",
        content.lines().count()
    );
    // Verify minimal 5-file set
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
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_preserves_existing_instructions() {
    let dir = tempfile_dir();
    let instr_path = dir.join(".github").join("copilot-instructions.md");
    std::fs::create_dir_all(instr_path.parent().unwrap()).unwrap();
    std::fs::write(
        &instr_path,
        "# My Copilot Instructions\n\nExisting content.\n",
    )
    .unwrap();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&instr_path).unwrap();
    assert!(
        content.contains("My Copilot Instructions"),
        "should preserve existing header"
    );
    assert!(
        content.contains("Existing content"),
        "should preserve existing body"
    );
    assert!(content.contains("## aimt"), "should add aimt section");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_preserves_existing_github_files() {
    let dir = tempfile_dir();
    let other_file = dir.join(".github").join("workflows").join("ci.yml");
    std::fs::create_dir_all(other_file.parent().unwrap()).unwrap();
    std::fs::write(&other_file, "name: CI\n").unwrap();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_file).unwrap(),
        "name: CI\n",
        "should preserve other .github files"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_copilot(&dir);
    assert!(out1.status.success());
    let content1 = std::fs::read_to_string(dir.join(".github/copilot-instructions.md")).unwrap();
    let out2 = run_install_copilot(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let content2 = std::fs::read_to_string(dir.join(".github/copilot-instructions.md")).unwrap();
    assert_eq!(content1, content2, "content should be identical");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.to_lowercase().contains("aimt installed for")
            || stdout2.contains("skipped")
            || stdout2.contains("already"),
        "second run should report installed or skipped: {}",
        stdout2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_no_duplicate_content() {
    let dir = tempfile_dir();
    let out1 = run_install_copilot(&dir);
    assert!(out1.status.success());
    let out2 = run_install_copilot(&dir);
    assert!(out2.status.success());
    let content = std::fs::read_to_string(dir.join(".github/copilot-instructions.md")).unwrap();
    assert!(content.contains("aimt"));
    assert_eq!(
        content.matches("## aimt").count(),
        1,
        "should not duplicate marker"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_copilot_nonexistent_{}",
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
            "copilot",
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
fn install_copilot_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_copilot(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_copilot_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_copilot = run_install_copilot(&dir);
    assert!(out_copilot.status.success());
    assert!(dir.join(".github/copilot-instructions.md").is_file());
    // Should not create opencode or claude specific files
    // But may create .github directory, which is shared with other tools? For copilot, .github is specific.
    // Install opencode afterwards should preserve copilot file
    let out_opencode = {
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
    };
    assert!(out_opencode.status.success());
    assert!(dir.join(".github/copilot-instructions.md").is_file());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
