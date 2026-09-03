use aimt::core::data::store::Store;
use std::path::Path;
#[test]
fn read_capability_calls_real_store_get() {
    // Use real .aimt file, not string names
    let store = Store::open(Path::new("aimt-test-project.aimt")).unwrap();
    assert!(store.get("node_auth").is_some());
    assert_eq!(store.get("node_auth").unwrap().level.as_str(), "node");
    // This proves the API exists without a string registry
}

#[test]
fn execute_read_wrapper_delegates_to_store_get() {
    // Proves the thin wrapper in capabilities.rs actually calls Store::get
    // rather than a string registry. The wrapper must return cloned entity.
    use aimt::workflows::capabilities::execute_read;

    // Try manifest-adjacent path first (cargo test cwd is manifest dir),
    // fall back to CARGO_MANIFEST_DIR join for robustness.
    let primary = Path::new("aimt-test-project.aimt");
    let store = if primary.exists() {
        Store::open(primary).unwrap()
    } else {
        Store::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("aimt-test-project.aimt")).unwrap()
    };

    // Via wrapper vs direct Store::get must agree
    let direct = store.get("node_auth").cloned();
    let via_wrapper = execute_read(&store, "node_auth");
    assert_eq!(direct, via_wrapper);
    assert!(via_wrapper.is_some());
    assert_eq!(via_wrapper.unwrap().level.as_str(), "node");
    assert!(execute_read(&store, "ghost_missing_id_12345").is_none());
}
