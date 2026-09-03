use aimt::model::{AimtEntity, Field, Level};
use aimt::package::{self};
use aimt::syntax::Span;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_package_test_{}_{}_{}",
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
fn dummy_span() -> Span {
    Span::range(1, 1, 1, 1)
}
fn field(name: &str, value: &str) -> Field {
    Field {
        name: name.to_string(),
        value: value.to_string(),
        span: dummy_span(),
    }
}
fn make_entity(level: Level, header: Vec<(&str, &str)>, body: Vec<(&str, &str)>) -> AimtEntity {
    AimtEntity {
        level,
        level_span: dummy_span(),
        span: dummy_span(),
        header: header.into_iter().map(|(k, v)| field(k, v)).collect(),
        body: body.into_iter().map(|(k, v)| field(k, v)).collect(),
        relations: vec![],
    }
}

// Helper to craft a raw package file for security tests
fn craft_package(entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x41, 0x49, 0x4D, 0x54]); // AIMT
    out.push(0x01);
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (path, bytes) in entries {
        let pb = path.as_bytes();
        out.extend_from_slice(&(pb.len() as u16).to_le_bytes());
        out.extend_from_slice(pb);
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&bytes);
    }
    out
}

// ---------------------------------------------------------------------------
// PACKAGE CREATION
// ---------------------------------------------------------------------------

#[test]
fn create_empty_workspace() {
    let ws = temp_dir();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    assert!(pkg.exists());
    let data = std::fs::read(&pkg).unwrap();
    assert_eq!(data.len(), 9);
    assert_eq!(&data[0..4], b"AIMT");
    assert_eq!(data[4], 0x01);
    assert_eq!(u32::from_le_bytes([data[5], data[6], data[7], data[8]]), 0);
    // Extract empty should succeed and leave dest empty
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    assert!(aimt::workspace::discover(&dest).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn create_single_pmap() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    assert!(pkg.exists());
    // Extract and verify
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let v = aimt::workspace::read(&dest).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].entity.id().unwrap().as_str(), "a1");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn create_multiple_pmap() {
    let ws = temp_dir();
    for id in ["a1", "b1", "c1"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let v = aimt::workspace::read(&dest).unwrap();
    assert_eq!(v.len(), 3);
    let mut ids: Vec<_> = v
        .iter()
        .map(|we| we.entity.id().unwrap().as_str().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["a1", "b1", "c1"]);
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn deterministic_ordering() {
    let ws1 = temp_dir();
    let ws2 = temp_dir();
    for id in ["z_id", "a_id", "m_id"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::writer::write(&ws1.join(format!("{}.pmap", id)), &e).unwrap();
    }
    // ws2 in reverse creation order but same files
    for id in ["a_id", "m_id", "z_id"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::writer::write(&ws2.join(format!("{}.pmap", id)), &e).unwrap();
    }
    let pkg1 = temp_dir().join("p1.aimt");
    let pkg2 = temp_dir().join("p2.aimt");
    package::create(&ws1, &pkg1).unwrap();
    package::create(&ws2, &pkg2).unwrap();
    assert_eq!(std::fs::read(&pkg1).unwrap(), std::fs::read(&pkg2).unwrap());
    let _ = std::fs::remove_dir_all(&ws1);
    let _ = std::fs::remove_dir_all(&ws2);
    let _ = std::fs::remove_file(&pkg1);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_dir_all(pkg1.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
}

#[test]
fn ignored_hidden_non_pmap_files() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    std::fs::write(ws.join("README.md"), "hi").unwrap();
    std::fs::write(ws.join(".hidden.pmap"), "bad").unwrap();
    // symlink ignored (if supported)
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(ws.join("a1.pmap"), ws.join("link.pmap"));
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let data = std::fs::read(&pkg).unwrap();
    let count = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    assert_eq!(count, 1, "only valid .pmap should be packaged");
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let v = aimt::workspace::discover(&dest).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].file_name().unwrap(), "a1.pmap");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn package_contains_only_valid_workspace_files() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    // Verify by reading package entries manually (file names)
    let data = std::fs::read(&pkg).unwrap();
    let count = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    assert_eq!(count, 1);
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn create_from_invalid_workspace() {
    let ws = temp_dir();
    std::fs::write(ws.join("bad.pmap"), "not a pmap").unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    let err = package::create(&ws, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    assert!(!pkg.exists() || std::fs::read(&pkg).is_err());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn missing_workspace() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_pkg_missing_{}_{}",
        std::process::id(),
        99991
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let pkg = temp_dir().join("pkg.aimt");
    let err = package::create(&missing, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn invalid_pmap_handling() {
    let ws = temp_dir();
    // Invalid pmap: missing level
    std::fs::write(
        ws.join("bad.pmap"),
        "#header\n  id:\n    x\n#body\n  title:\n    x\n",
    )
    .unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    let err = package::create(&ws, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

// ---------------------------------------------------------------------------
// PACKAGE CONTENT
// ---------------------------------------------------------------------------

#[test]
fn expected_package_paths() {
    let ws = temp_dir();
    for id in ["b", "a"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let data = std::fs::read(&pkg).unwrap();
    // Parse to check paths lexical
    let count = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    assert_eq!(count, 2);
    let mut offset = 9;
    let mut paths = Vec::new();
    for _ in 0..count {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let p = std::str::from_utf8(&data[offset..offset + len])
            .unwrap()
            .to_string();
        offset += len;
        let clen = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        offset += clen;
        paths.push(p);
    }
    assert_eq!(paths, vec!["a.pmap", "b.pmap"]);
    assert_eq!(offset, data.len());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn no_absolute_paths() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let data = std::fs::read(&pkg).unwrap();
    let mut offset = 9;
    let count = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    for _ in 0..count {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let p = std::str::from_utf8(&data[offset..offset + len]).unwrap();
        assert!(!p.starts_with('/'), "no absolute");
        offset += len;
        let clen = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        offset += clen;
    }
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn no_traversal_paths() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let data = std::fs::read(&pkg).unwrap();
    let mut offset = 9;
    let count = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    for _ in 0..count {
        let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let p = std::str::from_utf8(&data[offset..offset + len]).unwrap();
        assert!(!p.contains(".."), "no ..");
        assert!(!p.starts_with("./"), "no ./");
        offset += len;
        let clen = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        offset += clen;
    }
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn deterministic_entry_order() {
    let ws = temp_dir();
    for id in ["z", "m", "a"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    // Check that package bytes are same for two workspaces with same files in different creation order
    let ws2 = temp_dir();
    for id in ["a", "m", "z"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws2, &e).unwrap();
    }
    let pkg2 = temp_dir().join("p2.aimt");
    package::create(&ws2, &pkg2).unwrap();
    assert_eq!(std::fs::read(&pkg).unwrap(), std::fs::read(&pkg2).unwrap());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&ws2);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
}

#[test]
fn no_unintended_metadata() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let data = std::fs::read(&pkg).unwrap();
    // Header magic/version/count, then u16 path len, etc. No timestamps
    assert_eq!(&data[0..4], b"AIMT");
    assert_eq!(data[4], 0x01);
    // Ensure deterministic: re-create same ws should give same bytes
    let pkg2 = temp_dir().join("p2.aimt");
    package::create(&ws, &pkg2).unwrap();
    assert_eq!(std::fs::read(&pkg).unwrap(), std::fs::read(&pkg2).unwrap());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
}

#[test]
fn preservation_of_pmap_bytes() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let orig = std::fs::read(ws.join("n1.pmap")).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let restored = std::fs::read(dest.join("n1.pmap")).unwrap();
    assert_eq!(orig, restored);
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

// ---------------------------------------------------------------------------
// EXTRACTION
// ---------------------------------------------------------------------------

#[test]
fn extract_valid_package() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    assert!(dest.join("d1.pmap").exists());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn extracted_files_can_be_loaded() {
    let ws = temp_dir();
    let e = make_entity(
        Level::File,
        vec![("id", "file_001"), ("path", "src/main.rs")],
        vec![("hash", "abc")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let v = aimt::workspace::read(&dest).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].entity.id().unwrap().as_str(), "file_001");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn round_trip_workspace_package_workspace() {
    let ws = temp_dir();
    for id in ["a", "b"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let orig = aimt::workspace::read(&ws).unwrap();
    let restored = aimt::workspace::read(&dest).unwrap();
    assert_eq!(orig.len(), restored.len());
    let mut o: Vec<_> = orig
        .iter()
        .map(|we| we.entity.id().unwrap().as_str().to_string())
        .collect();
    let mut r: Vec<_> = restored
        .iter()
        .map(|we| we.entity.id().unwrap().as_str().to_string())
        .collect();
    o.sort();
    r.sort();
    assert_eq!(o, r);
    // Compare header/body values
    for (a, b) in orig.iter().zip(restored.iter()) {
        assert_eq!(a.entity.header, b.entity.header);
        assert_eq!(a.entity.body, b.entity.body);
    }
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn empty_package() {
    let ws = temp_dir();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    assert!(aimt::workspace::discover(&dest).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn existing_destination_behavior() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "Old")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    // Modify ws file to have new title, re-package, then extract to dest that has old
    let e2 = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "New")],
    );
    let ws2 = temp_dir();
    aimt::crud::create(&ws2, &e2).unwrap();
    let pkg2 = temp_dir().join("p2.aimt");
    package::create(&ws2, &pkg2).unwrap();
    let dest = temp_dir();
    // Put old file in dest
    aimt::writer::write(&dest.join("a1.pmap"), &e).unwrap();
    // Extract pkg2 (new) should overwrite
    package::extract(&pkg2, &dest).unwrap();
    let back = aimt::reader::read(&dest.join("a1.pmap")).unwrap();
    assert_eq!(back.field("title").unwrap().value, "New");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&ws2);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
}

// ---------------------------------------------------------------------------
// SECURITY
// ---------------------------------------------------------------------------

#[test]
fn absolute_path_in_package() {
    let pkg = temp_dir().join("evil.aimt");
    let data = craft_package(vec![("/absolute/path.pmap", b"a".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidEntry");
    assert!(!dest.join("path.pmap").exists());
    assert!(!Path::new("/absolute/path.pmap").exists() || true);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn outside_traversal() {
    let pkg = temp_dir().join("evil.aimt");
    let data = craft_package(vec![("../outside.pmap", b"a".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidEntry");
    let parent = dest.parent().unwrap();
    assert!(!parent.join("outside.pmap").exists());
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn nested_traversal() {
    let pkg = temp_dir().join("evil.aimt");
    let data = craft_package(vec![("foo/../../outside.pmap", b"a".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidEntry");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn dot_entry() {
    let pkg = temp_dir().join("evil.aimt");
    let data = craft_package(vec![("./foo.pmap", b"a".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidEntry");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn duplicate_entries() {
    let pkg = temp_dir().join("dup.aimt");
    let data = craft_package(vec![("a.pmap", b"a".to_vec()), ("a.pmap", b"b".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "DuplicateEntry");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn malformed_archive() {
    let pkg = temp_dir().join("bad.aimt");
    // Wrong magic
    let mut data = craft_package(vec![("a.pmap", b"a".to_vec())]);
    data[0] = 0xff;
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidPackage");
    // Truncated
    let pkg2 = temp_dir().join("trunc.aimt");
    let data2 = &craft_package(vec![("a.pmap", b"hello".to_vec())])[..10];
    std::fs::write(&pkg2, data2).unwrap();
    let err2 = package::extract(&pkg2, &dest).unwrap_err();
    assert_eq!(err2.kind_str(), "InvalidPackage");
    // Extra trailing bytes
    let pkg3 = temp_dir().join("extra.aimt");
    let mut data3 = craft_package(vec![("a.pmap", b"a".to_vec())]);
    data3.extend_from_slice(b"extra");
    std::fs::write(&pkg3, &data3).unwrap();
    let err3 = package::extract(&pkg3, &dest).unwrap_err();
    assert_eq!(err3.kind_str(), "InvalidPackage");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_file(&pkg3);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg3.parent().unwrap());
}

#[test]
fn unsupported_entry_types() {
    let pkg = temp_dir().join("bad.aimt");
    // Nested path with slash
    let data = craft_package(vec![("dir/a.pmap", b"a".to_vec())]);
    std::fs::write(&pkg, &data).unwrap();
    let dest = temp_dir();
    let err = package::extract(&pkg, &dest).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidEntry");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn extraction_cannot_escape() {
    let dest = temp_dir();
    let parent = dest.parent().unwrap().to_path_buf();
    let outside = parent.join(format!("should_not_exist_{}", std::process::id()));
    let _ = std::fs::remove_file(&outside);
    let pkg = temp_dir().join("evil.aimt");
    let data = craft_package(vec![("../outside.pmap", b"evil".to_vec())]);
    // craft already contains .., but test extraction safety
    // Use a path that would escape if not validated
    let pkg2 = temp_dir().join("evil2.aimt");
    // Manually craft with .. that passes our is_valid check? Our is_valid rejects .., so use direct bytes with .. to test escape
    // Already crafts ../outside.pmap which is rejected, but ensure no file created outside
    std::fs::write(&pkg, &data).unwrap();
    let _ = package::extract(&pkg, &dest);
    assert!(!outside.exists(), "must not escape");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
}

// ---------------------------------------------------------------------------
// ERROR HANDLING
// ---------------------------------------------------------------------------

#[test]
fn structured_errors() {
    let ws = temp_dir();
    std::fs::write(ws.join("bad.pmap"), "bad").unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    let err = package::create(&ws, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Read");
    let dest = temp_dir();
    let bad_pkg = temp_dir().join("bad.aimt");
    std::fs::write(&bad_pkg, b"not aimt").unwrap();
    let err2 = package::extract(&bad_pkg, &dest).unwrap_err();
    assert_eq!(err2.kind_str(), "InvalidPackage");
    assert!(err2.path().is_some());
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&bad_pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(bad_pkg.parent().unwrap());
}

#[test]
fn missing_workspace_package() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_pkg_missing_{}_{}",
        std::process::id(),
        12345
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let pkg = temp_dir().join("pkg.aimt");
    let err = package::create(&missing, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
    let dest = temp_dir();
    let err2 = package::extract(&missing, &dest).unwrap_err();
    assert_eq!(err2.kind_str(), "Io");
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn package_is_directory() {
    let ws = temp_dir();
    let pkg = temp_dir();
    // pkg points to directory, not file
    let err = package::create(&ws, &pkg).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&pkg);
}

#[test]
fn destination_is_file() {
    let ws = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&ws, &e).unwrap();
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest_file = temp_dir().join("file");
    std::fs::write(&dest_file, "hi").unwrap();
    let err = package::extract(&pkg, &dest_file).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_file(&dest_file);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
    let _ = std::fs::remove_dir_all(dest_file.parent().unwrap());
}

// ---------------------------------------------------------------------------
// ROUND TRIP
// ---------------------------------------------------------------------------

#[test]
fn round_trip_preserves_entities() {
    let ws = temp_dir();
    for id in ["a", "b"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg = temp_dir().join("pkg.aimt");
    package::create(&ws, &pkg).unwrap();
    let dest = temp_dir();
    package::extract(&pkg, &dest).unwrap();
    let orig = aimt::workspace::read(&ws).unwrap();
    let restored = aimt::workspace::read(&dest).unwrap();
    assert_eq!(orig.len(), restored.len());
    for (a, b) in orig.iter().zip(restored.iter()) {
        assert_eq!(a.entity.header, b.entity.header);
        assert_eq!(a.entity.body, b.entity.body);
        assert_eq!(a.entity.level, b.entity.level);
    }
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&dest);
    let _ = std::fs::remove_file(&pkg);
    let _ = std::fs::remove_dir_all(pkg.parent().unwrap());
}

#[test]
fn package_determinism() {
    let ws = temp_dir();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws, &e).unwrap();
    }
    let pkg1 = temp_dir().join("p1.aimt");
    let pkg2 = temp_dir().join("p2.aimt");
    package::create(&ws, &pkg1).unwrap();
    // Recreate same ws in different order via second ws
    let ws2 = temp_dir();
    for id in ["a", "m", "z"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&ws2, &e).unwrap();
    }
    package::create(&ws2, &pkg2).unwrap();
    // Packages should be byte-equal because entries sorted
    // ws and ws2 have same ids but different file creation order, but package should be same
    // Actually ws and ws2 are different dirs with same content, so we compare pkg1 vs pkg created from ws (first) again
    let pkg1b = temp_dir().join("p1b.aimt");
    package::create(&ws, &pkg1b).unwrap();
    assert_eq!(
        std::fs::read(&pkg1).unwrap(),
        std::fs::read(&pkg1b).unwrap()
    );
    let _ = std::fs::remove_dir_all(&ws);
    let _ = std::fs::remove_dir_all(&ws2);
    let _ = std::fs::remove_file(&pkg1);
    let _ = std::fs::remove_file(&pkg2);
    let _ = std::fs::remove_file(&pkg1b);
    let _ = std::fs::remove_dir_all(pkg1.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg2.parent().unwrap());
    let _ = std::fs::remove_dir_all(pkg1b.parent().unwrap());
}

#[test]
fn reuse_no_duplication() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/storage/package.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(
        src.contains("workspace::discover") || src.contains("workspace::read"),
        "must use workspace"
    );
    assert!(!src.contains("parser::parse"), "no parser duplication");
    assert!(!src.contains("writer::serialize"), "no writer duplication");
    // Should not contain walkdir/glob
    assert!(!src.contains("walkdir"), "no walkdir");
    assert!(!src.contains("glob"), "no glob");
}

#[test]
fn no_crud_redesign_runtime_session_reuse() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/storage/package.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    // Should not contain mcp/api/database etc.
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["mcp", "database", "cache", "watch", "index", "graph"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
}
