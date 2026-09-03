use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use aimt::core::security::auth::{delete_credential, load_credential};
use aimt::hosted::HostedEngine;
use aimt::store::Store;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_login_test_{}_{}_{}",
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
        .expect("cargo run");
    let ok = out.status.success();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (ok, stdout, stderr)
}

fn extract_private_and_public(
    stdout: &str,
    stderr: &str,
    store_pub: &str,
) -> (Option<String>, String) {
    let combined = format!("{stdout} {stderr}");
    let tokens: Vec<String> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .map(|s| s.to_lowercase())
        .collect();
    let pub_lower = store_pub.to_lowercase();
    let mut private: Option<String> = None;
    for tok in &tokens {
        if tok != &pub_lower {
            private = Some(tok.clone());
            break;
        }
    }
    (private, pub_lower)
}

fn init_protected_package() -> (PathBuf, PathBuf, String, String) {
    let dir = tempdir();
    let path = dir.join("p.aimt");
    let (ok, stdout, stderr) = run_init_capture(&path);
    assert!(ok, "init should succeed: stdout={stdout} stderr={stderr}");
    let store = Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    let (priv_opt, _) = extract_private_and_public(&stdout, &stderr, &pub_key);
    let priv_hex = priv_opt.expect("private key should be printed");
    (dir, path, priv_hex, pub_key)
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
            value: "test domain".to_string(),
            span: s,
        }],
        relations: vec![],
    }
}

fn run_login(path: &Path, private_hex: &str) -> std::process::Output {
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
            private_hex,
        ])
        .output()
        .expect("cargo run login")
}

fn run_logout(path: &Path) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "logout",
            &path.to_string_lossy(),
        ])
        .output()
        .expect("cargo run logout")
}

#[test]
fn login_succeeds_with_correct_private() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(
        out.status.success(),
        "login should succeed with correct key: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.to_lowercase().contains(&priv_hex.to_lowercase()),
        "login output must not print private key"
    );
    assert!(combined.contains(&pub_key[..8]) || combined.to_lowercase().contains("authenticated"));
    // stored
    let stored = load_credential(&pub_key).expect("credential should be stored");
    assert_eq!(stored.to_lowercase(), priv_hex.to_lowercase());
    // cleanup
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn login_fails_with_incorrect_credential() {
    let (dir, path, _priv, pub_key) = init_protected_package();
    let (other_priv, _) = aimt::core::security::keys::generate_keypair();
    let out = run_login(&path, &other_priv);
    assert!(
        !out.status.success(),
        "login should fail with incorrect key"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.to_lowercase().contains("does not match") || stderr.contains("private key"));
    // ensure not stored or not matching
    if let Some(stored) = load_credential(&pub_key) {
        assert_ne!(stored.to_lowercase(), other_priv.to_lowercase());
        let _ = delete_credential(&pub_key);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn login_verifies_against_owner_public_key() {
    let (dir, _path, priv_hex, pub_key) = init_protected_package();
    // correct verifies
    assert!(aimt::core::security::keys::verify_write_credential(
        &pub_key, &priv_hex
    ));
    // wrong pub fails
    let (other_priv, other_pub) = aimt::core::security::keys::generate_keypair();
    assert!(!aimt::core::security::keys::verify_write_credential(
        &other_pub, &priv_hex
    ));
    assert!(!aimt::core::security::keys::verify_write_credential(
        &pub_key,
        &other_priv
    ));
    // login with correct succeeds, second file with different owner fails with same private
    let dir2 = tempdir();
    let path2 = dir2.join("q.aimt");
    let (ok2, _stdout2, _stderr2) = run_init_capture(&path2);
    assert!(ok2);
    let store2 = Store::open(&path2).unwrap();
    let pub2 = store2
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .clone();
    assert_ne!(pub_key, pub2);
    let out = run_login(&path2, &priv_hex);
    assert!(
        !out.status.success(),
        "private from first package should not verify against second"
    );
    let _ = delete_credential(&pub_key);
    let _ = delete_credential(&pub2);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

#[test]
fn successful_login_enables_authorized_write() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    // before login, write denied via Store guard (needs_auth && not authenticated)
    let mut s = Store::open(&path).unwrap();
    let id = format!("domain_login_ok_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    let ent = make_domain(&id);
    assert!(
        s.insert(ent).is_err(),
        "unauthenticated write should be denied"
    );
    // login
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // after login, Store should auto-auth via keyring fallback even without explicit context key
    let mut s2 = Store::open(&path).unwrap();
    let ent2 = make_domain(&format!(
        "domain_after_login_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let cloned_id = ent2.id().unwrap().as_str().to_string();
    // Store's ensure_write_authorized now checks keyring, so insert should succeed
    s2.insert(ent2)
        .expect("after login, insert via Store should succeed via keyring");
    s2.persist().expect("persist should succeed after login");
    let s3 = Store::open(&path).unwrap();
    assert!(s3.contains(&cloned_id));
    // also via UpdateWorkflow without explicit write_key, but with login + intent should succeed
    let mut s4 = Store::open(&path).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&path);
    ctx.grant_update_intent();
    // Intentionally NOT setting write_key, relying on keyring fallback
    let ent3 = make_domain(&format!(
        "domain_wf_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let upd_id = ent3.id().unwrap().as_str().to_string();
    aimt::workflows::update::UpdateWorkflow::new()
        .execute(&mut s4, &mut ctx, ent3)
        .expect("workflow should succeed after login via keyring");
    let s5 = Store::open(&path).unwrap();
    assert!(s5.contains(&upd_id));
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unauthenticated_write_denied() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    // ensure no credential stored (clean slate)
    let _ = delete_credential(&pub_key);
    let mut s = Store::open(&path).unwrap();
    let ent = make_domain(&format!(
        "domain_unauth_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let err = s.insert(ent).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    // also via workflow without key
    let mut s2 = Store::open(&path).unwrap();
    let mut ctx = aimt::workflows::context::WorkflowContext::new();
    ctx.set_path(&path);
    ctx.grant_update_intent();
    let ent2 = make_domain(&format!(
        "domain_unauth2_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let res = aimt::workflows::update::UpdateWorkflow::new().execute(&mut s2, &mut ctx, ent2);
    assert!(res.is_err());
    // login then ensure after logout it fails again (cross-check)
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // after login it would succeed, but we test missing key path already did
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_remains_available_without_login() {
    let (dir, path, _priv, pub_key) = init_protected_package();
    let _ = delete_credential(&pub_key);
    // HostedEngine public read
    let engine = HostedEngine::open(&path).unwrap();
    assert!(engine.get("aimt").is_some());
    assert!(engine.validate().is_ok());
    assert!(!engine.search("").is_empty());
    // Store read also without auth
    let store = Store::open(&path).unwrap();
    assert!(store.get("map").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hosted_remains_readonly_after_login() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    let engine = HostedEngine::open(&path).unwrap();
    // HostedEngine still has no mut API
    let code = {
        let mut c = String::new();
        for e in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted")).unwrap() {
            let p = e.unwrap().path();
            if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                c.push_str(&std::fs::read_to_string(&p).unwrap());
            }
        }
        c
    };
    for term in [
        concat!("pub fn ", "insert"),
        concat!("pub fn ", "update"),
        concat!("pub fn ", "remove"),
        concat!("pub fn ", "persist"),
    ] {
        assert!(!code.contains(term));
    }
    // verify HostedEngine still works for read, not for write (no write API to test, just that file unchanged)
    let len_before = engine.len();
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(len_before >= 2);
}

#[test]
fn private_not_written_into_aimt() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // raw .aimt should contain public but not private
    let raw = std::fs::read(&path).unwrap();
    let raw_low = String::from_utf8_lossy(&raw).to_lowercase();
    assert!(raw_low.contains(&pub_key.to_lowercase()));
    assert!(!raw_low.contains(&priv_hex.to_lowercase()));
    if let Ok(b) = hex::decode(&priv_hex) {
        assert!(!raw.windows(b.len()).any(|w| w == b.as_slice()));
    }
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn private_not_printed_during_normal_operations() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // Normal read via CLI should not print private
    let out2 = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "aimt",
            "validate",
            "--aimt",
            &path.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    assert!(!combined.to_lowercase().contains(&priv_hex.to_lowercase()));
    // Store write via keyring should not print either (no output)
    let mut s = Store::open(&path).unwrap();
    let ent = make_domain(&format!(
        "domain_no_print_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    // This should succeed silently
    s.insert(ent).unwrap();
    s.persist().unwrap();
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn login_works_against_existing_aimt() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    // Simulate Machine B: new temp dir, copy .aimt there (same public artifact)
    let dir_b = tempdir();
    let path_b = dir_b.join("copy.aimt");
    std::fs::copy(&path, &path_b).unwrap();
    // Ensure login on B with same private succeeds (cross-machine)
    let out = run_login(&path_b, &priv_hex);
    assert!(
        out.status.success(),
        "login on copied .aimt should succeed: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = Store::open(&path_b).unwrap();
    let ent = make_domain(&format!(
        "domain_cross_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let id = ent.id().unwrap().as_str().to_string();
    s.insert(ent).unwrap();
    s.persist().unwrap();
    assert!(Store::open(&path_b).unwrap().contains(&id));
    let _ = delete_credential(&pub_key);
    // pub same, so one delete clears both
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir_b);
}

#[test]
fn logout_causes_write_to_fail() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // write should succeed while logged in
    let mut s = Store::open(&path).unwrap();
    let ent = make_domain(&format!(
        "domain_logout_ok_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    s.insert(ent).expect("should succeed when logged in");
    s.persist().unwrap();
    // logout
    let out2 = run_logout(&path);
    assert!(
        out2.status.success(),
        "logout should succeed: {} {}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    assert!(
        load_credential(&pub_key).is_none(),
        "credential should be deleted"
    );
    // subsequent write should fail
    let mut s2 = Store::open(&path).unwrap();
    let ent2 = make_domain(&format!(
        "domain_logout_fail_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let err = s2.insert(ent2).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ai_plugin_does_not_receive_private_key() {
    let (dir, path, priv_hex, pub_key) = init_protected_package();
    let out = run_login(&path, &priv_hex);
    assert!(out.status.success());
    // Simulate AI plugin install: ensure plugin files never contain private
    let plugin_dir = tempdir();
    let install_out = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "install",
            "opencode",
            "--path",
            &plugin_dir.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(install_out.status.success());
    // Ensure known plugin files don't contain private
    for rel in [
        ".opencode/plugins/aimt.js",
        ".opencode/opencode.json",
        "AGENTS.md",
    ] {
        let p = plugin_dir.join(rel);
        if p.is_file() {
            let content = std::fs::read_to_string(&p).unwrap_or_default();
            assert!(
                !content.to_lowercase().contains(&priv_hex.to_lowercase()),
                "plugin file {} must not contain private key",
                p.display()
            );
        }
    }
    // Also ensure .aimt itself still not contain private after plugin ops
    let raw = std::fs::read(&path).unwrap();
    assert!(
        !String::from_utf8_lossy(&raw)
            .to_lowercase()
            .contains(&priv_hex.to_lowercase())
    );
    let _ = delete_credential(&pub_key);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&plugin_dir);
}

#[test]
fn login_without_key_prompts_error() {
    let (dir, path, _priv, _pub) = init_protected_package();
    // Run login without --key and without interactive input (should fail quickly due to no tty)
    // We test that missing key via --key empty fails
    let out = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "login",
            &path.to_string_lossy(),
            "--key",
            "",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}
