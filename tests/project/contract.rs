use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_project_test_{}_{}_{}",
        std::process::id(),
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest_path() -> String {
    format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"))
}

fn run_status(dir: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--manifest-path",
        &manifest_path(),
        "--bin",
        "aimt",
        "--",
        "status",
    ]);
    cmd.args(extra);
    cmd.current_dir(dir);
    cmd.output().unwrap()
}

fn run_tool(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--manifest-path",
        &manifest_path(),
        "--bin",
        "aimt",
        "--",
    ]);
    cmd.args(args);
    cmd.current_dir(dir);
    cmd.output().unwrap()
}

fn run_init(dir: &Path, arg: &str) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--manifest-path",
            &manifest_path(),
            "--bin",
            "aimt",
            "--",
            "init",
            arg,
        ])
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn no_project_reports_no_aimt() {
    let dir = temp_dir();
    let out = run_status(&dir, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{} {}", stdout, stderr);
    // For status with 0 files, we show global status, not error. But for tool that requires project, it should error.
    // Test tool with 0 files should error.
    let out2 = run_tool(&dir, &["search", "--query", "test"]);
    let s2 = format!(
        "{} {}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    assert!(
        s2.contains("No AIMT project found"),
        "should report no project, got: {}",
        s2
    );
    assert!(s2.contains("aimt init"), "should suggest init, got: {}", s2);
    assert!(
        !s2.contains("aimt-test-project.aimt"),
        "must never use fixture"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = combined; // keep for status global case
}

#[test]
fn one_project_auto_uses() {
    let dir = temp_dir();
    // Copy fixture as single project
    let src = if Path::new("aimt-test-project.aimt").exists() {
        PathBuf::from("aimt-test-project.aimt")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    std::fs::copy(&src, dir.join("demo.aimt")).unwrap();
    let out = run_tool(&dir, &["search", "--query", "auth"]);
    assert!(
        out.status.success(),
        "search with one project should auto-use, stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("auth") || stdout.contains("count"),
        "should return search results"
    );
    assert!(
        !stdout.contains("aimt-test-project.aimt"),
        "must not use fixture name"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn explicit_project_wins_without_asking() {
    let dir = temp_dir();
    let src = if Path::new("aimt-test-project.aimt").exists() {
        PathBuf::from("aimt-test-project.aimt")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    std::fs::copy(&src, dir.join("a.aimt")).unwrap();
    std::fs::copy(&src, dir.join("b.aimt")).unwrap();
    // Explicit b.aimt should be used without prompting (positional)
    let out = run_tool(&dir, &["search", "b.aimt", "--query", "auth"]);
    assert!(
        out.status.success(),
        "explicit should win, stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_explicit_path_error() {
    let dir = temp_dir();
    let src = if Path::new("aimt-test-project.aimt").exists() {
        PathBuf::from("aimt-test-project.aimt")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    std::fs::copy(&src, dir.join("a.aimt")).unwrap();
    let out = run_tool(&dir, &["search", "missing.aimt", "--query", "x"]);
    assert!(!out.status.success());
    let s = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        s.contains("not found") || s.contains("AIMT project not found"),
        "should report not found, got: {}",
        s
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_name_normalization() {
    // Test via CLI init (also covers normalize logic)

    let dir = temp_dir();
    let out = run_init(&dir, "demo");
    assert!(out.status.success());
    assert!(dir.join("demo.aimt").exists());
    assert!(!dir.join("demo").exists());
    assert!(!dir.join("demo.aimt.aimt").exists());
    let _ = std::fs::remove_dir_all(&dir);

    let dir2 = temp_dir();
    let out2 = run_init(&dir2, "demo.aimt");
    assert!(out2.status.success());
    assert!(dir2.join("demo.aimt").exists());
    let _ = std::fs::remove_dir_all(&dir2);

    let dir3 = temp_dir();
    let out3 = run_init(&dir3, "demo.aimt.aimt");
    assert!(out3.status.success());
    assert!(dir3.join("demo.aimt").exists());
    assert!(!dir3.join("demo.aimt.aimt").exists());
    let _ = std::fs::remove_dir_all(&dir3);
}

#[test]
fn non_interactive_multiple_projects_error() {
    let dir = temp_dir();
    let src = if Path::new("aimt-test-project.aimt").exists() {
        PathBuf::from("aimt-test-project.aimt")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    std::fs::copy(&src, dir.join("a.aimt")).unwrap();
    std::fs::copy(&src, dir.join("b.aimt")).unwrap();
    // Run with stdin not a tty (cargo test's stdin is not a tty) -> should error, not prompt
    let out = run_tool(&dir, &["search", "--query", "x"]);
    assert!(!out.status.success());
    let s = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        s.contains("Multiple AIMT projects found") && s.contains("project path"),
        "should require explicit project, got: {}",
        s
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_fixture_never_used_as_default() {
    let dir = temp_dir();
    // Empty dir, even though repo root has aimt-test-project.aimt, discovery in empty dir should not use it
    let out = run_tool(&dir, &["search", "--query", "x"]);
    let s = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !s.contains("aimt-test-project.aimt"),
        "must never use fixture as default"
    );
    assert!(
        s.contains("No AIMT project found"),
        "should report no project"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
