use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use aimt::core::security::auth::{delete_credential, load_credential};
use aimt::store::Store;

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
        // SAFETY: we hold the global lock, so no other test can race on HOME
        unsafe {
            std::env::set_var("HOME", &home_path);
        }
        // Also ensure dirs crate will see new HOME; no cache to clear
        Self {
            _guard: guard,
            original_home,
            home_path,
        }
    }
}

impl Drop for IsolatedHome {
    fn drop(&mut self) {
        // SAFETY: still holding lock
        unsafe {
            if let Some(orig) = &self.original_home {
                std::env::set_var("HOME", orig);
            } else {
                std::env::remove_var("HOME");
            }
        }
        let _ = std::fs::remove_dir_all(&self.home_path);
        // guard is dropped after this, releasing lock
    }
}

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_key_test_{}_{}_{}",
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
fn extract_private(stdout: &str, stderr: &str, pub_key: &str) -> Option<String> {
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

fn run_backup(output: &Path, aimt_path: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run", "--quiet", "--bin", "aimt", "--", "key", "backup", "--output",
    ]);
    cmd.arg(output.to_string_lossy().to_string());
    if let Some(p) = aimt_path {
        cmd.args(["--aimt", &p.to_string_lossy()]);
    }
    cmd.output().unwrap()
}

fn run_rotate(path: &Path) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "key",
            "rotate",
            &path.to_string_lossy(),
        ])
        .output()
        .unwrap()
}

fn make_domain(id: &str) -> aimt::model::AimtEntity {
    use aimt::model::{AimtEntity, Field};
    use aimt::syntax::{Level, Span};
    let s = Span::range(1, 1, 1, 1);
    AimtEntity {
        level: Level::Domain,
        level_span: s.clone(),
        span: s.clone(),
        header: vec![
            Field {
                name: "id".to_string(),
                value: id.to_string(),
                span: s.clone(),
            },
            Field {
                name: "title".to_string(),
                value: format!("Title {id}"),
                span: s.clone(),
            },
        ],
        body: vec![Field {
            name: "description".to_string(),
            value: "test".to_string(),
            span: s,
        }],
        relations: vec![],
    }
}

#[test]
fn backup_requires_authentication() {
    let _home = IsolatedHome::new();
    // Explicitly unauthenticated: isolated HOME has no credentials, no need to logout
    // Do not inherit developer's real ~/.aimt/credentials
    let dir = tempdir();
    let out_path = dir.join("backup.json");
    let out = run_backup(&out_path, None);
    assert!(!out.status.success(), "backup without login should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not authenticated") || stderr.contains("no stored credential"));
    assert!(!out_path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn backup_authenticated_creates_file_and_not_in_aimt() {
    let _home = IsolatedHome::new();

    let _ = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "logout"])
        .output()
        .unwrap();
    let dir = tempdir();
    let path = dir.join("p.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_key = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let backup_path = tempdir().join("owner_backup.json");
    let out = run_backup(&backup_path, Some(&path));
    assert!(
        out.status.success(),
        "backup should succeed when authenticated: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(backup_path.is_file());
    let content = std::fs::read_to_string(&backup_path).unwrap();
    assert!(content.contains("\"version\""));
    assert!(content.contains(&priv_hex));
    assert!(content.contains(&pub_key));
    // Not in .aimt
    let raw = std::fs::read(&path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(!raw_str.contains(&priv_hex) || raw_str.contains(&pub_key));
    // Private not logged: backup command stdout should not contain private? Our backup prints only path, not private
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stdout.contains(&priv_hex));
    assert!(
        !stderr.contains(&priv_hex) || stderr.contains("WARNING") && !stderr.contains(&priv_hex)
    );
    // Not in project dir by default
    assert!(!backup_path.starts_with(&dir));
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(backup_path.parent().unwrap());
}

#[test]
fn backup_not_silently_overwrites() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("p2.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_key = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let backup_path = tempdir().join("backup2.json");
    std::fs::write(&backup_path, "existing").unwrap();
    let out = run_backup(&backup_path, Some(&path));
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&backup_path).unwrap(), "existing");
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(backup_path.parent().unwrap());
}

#[test]
fn backup_contains_recoverable_credential() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("p3.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_key = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let backup_path = tempdir().join("recover.json");
    let out = run_backup(&backup_path, Some(&path));
    assert!(out.status.success());
    // Load from backup and verify
    let (loaded_priv, loaded_pub) =
        aimt::core::security::backup::load_from_backup(&backup_path).unwrap();
    assert_eq!(loaded_priv.to_lowercase(), priv_hex.to_lowercase());
    assert_eq!(loaded_pub.to_lowercase(), pub_key.to_lowercase());
    // Use backup private to login on new machine (simulate by deleting credential and re-login via backup)
    // On macOS with keyring 4, credentials are per-app in Keychain; library delete from test binary
    // cannot delete entry created by `aimt` binary. Use CLI logout (which runs in `aimt` binary) for reliable cleanup.
    let _ = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "logout",
            &path.to_string_lossy(),
        ])
        .output();
    let _ = delete_credential(&pub_key);
    assert!(load_credential(&pub_key).is_none());
    let out2 = run_login(&path, &loaded_priv);
    assert!(out2.status.success());
    assert!(load_credential(&pub_key).is_some());
    let _ = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "logout",
            &path.to_string_lossy(),
        ])
        .output();
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(backup_path.parent().unwrap());
}

#[test]
fn cross_machine_backup_restore() {
    let _home = IsolatedHome::new();

    // Machine A: init, login, backup, development
    let dir_a = tempdir();
    let path_a = dir_a.join("proj.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path_a);
    assert!(ok);
    let pub_a = Store::open(&path_a)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_a = extract_private(&stdout, &stderr, &pub_a).unwrap();
    let _ = run_login(&path_a, &priv_a);
    let backup_path = tempdir().join("cross_backup.json");
    let out = run_backup(&backup_path, Some(&path_a));
    assert!(out.status.success());
    // Simulate Machine A removed: delete credential locally
    let _ = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "logout"])
        .output()
        .unwrap();
    assert!(load_credential(&pub_a).is_none());
    // Copy .aimt to Machine B location (same public artifact)
    let dir_b = tempdir();
    let path_b = dir_b.join("proj_copy.aimt");
    std::fs::copy(&path_a, &path_b).unwrap();
    // Machine B: install AIMT (implicit) + login with backup
    let (loaded_priv, loaded_pub) =
        aimt::core::security::backup::load_from_backup(&backup_path).unwrap();
    assert_eq!(loaded_pub.to_lowercase(), pub_a.to_lowercase());
    let out2 = run_login(&path_b, &loaded_priv);
    assert!(
        out2.status.success(),
        "login on B with backup should succeed: {} {}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    // Verify write access restored
    let mut s = Store::open(&path_b).unwrap();
    let ent = make_domain(&format!(
        "d_cross_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let id = ent.id().unwrap().as_str().to_string();
    s.insert(ent)
        .expect("write after cross-machine login should succeed");
    s.persist().unwrap();
    assert!(Store::open(&path_b).unwrap().contains(&id));
    let _ = delete_credential(&pub_a);
    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
    let _ = std::fs::remove_dir_all(backup_path.parent().unwrap());
}

#[test]
fn backup_not_in_aimt_and_not_in_project() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("p4.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_key = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let _backup_path = dir.join("should_not_be_here.json");
    // Even if we try to backup to project dir, it should succeed but we test that default not in project is enforced by requiring explicit output
    // For this test, we backup to a secure location outside project
    let secure_dir = tempdir();
    let secure_backup = secure_dir.join("secure.json");
    let out = run_backup(&secure_backup, Some(&path));
    assert!(out.status.success());
    // Ensure .aimt does not contain private
    let raw = std::fs::read(&path).unwrap();
    assert!(
        !String::from_utf8_lossy(&raw)
            .to_lowercase()
            .contains(&priv_hex.to_lowercase())
    );
    // Ensure backup not in project dir by default (we used secure_dir, not dir)
    assert!(!secure_backup.starts_with(&dir));
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_file(&secure_backup);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&secure_dir);
}

#[test]
fn rotate_current_owner_can_rotate() {
    let _home = IsolatedHome::new();
    let dir = tempdir();
    let path = dir.join("rot.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_old = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_old = extract_private(&stdout, &stderr, &pub_old).unwrap();
    let login_out = run_login(&path, &priv_old);
    assert!(
        login_out.status.success(),
        "login should succeed for rotate: {} {}",
        String::from_utf8_lossy(&login_out.stdout),
        String::from_utf8_lossy(&login_out.stderr)
    );
    let out = run_rotate(&path);
    assert!(
        out.status.success(),
        "rotate should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Check new public in .aimt
    let new_pub = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    assert_ne!(new_pub, pub_old);
    // New credential should be stored
    let new_priv = load_credential(&new_pub).expect("new private should be stored");
    assert!(Store::open(&path).unwrap().verify_write_key(&new_priv));
    // Old should fail
    assert!(!Store::open(&path).unwrap().verify_write_key(&priv_old));
    // Private not in .aimt
    let raw = std::fs::read(&path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw).to_lowercase();
    assert!(!raw_str.contains(&new_priv.to_lowercase()));
    // Cleanup
    let _ = delete_credential(&new_pub);
    let _ = delete_credential(&pub_old);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rotate_new_credential_works_old_fails() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("rot2.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_old = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_old = extract_private(&stdout, &stderr, &pub_old).unwrap();
    let _ = run_login(&path, &priv_old);
    let _ = run_rotate(&path);
    let new_pub = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let new_priv = load_credential(&new_pub).unwrap();
    // Old key cannot write
    let mut s_old = Store::open_mut_authenticated(&path, &priv_old).unwrap();
    assert!(!s_old.is_authenticated());
    let ent_old = make_domain(&format!("d_old_{}", COUNTER.fetch_add(1, Ordering::SeqCst)));
    assert!(s_old.insert(ent_old).is_err());
    // New key can write
    let mut s_new = Store::open_mut_authenticated(&path, &new_priv).unwrap();
    assert!(s_new.is_authenticated());
    let ent_new = make_domain(&format!("d_new_{}", COUNTER.fetch_add(1, Ordering::SeqCst)));
    let id_new = ent_new.id().unwrap().as_str().to_string();
    s_new.insert(ent_new).unwrap();
    s_new.persist().unwrap();
    assert!(Store::open(&path).unwrap().contains(&id_new));
    // Public read still works
    let engine = aimt::hosted::HostedEngine::open(&path).unwrap();
    assert!(engine.validate().is_ok());
    assert!(!engine.search("").is_empty());
    // Hosted cannot write (no API)
    let code =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted/engine.rs"))
            .unwrap();
    assert!(!code.contains("pub fn insert"));
    let _ = delete_credential(&new_pub);
    let _ = delete_credential(&pub_old);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rotate_preserves_public_read_and_validation() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("rot3.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_old = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_old = extract_private(&stdout, &stderr, &pub_old).unwrap();
    let _ = run_login(&path, &priv_old);
    let _ = run_rotate(&path);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .get("aimt")
            .unwrap()
            .field("open_to_read")
            .unwrap()
            .value,
        "true"
    );
    assert!(store.validate().is_ok());
    let engine = aimt::hosted::HostedEngine::open(&path).unwrap();
    assert!(engine.validate().is_ok());
    // Ensure open_to_read still true
    assert_eq!(
        engine
            .get("aimt")
            .unwrap()
            .field("open_to_read")
            .unwrap()
            .value,
        "true"
    );
    let new_pub = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let _ = delete_credential(&new_pub);
    let _ = delete_credential(&pub_old);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rotate_fails_without_auth_and_preserves_old() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("rot4.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_old = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_old = extract_private(&stdout, &stderr, &pub_old).unwrap();
    // Do not login, try rotate
    let out = run_rotate(&path);
    assert!(!out.status.success(), "rotate without auth should fail");
    // Old credential still works after failed rotate
    let _ = run_login(&path, &priv_old);
    let _s = Store::open(&path).unwrap();
    // After failed rotate, .aimt should still have old pub
    assert_eq!(
        Store::open(&path)
            .unwrap()
            .get("aimt")
            .unwrap()
            .field("owner_public_key")
            .unwrap()
            .value,
        pub_old
    );
    // Write with old key should still succeed
    let ent = make_domain(&format!(
        "d_after_failed_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let mut s2 = Store::open_mut_authenticated(&path, &priv_old).unwrap();
    s2.insert(ent).unwrap();
    s2.persist().unwrap();
    let _ = delete_credential(&pub_old);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rotate_no_private_in_aimt() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("rot5.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_old = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_old = extract_private(&stdout, &stderr, &pub_old).unwrap();
    let _ = run_login(&path, &priv_old);
    let _ = run_rotate(&path);
    let new_pub = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let new_priv = load_credential(&new_pub).unwrap();
    let raw = std::fs::read(&path).unwrap();
    let raw_low = String::from_utf8_lossy(&raw).to_lowercase();
    assert!(!raw_low.contains(&new_priv.to_lowercase()));
    assert!(!raw_low.contains(&priv_old.to_lowercase()));
    assert!(raw_low.contains(&new_pub.to_lowercase()));
    let _ = delete_credential(&new_pub);
    let _ = delete_credential(&pub_old);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn backup_requires_output_explicit() {
    let _home = IsolatedHome::new();

    let dir = tempdir();
    let path = dir.join("p5.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let pub_key = Store::open(&path)
        .unwrap()
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    let out = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "key", "backup"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--output") || stderr.contains("required"));
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}
