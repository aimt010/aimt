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
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", "aimt", "--"]);
    cmd.args(args);
    cmd.env("AIMT_INSTALL_MANIFEST", global_path.to_str().unwrap());
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
// Uninstall tests
// ---------------------------------------------------------------------------

#[test]
fn uninstall_successful_removes_installation() {
    let global = temp_global("uninstall_success");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_success_proj");
    let out_install = install_opencode(&proj, &global);
    assert!(out_install.status.success());
    assert!(proj.join(".agents/aimt/index.md").exists());
    let out = run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(
        out.status.success(),
        "uninstall should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("successfully uninstalled") || stdout.contains("uninstalled"),
        "should report success: {}",
        stdout
    );
    assert!(!global.exists(), "global manifest should be removed");
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_already_uninstalled_is_noop() {
    let global = temp_global("uninstall_already");
    assert!(!global.exists());
    let proj = temp_dir("uninstall_already_proj");
    let out = run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(
        out.status.success(),
        "already uninstalled should be success/noop"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("not installed") || stdout.contains("Nothing to uninstall"),
        "should report not installed: {}",
        stdout
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_aimt_preserved() {
    let global = temp_global("uninstall_aimt_pres");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_aimt_pres_proj");
    install_opencode(&proj, &global);
    std::fs::write(proj.join("demo.aimt"), "project knowledge").unwrap();
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("demo.aimt")).unwrap(),
        "project knowledge"
    );
    assert!(!global.exists());
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_source_preserved() {
    let global = temp_global("uninstall_source_pres");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_source_pres_proj");
    install_opencode(&proj, &global);
    std::fs::create_dir_all(proj.join("src")).unwrap();
    std::fs::write(proj.join("src/main.rs"), "fn main() {}").unwrap();
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
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
fn uninstall_aimt_owned_files_removed() {
    let global = temp_global("uninstall_owned");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_owned_proj");
    install_opencode(&proj, &global);
    assert!(proj.join(".agents/aimt/index.md").exists());
    assert!(proj.join(".agents/aimt/core.md").exists());
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    // Owned files should be removed (since not user-modified)
    assert!(
        !proj.join(".agents/aimt/index.md").exists(),
        "owned index.md should be removed"
    );
    assert!(
        !proj.join(".agents/aimt/core.md").exists(),
        "owned core.md should be removed"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_user_owned_files_preserved() {
    let global = temp_global("uninstall_user_owned");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_user_owned_proj");
    install_opencode(&proj, &global);
    // Create a user-owned file not in manifest
    let user_file = proj.join(".agents/aimt/custom_user.md");
    std::fs::write(&user_file, "user content").unwrap();
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(user_file.exists(), "user-owned file should be preserved");
    assert_eq!(std::fs::read_to_string(&user_file).unwrap(), "user content");
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_user_modified_aimt_files_preserved_when_uncertain() {
    let global = temp_global("uninstall_user_mod");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_user_mod_proj");
    install_opencode(&proj, &global);
    let core_path = proj.join(".agents/aimt/core.md");
    std::fs::write(&core_path, "CUSTOM MODIFIED").unwrap();
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(
        core_path.exists(),
        "user-modified core.md should be preserved"
    );
    assert_eq!(
        std::fs::read_to_string(&core_path).unwrap(),
        "CUSTOM MODIFIED"
    );
    // Global should be removed, but project manifest retained
    assert!(!global.exists());
    assert!(
        proj.join(".agents/.aimt-manifest.json").exists(),
        "manifest should be retained when ownership uncertain"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_missing_manifest_handled_safely_preserves_files() {
    let global = temp_global("uninstall_missing_manifest");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_missing_manifest_proj");
    std::fs::create_dir_all(proj.join(".agents/aimt")).unwrap();
    std::fs::write(
        proj.join(".agents/aimt/some.md"),
        "user file without manifest",
    )
    .unwrap();
    std::fs::write(proj.join("demo.aimt"), "knowledge").unwrap();
    // No project manifest exists (missing)
    assert!(!proj.join(".agents/.aimt-manifest.json").exists());
    let out = run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(out.status.success());
    assert!(
        proj.join(".agents/aimt/some.md").exists(),
        "should preserve files when manifest missing"
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("demo.aimt")).unwrap(),
        "knowledge"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_corrupt_manifest_handled_safely() {
    let global = temp_global("uninstall_corrupt");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_corrupt_proj");
    std::fs::create_dir_all(proj.join(".agents/aimt")).unwrap();
    std::fs::write(proj.join(".agents/.aimt-manifest.json"), "corrupted {").unwrap();
    std::fs::write(proj.join(".agents/aimt/index.md"), "content").unwrap();
    std::fs::write(proj.join("demo.aimt"), "knowledge").unwrap();
    let out = run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert!(
        out.status.success(),
        "should handle corrupt manifest safely: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Should preserve files
    assert!(
        proj.join(".agents/aimt/index.md").exists(),
        "should preserve files on corrupt manifest"
    );
    assert!(
        proj.join(".agents/.aimt-manifest.json").exists(),
        "corrupt manifest should be retained (not deleted without --force)"
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("demo.aimt")).unwrap(),
        "knowledge"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_verification_failure_reports_error() {
    let global = temp_global("uninstall_verify_fail");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_verify_fail_proj");
    install_opencode(&proj, &global);
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--bin",
        "aimt",
        "--",
        "uninstall",
        "--path",
        proj.to_str().unwrap(),
        "--yes",
    ]);
    cmd.env("AIMT_INSTALL_MANIFEST", global.to_str().unwrap());
    cmd.env("AIMT_UNINSTALL_VERIFY_FAIL", "1");
    let out = cmd.output().unwrap();
    assert!(!out.status.success(), "should fail verification");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("verification") || combined.contains("uninstall failed"),
        "should report verification failure: {}",
        combined
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}

#[test]
fn uninstall_never_deletes_aimt_looking_files_outside_agents() {
    let global = temp_global("uninstall_never_delete");
    write_global(&global, "0.2.0");
    let proj = temp_dir("uninstall_never_delete_proj");
    install_opencode(&proj, &global);
    // Create .aimt file and also a file that looks like AIMT but is source
    std::fs::write(proj.join("demo.aimt"), "keep").unwrap();
    std::fs::write(proj.join("aimt_fake.md"), "keep fake").unwrap();
    std::fs::create_dir_all(proj.join("src")).unwrap();
    std::fs::write(proj.join("src/lib.rs"), "source").unwrap();
    run_with_global(
        &["uninstall", "--path", proj.to_str().unwrap(), "--yes"],
        &global,
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("demo.aimt")).unwrap(),
        "keep"
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("aimt_fake.md")).unwrap(),
        "keep fake"
    );
    assert_eq!(
        std::fs::read_to_string(proj.join("src/lib.rs")).unwrap(),
        "source"
    );
    let _ = std::fs::remove_dir_all(&proj);
    let _ = std::fs::remove_file(&global);
}
