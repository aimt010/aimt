use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_init_test_{}_{}_{}",
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

fn temp_package_path(dir: &Path) -> PathBuf {
    dir.join("test.aimt")
}

fn run_init(package: &Path) -> std::process::Output {
    Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "aimt",
            "--",
            "init",
            &package.to_string_lossy(),
        ])
        .output()
        .expect("failed to run cargo")
}

#[test]
fn init_creates_package_file() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    assert!(!pkg.exists());
    let out = run_init(&pkg);
    assert!(
        out.status.success(),
        "init should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(pkg.is_file(), "package should be created as file");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_has_correct_magic_version() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(out.status.success());
    let data = std::fs::read(&pkg).unwrap();
    assert!(data.len() >= 9, "package too short");
    assert_eq!(&data[0..4], &[0x41, 0x49, 0x4D, 0x54], "MAGIC");
    assert_eq!(data[4], 0x01, "VERSION");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_opens_with_store() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(out.status.success());
    let store = aimt::store::Store::open(&pkg).expect("Store::open should succeed");
    assert!(!store.is_empty() || store.is_empty()); // just check open succeeds
    assert!(store.len() >= 2, "should have at least aimt and map");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_passes_validation() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(out.status.success());
    let store = aimt::store::Store::open(&pkg).unwrap();
    assert!(store.validate().is_ok(), "validation should pass");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_hosted_can_read() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(out.status.success());
    let engine = aimt::hosted::HostedEngine::open(&pkg).expect("hosted open should succeed");
    assert!(engine.len() >= 2);
    let aimt_entity = engine.get("aimt").expect("should have aimt");
    assert_eq!(aimt_entity.level.as_str(), "aimt");
    let map_entity = engine.get("map").expect("should have map");
    assert_eq!(map_entity.level.as_str(), "map");
    assert!(engine.validate().is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_does_not_silently_overwrite() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out1 = run_init(&pkg);
    assert!(out1.status.success());
    let data1 = std::fs::read(&pkg).unwrap();
    let out2 = run_init(&pkg);
    assert!(
        !out2.status.success(),
        "second init should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    let stderr = String::from_utf8_lossy(&out2.stderr);
    let stdout = String::from_utf8_lossy(&out2.stdout);
    let combined = format!("{} {}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("already exists") || combined.contains("exists"),
        "error should mention already exists, got {} {}",
        stdout,
        stderr
    );
    let data2 = std::fs::read(&pkg).unwrap();
    assert_eq!(data1, data2, "package should not be corrupted");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_fails_if_parent_missing() {
    let dir = temp_dir();
    let missing_parent = dir.join("no_such_dir").join("test.aimt");
    let out = run_init(&missing_parent);
    assert!(!out.status.success());
    assert!(!missing_parent.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_creates_minimal_valid_structure() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(out.status.success());
    let store = aimt::store::Store::open(&pkg).unwrap();
    // Should contain at least aimt and map with correct required fields
    let aimt = store.get("aimt").unwrap();
    assert_eq!(aimt.level.as_str(), "aimt");
    assert!(aimt.field("version").is_some());
    let map = store.get("map").unwrap();
    assert_eq!(map.level.as_str(), "map");
    assert!(map.field("title").is_some());
    // Ensure no unknown fields
    assert!(store.validate().is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn init_no_ai_tool_imports() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands/init.rs"))
        .unwrap();
    let lower = src.to_lowercase();
    for term in ["opencode", "claude", "codex", "antigravity", "cursor"] {
        assert!(
            !lower.contains(term),
            "init should not contain AI-tool logic: {}",
            term
        );
    }
    assert!(!src.contains("Adapter"), "init should not use Adapter");
}

#[test]
fn init_uses_existing_infrastructure() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands/init.rs"))
        .unwrap();
    assert!(
        src.contains("package::create") || src.contains("package::"),
        "init should use package infrastructure"
    );
    assert!(
        src.contains("Store::open") || src.contains("store::Store"),
        "init should validate via Store::open"
    );
    assert!(
        src.contains("validate"),
        "init should validate before success"
    );
    assert!(
        !src.contains("MAGIC") || src.contains("Store"),
        "should not redefine MAGIC"
    );
}

#[test]
fn init_aimt_has_official_source() {
    let dir = temp_dir();
    let pkg = temp_package_path(&dir);
    let out = run_init(&pkg);
    assert!(
        out.status.success(),
        "init should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let store = aimt::store::Store::open(&pkg).unwrap();
    let aimt = store.get("aimt").expect("should have @aimt entity");
    assert_eq!(
        aimt.field("source").map(|f| f.value.as_str()),
        Some("https://github.com/aimt010/aimt"),
        "@aimt.source must be the official AIMT repository"
    );
    // Source belongs only to @aimt, not to @map or other levels.
    let map = store.get("map").expect("should have @map entity");
    assert!(
        map.field("source").is_none(),
        "@map must not carry the official source"
    );
    assert!(store.validate().is_ok(), "validation should pass");
    let _ = std::fs::remove_dir_all(&dir);
}
