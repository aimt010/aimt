use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(prefix: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let p = base.join(format!(
        "aimt_{}_{}",
        prefix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn temp_global(prefix: &str) -> PathBuf {
    let base = std::env::temp_dir();
    base.join(format!(
        "aimt_global_{}_{}.json",
        prefix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn run_with_global(args: &[&str], global_path: &Path) -> std::process::Output {
    let g = global_path.to_str().unwrap().to_string();
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", "aimt", "--"]);
    cmd.args(args);
    cmd.env("AIMT_INSTALL_MANIFEST", &g);
    cmd.output().unwrap()
}

fn write_global(path: &Path, version: &str) {
    let content = format!(
        "{{\"version\":\"{}\",\"install_path\":\"{}\",\"installed_at\":\"123\"}}\n",
        version,
        path.display()
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn install_opencode(project_dir: &Path, global_path: &Path) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--bin",
        "aimt",
        "--",
        "install",
        "opencode",
        "--path",
        project_dir.to_str().unwrap(),
    ]);
    cmd.env("AIMT_INSTALL_MANIFEST", global_path.to_str().unwrap());
    cmd.output().unwrap()
}

// ---------------------------------------------------------------------------
// Upgrade tests
// ---------------------------------------------------------------------------

#[test]
fn upgrade_no_installation_fails() {
    let global = temp_global("upgrade_no_inst");
    assert!(!global.exists());
    let proj = temp_dir("upgrade_no_inst_proj");
    let out = run_with_global(&["upgrade", "--path", proj.to_str().unwrap()], &global);
    assert!(!out.status.success(), "should fail when not installed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("not installed") || combined.contains("upgrade failed"),
        "should mention not installed: {}",
        combined
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_already_latest_reports_no_changes() {
    let global = temp_global("upgrade_latest");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_latest_proj");
    // Install to create project manifest
    let out_install = install_opencode(&proj, &global);
    assert!(
        out_install.status.success(),
        "install should succeed: {:?}",
        String::from_utf8_lossy(&out_install.stderr)
    );
    // Try upgrade with same version via --target
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.1.0",
        ],
        &global,
    );
    assert!(
        out.status.success(),
        "already latest should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("already") && stdout.contains("latest"),
        "should report already latest: {}",
        stdout
    );
    // Verify global unchanged
    let content = std::fs::read_to_string(&global).unwrap();
    assert!(content.contains("0.1.0"), "global should remain 0.1.0");
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_successful_upgrades_engine() {
    let global = temp_global("upgrade_success");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_success_proj");
    let out_install = install_opencode(&proj, &global);
    assert!(out_install.status.success());
    // Add .aimt and source to verify preservation
    std::fs::write(proj.join("demo.aimt"), "original knowledge").unwrap();
    std::fs::write(proj.join("keep.txt"), "source keep").unwrap();
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert!(
        out.status.success(),
        "upgrade should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Successfully upgraded") || stdout.contains("0.1.0 → 0.2.0"),
        "should report success: {}",
        stdout
    );
    assert!(
        stdout.contains("0.1.0") && stdout.contains("0.2.0"),
        "should show versions"
    );
    let global_content = std::fs::read_to_string(&global).unwrap();
    assert!(global_content.contains("0.2.0"), "global should be 0.2.0");
    assert_eq!(
        std::fs::read_to_string(proj.join("demo.aimt")).unwrap(),
        "original knowledge"
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("keep.txt")).unwrap(),
        "source keep"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_incompatible_target_fails_and_preserves() {
    let global = temp_global("upgrade_incompat");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_incompat_proj");
    let out_install = install_opencode(&proj, &global);
    assert!(out_install.status.success());
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "1.0.0",
        ],
        &global,
    );
    assert!(!out.status.success(), "incompatible should fail");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("incompatible") || combined.contains("major"),
        "should mention incompatibility: {}",
        combined
    );
    let global_content = std::fs::read_to_string(&global).unwrap();
    assert!(
        global_content.contains("0.1.0"),
        "should remain 0.1.0 after failed upgrade"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_failed_upgrade_preserves_installation() {
    let global = temp_global("upgrade_failed");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_failed_proj");
    let out_install = install_opencode(&proj, &global);
    assert!(out_install.status.success());
    // Set verify fail env
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--bin",
        "aimt",
        "--",
        "upgrade",
        "--path",
        proj.to_str().unwrap(),
        "--target",
        "0.2.1",
    ]);
    cmd.env("AIMT_INSTALL_MANIFEST", global.to_str().unwrap());
    cmd.env("AIMT_UPGRADE_VERIFY_FAIL", "1");
    let out = cmd.output().unwrap();
    assert!(!out.status.success(), "should fail on verify");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("upgrade failed") || combined.contains("verification"),
        "should report failure: {}",
        combined
    );
    let global_content = std::fs::read_to_string(&global).unwrap();
    assert!(
        global_content.contains("0.1.0"),
        "should rollback to 0.1.0, got {}",
        global_content
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_version_migration_bumps_manifest() {
    let global = temp_global("upgrade_migrate");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_migrate_proj");
    install_opencode(&proj, &global);
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert!(out.status.success());
    let proj_manifest = std::fs::read_to_string(proj.join(".agents/.aimt-manifest.json")).unwrap();
    assert!(
        proj_manifest.contains("0.2.0"),
        "project manifest should migrate to 0.2.0: {}",
        proj_manifest
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_aimt_preserved() {
    let global = temp_global("upgrade_aimt_pres");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_aimt_pres_proj");
    install_opencode(&proj, &global);
    std::fs::write(proj.join("my.aimt"), "knowledge v0").unwrap();
    run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("my.aimt")).unwrap(),
        "knowledge v0"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_source_preserved() {
    let global = temp_global("upgrade_source_pres");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_source_pres_proj");
    install_opencode(&proj, &global);
    std::fs::create_dir_all(proj.join("src")).unwrap();
    std::fs::write(proj.join("src/main.rs"), "fn main() {}").unwrap();
    run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("src/main.rs")).unwrap(),
        "fn main() {}"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_aimt_owned_files_upgraded() {
    let global = temp_global("upgrade_owned");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_owned_proj");
    install_opencode(&proj, &global);
    let index_before = std::fs::read_to_string(proj.join(".agents/aimt/index.md")).unwrap();
    assert!(!index_before.is_empty());
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should report upgraded count >0
    assert!(
        stdout.contains("upgraded") || stdout.contains("Updating"),
        "should mention upgraded: {}",
        stdout
    );
    // Files should still exist
    assert!(proj.join(".agents/aimt/index.md").exists());
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_user_modified_files_protected() {
    let global = temp_global("upgrade_user_mod");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_user_mod_proj");
    install_opencode(&proj, &global);
    let core_path = proj.join(".agents/aimt/core.md");
    std::fs::write(&core_path, "CUSTOM USER CONTENT").unwrap();
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert!(out.status.success());
    assert_eq!(
        std::fs::read_to_string(&core_path).unwrap(),
        "CUSTOM USER CONTENT"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("preserved"),
        "should report preserved: {}",
        stdout
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_verification_failure_reports_and_rollback() {
    let global = temp_global("upgrade_verify_fail");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_verify_fail_proj");
    install_opencode(&proj, &global);
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--bin",
        "aimt",
        "--",
        "upgrade",
        "--path",
        proj.to_str().unwrap(),
        "--target",
        "0.2.0",
    ]);
    cmd.env("AIMT_INSTALL_MANIFEST", global.to_str().unwrap());
    cmd.env("AIMT_UPGRADE_VERIFY_FAIL", "1");
    let out = cmd.output().unwrap();
    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("upgrade failed") || combined.contains("verification"),
        "should fail verification: {}",
        combined
    );
    assert!(
        combined.contains("remains usable"),
        "should say remains usable"
    );
    let global_content = std::fs::read_to_string(&global).unwrap();
    assert!(
        global_content.contains("0.1.0"),
        "rollback should preserve original version"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn upgrade_corrupt_manifest_handled_safely_preserves_files() {
    let global = temp_global("upgrade_corrupt");
    write_global(&global, "0.1.0");
    let proj = temp_dir("upgrade_corrupt_proj");
    install_opencode(&proj, &global);
    // Corrupt project manifest (legacy path also considered)
    std::fs::write(proj.join(".agents/.aimt-manifest.json"), "corrupted {").unwrap();
    std::fs::write(
        proj.join(".agents/aimt/index.md"),
        "user content that should be preserved",
    )
    .unwrap();
    let out = run_with_global(
        &[
            "upgrade",
            "--path",
            proj.to_str().unwrap(),
            "--target",
            "0.2.0",
        ],
        &global,
    );
    assert!(
        out.status.success(),
        "upgrade with corrupt manifest should still succeed but preserve: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(proj.join(".agents/aimt/index.md")).unwrap(),
        "user content that should be preserved"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}
