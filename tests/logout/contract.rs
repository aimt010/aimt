use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use aimt::core::security::auth::{delete_credential, load_credential};
use aimt::store::Store;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_logout_test_{}_{}_{}",
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

fn run_logout(path: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", "aimt", "--", "logout"]);
    if let Some(p) = path {
        cmd.arg(p.to_string_lossy().to_string());
    }
    cmd.output().unwrap()
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
fn logout_removes_credential() {
    let dir = tempdir();
    let path = dir.join("p.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    assert!(load_credential(&pub_key).is_some());
    // logout with path
    let out = run_logout(Some(&path));
    assert!(out.status.success());
    assert!(
        load_credential(&pub_key).is_none(),
        "credential should be removed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn logout_without_path_clears_all() {
    let dir = tempdir();
    let path = dir.join("p2.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let _ = run_login(&path, &priv_hex);
    assert!(load_credential(&pub_key).is_some());
    // logout without path should clear all
    let out = run_logout(None);
    assert!(out.status.success());
    assert!(load_credential(&pub_key).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn logout_does_not_modify_aimt() {
    let dir = tempdir();
    let path = dir.join("p3.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok);
    let store = Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let priv_hex = extract_private(&stdout, &stderr, &pub_key).unwrap();
    let raw_before = std::fs::read(&path).unwrap();
    let _ = run_login(&path, &priv_hex);
    let _ = run_logout(Some(&path));
    let raw_after = std::fs::read(&path).unwrap();
    assert_eq!(
        raw_before, raw_after,
        ".aimt must be unchanged after logout"
    );
    let store2 = Store::open(&path).unwrap();
    assert_eq!(
        store2
            .get("aimt")
            .unwrap()
            .field("owner_public_key")
            .unwrap()
            .value,
        pub_key
    );
    assert_eq!(
        store2
            .get("aimt")
            .unwrap()
            .field("open_to_read")
            .unwrap()
            .value,
        "true"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn logout_does_not_affect_public_read() {
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
    let _ = run_logout(Some(&path));
    // Hosted read still works
    let engine = aimt::hosted::HostedEngine::open(&path).unwrap();
    assert!(engine.get("aimt").is_some());
    assert!(engine.validate().is_ok());
    let store = Store::open(&path).unwrap();
    assert!(store.get("map").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_denied_after_logout() {
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
    // login -> write allowed
    let _ = run_login(&path, &priv_hex);
    let mut s = Store::open(&path).unwrap();
    let ent = make_domain(&format!(
        "d_after_login_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    s.insert(ent).expect("write should succeed when logged in");
    s.persist().unwrap();
    // logout
    let _ = run_logout(Some(&path));
    // write should now be denied
    let mut s2 = Store::open(&path).unwrap();
    let ent2 = make_domain(&format!(
        "d_after_logout_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let err = s2.insert(ent2).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    // also via workflow
    let mut s3 = Store::open(&path).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&path);
    ctx.grant_update_intent();
    let ent3 = make_domain(&format!(
        "d_wf_after_logout_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let res = aimt::workflows::update::UpdateWorkflow::new().execute(&mut s3, &mut ctx, ent3);
    assert!(res.is_err());
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn logout_does_not_delete_project_data() {
    let dir = tempdir();
    let path = dir.join("p6.aimt");
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
    let mut s = Store::open(&path).unwrap();
    let id = format!("d_keep_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    s.insert(make_domain(&id)).unwrap();
    s.persist().unwrap();
    let _ = run_logout(Some(&path));
    // data still there
    let s2 = Store::open(&path).unwrap();
    assert!(s2.contains(&id));
    assert_eq!(s2.len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hosted_remains_working_after_logout() {
    let dir = tempdir();
    let path = dir.join("p7.aimt");
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
    let _ = run_logout(None);
    let engine = aimt::hosted::HostedEngine::open(&path).unwrap();
    assert!(engine.validate().is_ok());
    assert!(!engine.search("").is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
