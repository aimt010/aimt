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

fn run_install_cursor(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "cursor",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_cursor_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_cursor(&dir);
    assert!(
        out.status.success(),
        "install cursor should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_creates_rule() {
    let dir = tempfile_dir();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    let rule = dir.join(".cursor").join("rules").join("aimt.mdc");
    assert!(rule.is_file(), "rule should exist at {}", rule.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_rule_has_frontmatter() {
    let dir = tempfile_dir();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".cursor/rules/aimt.mdc")).unwrap();
    assert!(content.starts_with("---"), "should start with frontmatter");
    assert!(
        content.contains("description:"),
        "should contain description"
    );
    assert!(
        content.contains("alwaysApply:"),
        "should contain alwaysApply"
    );
    assert!(
        content.contains("alwaysApply: true"),
        "should be alwaysApply: true"
    );
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    assert!(parts.len() >= 3, "should have closing --- for frontmatter");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".cursor/rules/aimt.mdc")).unwrap();
    assert!(content.contains("AIMT"), "should contain AIMT");
    assert!(content.contains(".aimt"), "should mention .aimt");
    assert!(
        content.to_lowercase().contains("when") && content.contains("AIMT"),
        "should mention when to use"
    );
    assert!(
        content.contains("discover") || content.contains("Discover"),
        "should mention discover"
    );
    assert!(
        content.contains("search") && content.contains("read"),
        "should mention search/read"
    );
    assert!(
        content.contains("follow_parent") || content.contains("follow parent"),
        "should mention follow parent"
    );
    assert!(
        content.contains("follow_file") || content.contains("follow file"),
        "should mention follow file"
    );
    assert!(content.contains("relation"), "should mention relation");
    assert!(content.contains("validate"), "should mention validate");
    assert!(
        content.to_lowercase().contains("hosted") && content.to_lowercase().contains("install"),
        "should explain Hosted vs Install"
    );
    // Should not tell to blindly read entire .aimt file
    assert!(
        !content
            .to_lowercase()
            .contains("read the entire .aimt file")
            && !content.to_lowercase().contains("bypass the aimt interface"),
        "should not tell to blindly read entire .aimt or bypass interface"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_preserves_existing_rules() {
    let dir = tempfile_dir();
    let other_rule = dir.join(".cursor").join("rules").join("other.mdc");
    std::fs::create_dir_all(other_rule.parent().unwrap()).unwrap();
    std::fs::write(
        &other_rule,
        "---\ndescription: other rule\nalwaysApply: false\n---\n# Other\n",
    )
    .unwrap();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_rule).unwrap(),
        "---\ndescription: other rule\nalwaysApply: false\n---\n# Other\n",
        "should preserve other rule"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_no_duplicate_rule() {
    let dir = tempfile_dir();
    let out1 = run_install_cursor(&dir);
    assert!(out1.status.success());
    let out2 = run_install_cursor(&dir);
    assert!(out2.status.success());
    let content = std::fs::read_to_string(dir.join(".cursor/rules/aimt.mdc")).unwrap();
    assert_eq!(
        content.matches("description:").count(),
        1,
        "should not duplicate frontmatter"
    );
    assert_eq!(
        content.matches("alwaysApply: true").count(),
        1,
        "should not duplicate alwaysApply"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_cursor(&dir);
    assert!(out1.status.success());
    let content1 = std::fs::read_to_string(dir.join(".cursor/rules/aimt.mdc")).unwrap();
    let out2 = run_install_cursor(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let content2 = std::fs::read_to_string(dir.join(".cursor/rules/aimt.mdc")).unwrap();
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
fn install_cursor_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_cursor_nonexistent_{}",
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
            "cursor",
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
fn install_cursor_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_cursor = run_install_cursor(&dir);
    assert!(out_cursor.status.success());
    assert!(dir.join(".cursor/rules/aimt.mdc").is_file());
    // Should not create other tools' files by default
    assert!(
        !dir.join("CLAUDE.md").exists()
            || std::fs::read_to_string(dir.join("CLAUDE.md"))
                .unwrap_or_default()
                .is_empty(),
        "cursor should not create CLAUDE.md"
    );
    // Install opencode afterwards should preserve cursor rule
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
    assert!(dir.join(".cursor/rules/aimt.mdc").is_file());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_cursor_existing_rule_is_updated_safely() {
    let dir = tempfile_dir();
    // Create an existing aimt rule with old content
    let rule_path = dir.join(".cursor/rules/aimt.mdc");
    std::fs::create_dir_all(rule_path.parent().unwrap()).unwrap();
    std::fs::write(&rule_path, "---\ndescription: old\n---\n# Old AIMT\n").unwrap();
    let out = run_install_cursor(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&rule_path).unwrap();
    // Should be updated to new content, not duplicated
    assert!(content.contains("AIMT"), "should contain AIMT after update");
    assert!(
        content.contains("alwaysApply: true"),
        "should have correct frontmatter"
    );
    assert_eq!(
        content.matches("description:").count(),
        1,
        "should not duplicate"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
