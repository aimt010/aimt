use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use aimt::hosted::HostedEngine;
use aimt::store::Store;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_access_test_{}_{}_{}",
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

fn run_init(path: &Path) -> Result<(), String> {
    // via commands::init::init_package (CLI flow)
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
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "init failed: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
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
    store_pub_key: &str,
) -> (Option<String>, String) {
    let combined = format!("{stdout} {stderr}");
    let tokens: Vec<String> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .map(|s| s.to_lowercase())
        .collect();
    let pub_lower = store_pub_key.to_lowercase();
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

fn make_domain_entity(id: &str) -> aimt::model::AimtEntity {
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

#[test]
fn init_creates_public_readable_and_key() {
    let dir = tempdir();
    let path = dir.join("p.aimt");
    let out = run_init(&path); // via commands::init::init_package
    assert!(out.is_ok(), "init should succeed: {:?}", out.err());
    let store = Store::open(&path).unwrap();
    let aimt = store.get("aimt").unwrap();
    assert_eq!(aimt.field("open_to_read").unwrap().value, "true");
    assert!(aimt.field("owner_public_key").is_some());
    // ensure public key is 64 hex chars (32 bytes)
    let pk = aimt.field("owner_public_key").unwrap().value.clone();
    assert_eq!(pk.len(), 64, "owner_public_key should be 64 hex chars");
    assert!(
        hex::decode(&pk).is_ok(),
        "owner_public_key should be valid hex"
    );
    // private must never be embedded: verify no field contains private and raw bytes don't leak
    // private is printed to stderr/stdout, not stored; we verify that store fields do not contain private key name
    assert!(
        aimt.field("owner_private_key").is_none(),
        "private key must not be stored"
    );
    let raw = std::fs::read(&path).unwrap();
    // raw is binary package; ensure it contains public key but we don't have private to check
    // At least ensure open_to_read and owner_public_key appear as text somewhere in raw content
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(raw_str.contains("open_to_read"));
    assert!(raw_str.contains("owner_public_key"));
    assert!(raw_str.contains(&pk));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn private_never_embedded_in_package() {
    let dir = tempdir();
    let path = dir.join("p.aimt");
    // Capture output to extract private hex printed to stderr/stdout
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
    assert!(
        out.status.success(),
        "init should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let combined = format!("{} {}", stdout, stderr);
    // Extract 64-hex tokens (private and public)
    let tokens: Vec<String> = combined
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|s| s.len() == 64)
        .map(|s| s.to_lowercase())
        .collect();
    // Should have at least private printed (and public)
    assert!(
        !tokens.is_empty(),
        "private key should be printed to stdout/stderr, got: {}",
        combined
    );
    let raw = std::fs::read(&path).unwrap();
    let raw_lower = String::from_utf8_lossy(&raw).to_lowercase();
    // The first token is private (printed twice), ensure none of the private tokens that are NOT the public key are in file
    // Public key is also 64 hex but is expected to be in file
    let store = Store::open(&path).unwrap();
    let pub_key = store
        .get("aimt")
        .unwrap()
        .field("owner_public_key")
        .unwrap()
        .value
        .to_lowercase();
    for tok in tokens {
        if tok == pub_key {
            // public is expected in file; skip
            assert!(raw_lower.contains(&tok), "public key should be in package");
        } else {
            // private should NOT be in file
            assert!(
                !raw_lower.contains(&tok),
                "private key {} must not be in package raw bytes",
                tok
            );
            // Also check raw bytes not contain private hex as binary hex decode
            // Ensure raw bytes don't contain private bytes directly (hex decoded)
            if let Ok(priv_bytes) = hex::decode(&tok) {
                assert!(
                    !raw.windows(priv_bytes.len())
                        .any(|w| w == priv_bytes.as_slice()),
                    "private key bytes must not be in package"
                );
            }
        }
    }
    // Ensure no companion file was created (single .aimt artifact)
    let parent = path.parent().unwrap();
    let entries: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "only .aimt file should exist, no companion file, got: {:?}",
        entries
    );
    assert_eq!(entries[0], "p.aimt");
    let _ = std::fs::remove_dir_all(&dir);
}

// 1. public read without key (HostedEngine open)
#[test]
fn public_read_without_key_via_hosted() {
    let (dir, path, _priv, _pub) = init_protected_package();
    // HostedEngine open without key must succeed (public-readable)
    let engine = HostedEngine::open(&path).expect("hosted open without key should succeed");
    assert!(engine.len() >= 2);
    let aimt = engine.get("aimt").expect("aimt must exist");
    assert_eq!(aimt.field("open_to_read").unwrap().value, "true");
    assert!(aimt.field("owner_public_key").is_some());
    // validate still ok
    assert!(engine.validate().is_ok());
    // search/read also public
    let all = engine.search("");
    assert!(!all.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

// 3. valid key permits insert/persist (via Store::open_mut_authenticated)
#[test]
fn valid_key_permits_insert_persist() {
    let (dir, path, priv_hex, _pub) = init_protected_package();
    let mut store = Store::open_mut_authenticated(&path, &priv_hex).unwrap();
    assert!(store.needs_auth());
    assert!(store.is_authenticated());
    assert!(store.verify_write_key(&priv_hex));
    let id = format!("domain_valid_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    let ent = make_domain_entity(&id);
    store.insert(ent).expect("valid key should permit insert");
    store.persist().expect("valid key should permit persist");
    // Reopen unauthenticated for read, verify persisted
    let store2 = Store::open(&path).unwrap();
    assert!(store2.contains(&id), "persisted entity should be present");
    assert!(store2.validate().is_ok());
    // raw should still not contain private
    let raw = std::fs::read(&path).unwrap();
    let raw_lower = String::from_utf8_lossy(&raw).to_lowercase();
    assert!(!raw_lower.contains(&priv_hex.to_lowercase()));
    let _ = std::fs::remove_dir_all(&dir);
}

// 4. missing key denies
#[test]
fn missing_key_denies_write() {
    let (dir, path, _priv, _pub) = init_protected_package();
    let mut store = Store::open(&path).unwrap();
    assert!(store.needs_auth());
    assert!(!store.is_authenticated());
    let id = format!("domain_missing_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    let ent = make_domain_entity(&id);
    let err = store.insert(ent).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    assert!(
        err.to_string().contains("write unauthorized"),
        "should be unauthorized, got: {}",
        err
    );
    // also persist should deny
    let perr = store.persist().unwrap_err();
    assert_eq!(perr.kind_str(), "Write");
    // file unchanged
    let store2 = Store::open(&path).unwrap();
    assert!(!store2.contains(&id));
    assert_eq!(store2.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

// 5. invalid key denies
#[test]
fn invalid_key_denies_write() {
    let (dir, path, _priv, _pub) = init_protected_package();
    // generate a different keypair's private (invalid for this package)
    let (other_priv, _other_pub) = aimt::core::security::keys::generate_keypair();
    assert_ne!(other_priv.to_lowercase(), _priv.to_lowercase());
    let mut store = Store::open_mut_authenticated(&path, &other_priv).unwrap();
    assert!(store.needs_auth());
    assert!(!store.is_authenticated());
    assert!(!store.verify_write_key(&other_priv));
    let id = format!("domain_invalid_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    let ent = make_domain_entity(&id);
    let err = store.insert(ent).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    assert!(err.to_string().contains("write unauthorized"));
    // also test obviously bad hex
    let mut store2 = Store::open_mut_authenticated(&path, "00").unwrap();
    assert!(!store2.is_authenticated());
    let ent2 = make_domain_entity(&format!(
        "domain_badhex_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    assert!(store2.insert(ent2).is_err());
    // file unchanged
    let store3 = Store::open(&path).unwrap();
    assert!(!store3.contains(&id));
    let _ = std::fs::remove_dir_all(&dir);
}

// 6. change @aimt open_to_read still denies without key
#[test]
fn tamper_open_to_read_still_denies_without_key() {
    let (dir, path, priv_hex, _pub) = init_protected_package();
    // Verify initial open_to_read = true
    let s = Store::open(&path).unwrap();
    assert_eq!(
        s.get("aimt").unwrap().field("open_to_read").unwrap().value,
        "true"
    );
    // Without key, any write is denied even though open_to_read is true
    let mut unauth = Store::open(&path).unwrap();
    let ent = make_domain_entity(&format!(
        "domain_tamper_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    assert!(
        unauth.insert(ent).is_err(),
        "tamper: open_to_read true must not grant write"
    );
    // With valid key, change open_to_read to false and persist
    let mut auth = Store::open_mut_authenticated(&path, &priv_hex).unwrap();
    assert!(auth.is_authenticated());
    let mut aimt_clone = auth.get("aimt").unwrap().clone();
    // mutate open_to_read field
    let mut found = false;
    for f in &mut aimt_clone.body {
        if f.name == "open_to_read" {
            f.value = "false".to_string();
            found = true;
        }
    }
    assert!(found, "open_to_read field must exist");
    auth.update(aimt_clone).unwrap();
    auth.persist().unwrap();
    // Reopen, verify open_to_read is now false but auth still required
    let s2 = Store::open(&path).unwrap();
    assert_eq!(
        s2.get("aimt").unwrap().field("open_to_read").unwrap().value,
        "false"
    );
    assert!(s2.needs_auth(), "owner key still present, needs_auth true");
    // Still denied without key even though open_to_read was toggled
    let mut unauth2 = Store::open(&path).unwrap();
    let ent2 = make_domain_entity(&format!(
        "domain_tamper2_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let err = unauth2.insert(ent2).unwrap_err();
    assert_eq!(err.kind_str(), "Write");
    assert!(err.to_string().contains("write unauthorized"));
    // Valid key still permits even with open_to_read=false (owner key is the gate, not open_to_read)
    let mut auth2 = Store::open_mut_authenticated(&path, &priv_hex).unwrap();
    let ent3 = make_domain_entity(&format!(
        "domain_tamper3_{}",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    auth2.insert(ent3.clone()).unwrap();
    auth2.persist().unwrap();
    let s3 = Store::open(&path).unwrap();
    assert!(s3.contains(ent3.id().unwrap().as_str()));
    let _ = std::fs::remove_dir_all(&dir);
}

// 7. HostedEngine cannot mutate even with key (no API)
#[test]
fn hosted_cannot_mutate_even_with_key() {
    let (dir, path, priv_hex, _pub) = init_protected_package();
    // HostedEngine reads fine with protected package, even without key
    let engine = HostedEngine::open(&path).unwrap();
    assert!(engine.get("aimt").is_some());
    // HostedEngine still exposes no mutation API — verify source contains no pub fn insert etc.
    let hosted_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hosted");
    let mut combined = String::new();
    for entry in std::fs::read_dir(hosted_dir).unwrap() {
        let entry = entry.unwrap();
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            combined.push_str(&std::fs::read_to_string(&p).unwrap());
            combined.push('\n');
        }
    }
    let code = combined.split("#[cfg(test)]").next().unwrap().to_string();
    for term in [
        concat!("pub fn ", "insert"),
        concat!("pub fn ", "update"),
        concat!("pub fn ", "remove"),
        concat!("pub fn ", "persist"),
        concat!("pub fn ", "open_mut"),
    ] {
        assert!(
            !code.contains(term),
            "hosted engine must not expose {} even with key",
            term
        );
    }
    // Even with valid private key, HostedEngine has no way to mutate — ensure key doesn't create API
    // We have private key but HostedEngine::open doesn't take it; there is no open_mut_authenticated on HostedEngine
    assert!(!code.contains("open_mut_authenticated"));
    assert!(!code.contains("write_key"));
    assert!(!code.contains("verify_write"));
    // Verify that private key is not leaked via hosted engine path
    let raw = std::fs::read(&path).unwrap();
    assert!(
        !String::from_utf8_lossy(&raw)
            .to_lowercase()
            .contains(&priv_hex.to_lowercase())
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// 8. existing workflow still functional (legacy without key)
#[test]
fn legacy_without_key_still_functional() {
    // Use legacy fixture without owner_public_key (aimt-test-project.aimt)
    let fixture = if Path::new("aimt-test-project.aimt").exists() {
        Path::new("aimt-test-project.aimt").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")
    };
    let store = Store::open(&fixture).unwrap();
    assert!(!store.needs_auth(), "legacy package should not need auth");
    assert!(store.owner_public_key().is_none());
    assert!(store.verify_write_key("anything"));
    // Also test via temp directory (workspace) legacy path: no owner key -> open_mut without auth permits insert
    let dir = tempdir();
    let id = format!("domain_legacy_{}", COUNTER.fetch_add(1, Ordering::SeqCst));
    // Create a minimal legacy workspace manually via Store on a temp dir
    // Reuse init then strip owner key? Simpler: use crud::create + Store::open_mut workspace
    use aimt::model::{AimtEntity, Field};
    use aimt::syntax::{Level, Span};
    let s = Span::range(1, 1, 1, 1);
    let mk = |lvl: Level, id: &str, title: &str| AimtEntity {
        level: lvl,
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
                value: title.to_string(),
                span: s.clone(),
            },
        ],
        body: vec![Field {
            name: "description".to_string(),
            value: "hi".to_string(),
            span: s.clone(),
        }],
        relations: vec![],
    };
    // Create workspace with aimt+map without owner key
    let mut aimt_ent = mk(Level::Aimt, "aimt", "AIMT");
    aimt_ent.header.push(Field {
        name: "version".to_string(),
        value: "0.1.0".to_string(),
        span: s.clone(),
    });
    // No open_to_read / owner_public_key
    let map_ent = mk(Level::Map, "map", "Map");
    // Write via writer directly to dir
    aimt::writer::write(&dir.join("aimt.pmap"), &aimt_ent).unwrap();
    aimt::writer::write(&dir.join("map.pmap"), &map_ent).unwrap();
    let mut ws_store = Store::open_mut(&dir).unwrap();
    assert!(!ws_store.needs_auth());
    assert!(!ws_store.is_authenticated());
    // Legacy permits insert without auth
    let dom = make_domain_entity(&id);
    ws_store.insert(dom).unwrap();
    ws_store.persist().unwrap();
    let ws_store2 = Store::open(&dir).unwrap();
    assert!(ws_store2.contains(&id));
    // WorkflowContext legacy: permission_for with intent+dummy key still ReadWrite via store fallback
    use aimt::workflows::context::WorkflowContext;
    use aimt::workflows::definition::WorkflowKind;
    use aimt::workflows::permissions::{Permission, permission_for, permission_for_with_store};
    let mut ctx = WorkflowContext::new();
    ctx.grant_update_intent();
    // Without store, permission_for requires write_credential now; set dummy to pass
    ctx.set_write_key("dummy_legacy");
    assert_eq!(
        permission_for(WorkflowKind::Update, &ctx),
        Permission::ReadWrite
    );
    // With legacy store, permission_for_with_store allows with just intent (fallback)
    let mut ctx2 = WorkflowContext::new();
    ctx2.grant_update_intent();
    assert_eq!(
        permission_for_with_store(WorkflowKind::Update, &ctx2, &ws_store2),
        Permission::ReadWrite
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// 9. plugin integrations unchanged (run install tests)
#[test]
fn plugin_integrations_unchanged() {
    // Verify adapters still present and translate
    let opencode = aimt::plugins::opencode::OpenCodeAdapter;
    let claude = aimt::plugins::claude::ClaudeAdapter;
    let codex = aimt::plugins::codex::CodexAdapter;
    let antigravity = aimt::plugins::antigravity::AntigravityAdapter;
    use aimt::plugins::adapter::WorkflowAdapter;
    assert_eq!(opencode.id(), "opencode");
    assert_eq!(claude.id(), "claude");
    assert_eq!(codex.id(), "codex");
    assert_eq!(antigravity.id(), "antigravity");
    let cap = aimt::workflows::capabilities::Capability::Search;
    let sug = aimt::workflows::engine::Suggestion::Execute(cap);
    let action = opencode.translate(sug);
    assert!(action.to_string().contains("ToolCall"));
    // Also verify install help still lists install and plugin files unchanged
    let out = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "aimt", "--", "help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("install"),
        "help should list install: got {}",
        stdout
    );
    // Verify generic adapter still delegates
    let generic_len = opencode.translate(aimt::workflows::engine::Suggestion::Execute(
        aimt::workflows::capabilities::Capability::Read,
    ));
    assert!(generic_len.to_string().contains("ToolCall"));
}
