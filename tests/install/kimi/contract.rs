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

fn run_install_kimi(dir: &Path) -> std::process::Output {
    std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "kimi",
            "--path",
            dir.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
fn install_kimi_succeeds() {
    let dir = tempfile_dir();
    let out = run_install_kimi(&dir);
    assert!(
        out.status.success(),
        "install kimi should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_creates_skill() {
    let dir = tempfile_dir();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    let skill = dir
        .join(".kimi")
        .join("skills")
        .join("aimt")
        .join("SKILL.md");
    assert!(skill.is_file(), "skill should exist at {}", skill.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_skill_has_frontmatter() {
    let dir = tempfile_dir();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    let content = std::fs::read_to_string(dir.join(".kimi/skills/aimt/SKILL.md")).unwrap();
    assert!(content.starts_with("---"), "should start with frontmatter");
    assert!(content.contains("name: aimt"), "should contain name: aimt");
    assert!(
        content.contains("description:"),
        "should contain description"
    );
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    assert!(parts.len() >= 3, "should have closing --- for frontmatter");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_instructions_present() {
    let dir = tempfile_dir();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    let skill = std::fs::read_to_string(dir.join(".kimi/skills/aimt/SKILL.md")).unwrap();
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
        skill.to_lowercase().contains("hosted") && skill.to_lowercase().contains("install"),
        "should explain Hosted vs Install"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_preserves_existing_skills() {
    let dir = tempfile_dir();
    let other_skill = dir
        .join(".kimi")
        .join("skills")
        .join("other")
        .join("SKILL.md");
    std::fs::create_dir_all(other_skill.parent().unwrap()).unwrap();
    std::fs::write(&other_skill, "# other skill\n").unwrap();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_skill).unwrap(),
        "# other skill\n",
        "should preserve other skill"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_preserves_existing_agents_skills() {
    let dir = tempfile_dir();
    let other_skill = dir
        .join(".agents")
        .join("skills")
        .join("other")
        .join("SKILL.md");
    std::fs::create_dir_all(other_skill.parent().unwrap()).unwrap();
    std::fs::write(&other_skill, "# other agents skill\n").unwrap();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&other_skill).unwrap(),
        "# other agents skill\n",
        "should preserve .agents skills"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_idempotent() {
    let dir = tempfile_dir();
    let out1 = run_install_kimi(&dir);
    assert!(out1.status.success());
    let skill1 = std::fs::read_to_string(dir.join(".kimi/skills/aimt/SKILL.md")).unwrap();
    let out2 = run_install_kimi(&dir);
    assert!(out2.status.success(), "second install should succeed");
    let skill2 = std::fs::read_to_string(dir.join(".kimi/skills/aimt/SKILL.md")).unwrap();
    assert_eq!(skill1, skill2, "skill should be identical");
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
fn install_kimi_no_duplicate_content() {
    let dir = tempfile_dir();
    let out1 = run_install_kimi(&dir);
    assert!(out1.status.success());
    let out2 = run_install_kimi(&dir);
    assert!(out2.status.success());
    let skill = std::fs::read_to_string(dir.join(".kimi/skills/aimt/SKILL.md")).unwrap();
    assert!(skill.contains("aimt"));
    assert_eq!(
        skill.matches("name: aimt").count(),
        1,
        "should not duplicate frontmatter"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_invalid_project() {
    let dir = std::env::temp_dir().join(format!(
        "aimt_kimi_nonexistent_{}",
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
            "kimi",
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
fn install_kimi_preserves_unrelated_files() {
    let dir = tempfile_dir();
    let readme = dir.join("README.md");
    std::fs::write(&readme, "# Keep me\n").unwrap();
    let out = run_install_kimi(&dir);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&readme).unwrap(), "# Keep me\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_kimi_does_not_affect_other_tools() {
    let dir = tempfile_dir();
    let out_kimi = run_install_kimi(&dir);
    assert!(out_kimi.status.success());
    assert!(dir.join(".kimi/skills/aimt/SKILL.md").is_file());
    // Should not create opencode or claude specific files
    assert!(
        !dir.join("CLAUDE.md").exists()
            || std::fs::read_to_string(dir.join("CLAUDE.md"))
                .unwrap_or_default()
                .is_empty()
            || !dir.join(".claude").exists(),
        "kimi should not create CLAUDE.md by default"
    );
    // Install opencode afterwards should preserve kimi skill
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
    assert!(dir.join(".kimi/skills/aimt/SKILL.md").is_file());
    assert!(dir.join(".opencode/plugins/aimt.js").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
