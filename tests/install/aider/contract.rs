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

fn run_install_aider(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "aider",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_aider_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_aider(&dir);
    assert!(
        out.status.success(),
        "install aider should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_creates_conf_and_guide() {
    let dir = tempfile_dir();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let conf = dir.join(".aider.conf.yml");
    assert!(
        conf.is_file(),
        ".aider.conf.yml should exist at {}",
        conf.display()
    );
    let guide = dir.join(".aider").join("aimt.md");
    assert!(guide.is_file(), "guide should exist at {}", guide.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_guide_contains_instructions() {
    let dir = tempfile_dir();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".aider/aimt.md")).unwrap();
    // Minimal host guide — conceptual relationship only, not verbose gate (minimal 5-file architecture)
    assert!(content.contains("## aimt"), "should contain marker ## aimt");
    assert!(
        content.contains(".agents/aimt/index.md"),
        "should point to .agents/aimt/index.md"
    );
    assert!(content.contains(".aimt"), "should mention .aimt file");
    assert!(
        content.contains("primary knowledge map")
            || content.contains("knowledge map when available"),
        "should state AIMT is primary knowledge map when available"
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
    // Minimal 5-file architecture: .agents/aimt should be exactly 5 files alphabetical (Task 6)
    let aimt_dir = dir.join(".agents/aimt");
    if aimt_dir.is_dir() {
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
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_conf_contains_read_entry() {
    let dir = tempfile_dir();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".aider.conf.yml")).unwrap();
    assert!(content.contains("read:"), "should contain read key");
    assert!(
        content.contains(".aider/aimt.md"),
        "should contain AIMT guide path"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_preserves_existing_config() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".aider.conf.yml");
    std::fs::write(&cfg_path, "model: gpt-4\nmap-tokens: 4096\n").unwrap();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(content.contains("model: gpt-4"), "should preserve model");
    assert!(
        content.contains("map-tokens: 4096"),
        "should preserve map-tokens"
    );
    assert!(content.contains(".aider/aimt.md"), "should add aimt entry");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_preserves_existing_read_entries() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".aider.conf.yml");
    std::fs::write(&cfg_path, "read: [CONVENTIONS.md, docs/architecture.md]\n").unwrap();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(
        content.contains("CONVENTIONS.md"),
        "should preserve existing read entry"
    );
    assert!(
        content.contains("docs/architecture.md"),
        "should preserve second entry"
    );
    assert!(content.contains(".aider/aimt.md"), "should add aimt entry");
    // Check that existing entries are still there and not duplicated
    assert_eq!(content.matches("CONVENTIONS.md").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_preserves_bulleted_read_entries() {
    let dir = tempfile_dir();
    let cfg_path = dir.join(".aider.conf.yml");
    std::fs::write(&cfg_path, "read:\n  - CONVENTIONS.md\n  - other.md\n").unwrap();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(content.contains("CONVENTIONS.md"));
    assert!(content.contains("other.md"));
    assert!(content.contains(".aider/aimt.md"));
    assert_eq!(
        content.matches(".aider/aimt.md").count(),
        1,
        "should add exactly once"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_adds_aimt_exactly_once() {
    let dir = tempfile_dir();
    let out1 = run_install_aider(&dir);
    assert!(out1.status.success());
    let out2 = run_install_aider(&dir);
    assert!(out2.status.success());
    let content = std::fs::read_to_string(dir.join(".aider.conf.yml")).unwrap();
    assert_eq!(
        content.matches(".aider/aimt.md").count(),
        1,
        "should contain AIMT entry exactly once"
    );
    let guide_content = std::fs::read_to_string(dir.join(".aider/aimt.md")).unwrap();
    assert_eq!(guide_content.matches("## aimt").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_aider(&dir);
    assert!(out1.status.success());
    let conf1 = std::fs::read_to_string(dir.join(".aider.conf.yml")).unwrap();
    let guide1 = std::fs::read_to_string(dir.join(".aider/aimt.md")).unwrap();
    let out2 = run_install_aider(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let conf2 = std::fs::read_to_string(dir.join(".aider.conf.yml")).unwrap();
    let guide2 = std::fs::read_to_string(dir.join(".aider/aimt.md")).unwrap();
    assert_eq!(conf1, conf2, "config should be identical");
    assert_eq!(guide1, guide2, "guide should be identical");
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
fn install_aider_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_aider_nonexistent_{}",
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
            "aider",
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
fn install_aider_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_aider(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_aider_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_aider = run_install_aider(&dir);
    assert!(out_aider.status.success());
    assert!(dir.join(".aider/aimt.md").is_file());
    assert!(dir.join(".aider.conf.yml").is_file());
    // Should not create other tools' files by default
    // Install opencode afterwards should preserve aider files
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
    assert!(dir.join(".aider/aimt.md").is_file());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
