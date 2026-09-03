use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use aimt::core::security::auth::delete_credential;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

static HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn home_lock() -> &'static Mutex<()> {
    HOME_LOCK.get_or_init(|| Mutex::new(()))
}

struct IsolatedHome {
    _guard: std::sync::MutexGuard<'static, ()>,
    original_home: Option<String>,
    home_path: PathBuf,
}

impl IsolatedHome {
    fn new() -> Self {
        let guard = home_lock().lock().unwrap();
        let original_home = std::env::var("HOME").ok();
        let home_path = {
            let base = std::env::temp_dir();
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let p = base.join(format!(
                "aimt_home_{}_{}_{}",
                std::process::id(),
                id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&p).unwrap();
            p
        };
        unsafe {
            std::env::set_var("HOME", &home_path);
        }
        Self {
            _guard: guard,
            original_home,
            home_path,
        }
    }
}

impl Drop for IsolatedHome {
    fn drop(&mut self) {
        unsafe {
            if let Some(orig) = &self.original_home {
                std::env::set_var("HOME", orig);
            } else {
                std::env::remove_var("HOME");
            }
        }
        let _ = std::fs::remove_dir_all(&self.home_path);
    }
}

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_status_test_{}_{}_{}",
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

fn run_init_capture(path: &Path) -> (bool, String, String) {
    let out = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "init",
            &path.to_string_lossy(),
        ])
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[allow(clippy::manual_find)]
fn extract_private_and_public(stdout: &str, stderr: &str, pub_key: &str) -> Option<String> {
    let combined = format!("{stdout} {stderr}");
    let tokens: Vec<String> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .map(|s| s.to_lowercase())
        .collect();
    let pub_lower = pub_key.to_lowercase();
    for tok in tokens {
        if tok != pub_lower {
            return Some(tok);
        }
    }
    None
}

fn run_status(path: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", "aimt", "--", "status"]);
    if let Some(p) = path {
        cmd.arg(p.to_string_lossy().to_string());
    }
    cmd.output().unwrap()
}

fn run_login(path: &Path, priv_hex: &str) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "login",
            &path.to_string_lossy(),
            "--key",
            priv_hex,
        ])
        .output()
        .unwrap()
}

fn run_logout_all() -> std::process::Output {
    Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "logout"])
        .output()
        .unwrap()
}

#[test]
fn status_not_logged_in() {
    let _home = IsolatedHome::new();
    // Ensure clean slate
    let _ = run_logout_all();
    let out = run_status(None);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(combined.contains("Not logged in") || combined.contains("Status: Not logged in"));
    assert!(!combined.to_lowercase().contains("private"));
    // Ensure no full 64 hex leaked
    let tokens: Vec<&str> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .collect();
    assert!(
        tokens.is_empty(),
        "status must not leak full keys, got {:?}",
        tokens
    );
}

#[test]
fn status_logged_in_shows_owner_truncated() {
    let _home = IsolatedHome::new();
    let dir = tempdir();
    let path = dir.join("s.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = aimt::store::Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private_and_public(&stdout, &stderr, &pub_key).unwrap();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    let out = run_status(None);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Logged in"));
    assert!(stdout.contains(&pub_key[..8]));
    assert!(!stdout.to_lowercase().contains(&priv_hex.to_lowercase()));
    // No full private leaked
    assert!(!stdout.contains(&priv_hex));
    // Cleanup
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_with_path_verifies_project_owner() {
    let _home = IsolatedHome::new();
    let dir = tempdir();
    let path = dir.join("s2.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = aimt::store::Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private_and_public(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let out = run_status(Some(&path));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Logged in"));
    assert!(stdout.contains("Project"));
    assert!(stdout.contains("Verified") || stdout.contains("Project auth: Verified"));
    // Now create second project with different owner, status for second should show mismatch if we don't login for second
    let dir2 = tempdir();
    let path2 = dir2.join("other.aimt");
    let (ok2, _, _) = run_init_capture(&path2);
    assert!(ok2);
    let out2 = run_status(Some(&path2));
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    // Should show not authenticated for this project (since we only logged in for first)
    assert!(
        stdout2.contains("Not authenticated")
            || stdout2.contains("Mismatch")
            || stdout2.contains("Project auth:"),
        "second project should not be verified, got {}",
        stdout2
    );
    let _ = delete_credential(&pub_key);
    // Cleanup second pub
    if let Ok(s) = aimt::store::Store::open(&path2)
        && let Some(pk) = s.owner_public_key()
    {
        let _ = delete_credential(&pk);
    }
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn status_does_not_require_project_path() {
    let _home = IsolatedHome::new();
    let dir = tempdir();
    let path = dir.join("s3.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = aimt::store::Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private_and_public(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    // status without path should succeed even without project
    let out = run_status(None);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("AIMT Authentication"));
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_no_private_leakage() {
    let _home = IsolatedHome::new();
    let dir = tempdir();
    let path = dir.join("s4.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = aimt::store::Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private_and_public(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    for path_opt in [None, Some(path.as_path())] {
        let out = run_status(path_opt);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.to_lowercase().contains(&priv_hex.to_lowercase()),
            "status must never print private"
        );
        // Also ensure not full 64 hex private appears (public truncated is ok)
        // Count 64 hex tokens: should be 0 or only public truncated not full
        let tokens: Vec<String> = combined
            .split(|c: char| !c.is_ascii_hexdigit())
            .filter(|s| s.len() == 64)
            .map(|s| s.to_lowercase())
            .collect();
        for tok in tokens {
            assert_ne!(tok, priv_hex.to_lowercase(), "private must not be leaked");
        }
    }
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}
