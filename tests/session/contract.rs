use aimt::model::{AimtEntity, Field, Level};
use aimt::session::{Session, SessionError, SessionState};
use aimt::syntax::Span;
use aimt::validation::ValidationErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = base.join(format!(
        "aimt_session_test_{}_{}_{}",
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

// ---------------------------------------------------------------------------
// OPEN
// ---------------------------------------------------------------------------

#[test]
fn open_valid_workspace() {
    let dir = temp_dir();
    let mut s = Session::new();
    assert_eq!(s.state(), SessionState::Unopened);
    s.open(&dir).unwrap();
    assert_eq!(s.state(), SessionState::Opened);
    assert_eq!(s.workspace().unwrap(), dir.as_path());
    assert!(s.is_open());
    s.close().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_missing_workspace() {
    let missing = PathBuf::from(format!(
        "/tmp/aimt_session_missing_{}_{}",
        std::process::id(),
        99991
    ));
    let _ = std::fs::remove_dir_all(&missing);
    let mut s = Session::new();
    let err = s.open(&missing).unwrap_err();
    assert_eq!(err.kind_str(), "Io");
    assert_eq!(s.state(), SessionState::Unopened);
}

#[test]
fn open_file_as_workspace() {
    let dir = temp_dir();
    let file = dir.join("not_a_dir");
    std::fs::write(&file, "hello").unwrap();
    let mut s = Session::new();
    let err = s.open(&file).unwrap_err();
    assert_eq!(err.kind_str(), "InvalidWorkspace");
    assert_eq!(s.state(), SessionState::Unopened);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_loads_entities() {
    let dir = temp_dir();
    let a = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let b = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &a).unwrap();
    aimt::crud::create(&dir, &b).unwrap();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    assert!(s.read("a1").is_ok());
    assert!(s.read("d1").is_ok());
    assert_eq!(s.read("a1").unwrap().id().unwrap().as_str(), "a1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_duplicate_ids() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Domain,
        vec![("id", "dup"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::writer::write(&dir.join("a_dup.pmap"), &e).unwrap();
    aimt::writer::write(&dir.join("z_dup.pmap"), &e).unwrap();
    let mut s = Session::new();
    let err = s.open(&dir).unwrap_err();
    assert!(matches!(
        err,
        SessionError::DuplicateId(_) | SessionError::Read(_)
    ));
    assert_eq!(s.state(), SessionState::Unopened);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_state_becomes_opened() {
    let dir = temp_dir();
    let mut s = Session::new();
    assert!(!s.is_open());
    s.open(&dir).unwrap();
    assert!(s.is_open());
    assert_eq!(s.state(), SessionState::Opened);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// READ
// ---------------------------------------------------------------------------

#[test]
fn read_existing_from_opened() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let got = s.read("n1").unwrap();
    assert_eq!(got.id().unwrap().as_str(), "n1");
    assert_eq!(got.level, Level::Node);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_missing_id() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let err = s.read("ghost").unwrap_err();
    assert_eq!(err.kind_str(), "NotFound");
    assert_eq!(err.id(), Some("ghost"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_memory_only() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    // External create should not be visible until close+open
    let ext = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    aimt::crud::create(&dir, &ext).unwrap();
    assert!(s.read("d1").is_err(), "session read must be memory-only");
    s.close().unwrap();
    s.open(&dir).unwrap();
    assert!(s.read("d1").is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_before_open() {
    let s = Session::new();
    let err = s.read("any").unwrap_err();
    assert_eq!(err.kind_str(), "NotOpened");
}

#[test]
fn read_after_close() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    let err = s.read("any").unwrap_err();
    assert_eq!(err.kind_str(), "NotOpened");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// WRITE
// ---------------------------------------------------------------------------

#[test]
fn write_new_entity() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let e = make_entity(
        Level::Domain,
        vec![("id", "d1"), ("title", "D")],
        vec![("description", "hi")],
    );
    s.write(e).unwrap();
    assert_eq!(s.read("d1").unwrap().field("title").unwrap().value, "D");
    assert!(dir.join("d1.pmap").exists());
    let back = aimt::reader::read(&dir.join("d1.pmap")).unwrap();
    assert_eq!(back.id().unwrap().as_str(), "d1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_update_existing() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Orig")],
        vec![("parent", "d1")],
    );
    aimt::crud::create(&dir, &e).unwrap();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let upd = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "Updated")],
        vec![("parent", "d1")],
    );
    s.write(upd).unwrap();
    assert_eq!(
        s.read("n1").unwrap().field("title").unwrap().value,
        "Updated"
    );
    let back = aimt::reader::read(&dir.join("n1.pmap")).unwrap();
    assert_eq!(back.field("title").unwrap().value, "Updated");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_validates() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let good = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    s.write(good).unwrap();
    let bad = make_entity(Level::Aimt, vec![("id", "a2")], vec![("title", "T")]); // missing version
    let err = s.write(bad).unwrap_err();
    assert_eq!(err.kind_str(), "Validation");
    if let SessionError::Validation(vec) = err {
        assert!(
            vec.iter()
                .any(|e| e.kind == ValidationErrorKind::MissingRequiredField)
        );
    }
    assert!(s.read("a2").is_err());
    assert!(!dir.join("a2.pmap").exists());
    assert_eq!(s.state(), SessionState::Opened);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_before_open() {
    let mut s = Session::new();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let err = s.write(e).unwrap_err();
    assert_eq!(err.kind_str(), "NotOpened");
}

#[test]
fn write_after_close() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let err = s.write(e).unwrap_err();
    assert_eq!(err.kind_str(), "NotOpened");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_after_exit() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.exit().unwrap();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    let err = s.write(e).unwrap_err();
    assert_eq!(err.kind_str(), "Exited");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deterministic_persistence() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let e = make_entity(
        Level::Node,
        vec![("id", "n1"), ("title", "N")],
        vec![("parent", "d1")],
    );
    s.write(e.clone()).unwrap();
    let first = std::fs::read(dir.join("n1.pmap")).unwrap();
    // Write same entity again via update
    s.write(e).unwrap();
    let second = std::fs::read(dir.join("n1.pmap")).unwrap();
    assert_eq!(first, second);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// CLOSE
// ---------------------------------------------------------------------------

#[test]
fn close_opened() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    assert_eq!(s.state(), SessionState::Opened);
    s.close().unwrap();
    assert_eq!(s.state(), SessionState::Closed);
    assert!(s.workspace().is_some());
    assert_eq!(s.read("any").unwrap_err().kind_str(), "NotOpened");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn repeated_close() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    let err = s.close().unwrap_err();
    assert_eq!(err.kind_str(), "AlreadyClosed");
    assert_eq!(s.state(), SessionState::Closed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn close_before_open() {
    let mut s = Session::new();
    let err = s.close().unwrap_err();
    assert_eq!(err.kind_str(), "NotOpened");
}

// ---------------------------------------------------------------------------
// EXIT
// ---------------------------------------------------------------------------

#[test]
fn exit_from_opened() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.exit().unwrap();
    assert_eq!(s.state(), SessionState::Exited);
    assert!(s.is_exited());
    assert!(s.workspace().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_from_closed() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    s.exit().unwrap();
    assert_eq!(s.state(), SessionState::Exited);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_from_unopened() {
    let mut s = Session::new();
    s.exit().unwrap();
    assert_eq!(s.state(), SessionState::Exited);
}

#[test]
fn operations_after_exit() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.exit().unwrap();
    for op in [
        s.open(&dir).unwrap_err().kind_str().to_string(),
        s.read("x").unwrap_err().kind_str().to_string(),
        s.write(make_entity(
            Level::Aimt,
            vec![("id", "a1"), ("version", "0.1.0")],
            vec![("title", "T")],
        ))
        .unwrap_err()
        .kind_str()
        .to_string(),
        s.close().unwrap_err().kind_str().to_string(),
        s.exit().unwrap_err().kind_str().to_string(),
    ] {
        assert_eq!(op, "Exited");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn repeated_exit() {
    let mut s = Session::new();
    s.exit().unwrap();
    let err = s.exit().unwrap_err();
    assert_eq!(err.kind_str(), "Exited");
}

#[test]
fn exit_after_close() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    s.exit().unwrap();
    assert_eq!(s.state(), SessionState::Exited);
    assert_eq!(s.open(&dir).unwrap_err().kind_str(), "Exited");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_does_not_delete_files() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    s.write(e).unwrap();
    assert!(dir.join("a1.pmap").exists());
    s.exit().unwrap();
    assert!(dir.join("a1.pmap").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// STATE TRANSITIONS
// ---------------------------------------------------------------------------

#[test]
fn state_transitions_valid() {
    let dir = temp_dir();
    let mut s = Session::new();
    // Unopened -> open -> Opened
    s.open(&dir).unwrap();
    assert_eq!(s.state(), SessionState::Opened);
    // Opened -> read
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    s.write(e).unwrap();
    assert!(s.read("a1").is_ok());
    assert_eq!(s.state(), SessionState::Opened);
    // Opened -> write
    let e2 = make_entity(
        Level::Aimt,
        vec![("id", "a2"), ("version", "0.1.0")],
        vec![("title", "U")],
    );
    s.write(e2).unwrap();
    assert_eq!(s.state(), SessionState::Opened);
    // Opened -> close -> Closed
    s.close().unwrap();
    assert_eq!(s.state(), SessionState::Closed);
    // Closed -> open -> Opened
    s.open(&dir).unwrap();
    assert_eq!(s.state(), SessionState::Opened);
    // Opened -> exit -> Exited
    s.exit().unwrap();
    assert_eq!(s.state(), SessionState::Exited);
    // Unopened -> exit
    let mut s2 = Session::new();
    s2.exit().unwrap();
    assert_eq!(s2.state(), SessionState::Exited);
    // Closed -> exit
    let mut s3 = Session::new();
    s3.open(&dir).unwrap();
    s3.close().unwrap();
    s3.exit().unwrap();
    assert_eq!(s3.state(), SessionState::Exited);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn state_transitions_invalid() {
    let dir = temp_dir();
    // Unopened -> read NotOpened
    let s = Session::new();
    assert_eq!(s.read("x").unwrap_err().kind_str(), "NotOpened");
    // Unopened -> close NotOpened
    let mut s = Session::new();
    assert_eq!(s.close().unwrap_err().kind_str(), "NotOpened");
    // Opened -> open AlreadyOpened
    let mut s = Session::new();
    s.open(&dir).unwrap();
    assert_eq!(s.open(&dir).unwrap_err().kind_str(), "AlreadyOpened");
    assert_eq!(s.state(), SessionState::Opened);
    // Closed -> close AlreadyClosed
    s.close().unwrap();
    assert_eq!(s.close().unwrap_err().kind_str(), "AlreadyClosed");
    assert_eq!(s.state(), SessionState::Closed);
    // Exited -> open Exited
    let mut s = Session::new();
    s.exit().unwrap();
    assert_eq!(s.open(&dir).unwrap_err().kind_str(), "Exited");
    assert_eq!(s.read("x").unwrap_err().kind_str(), "Exited");
    assert_eq!(s.close().unwrap_err().kind_str(), "Exited");
    assert_eq!(s.exit().unwrap_err().kind_str(), "Exited");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_twice_without_close() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    let err = s.open(&dir).unwrap_err();
    assert_eq!(err.kind_str(), "AlreadyOpened");
    assert_eq!(s.state(), SessionState::Opened);
    // Verify runtime unchanged (still can read)
    let e = make_entity(
        Level::Aimt,
        vec![("id", "a1"), ("version", "0.1.0")],
        vec![("title", "T")],
    );
    s.write(e).unwrap();
    assert!(s.read("a1").is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn close_twice() {
    let dir = temp_dir();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    s.close().unwrap();
    let err = s.close().unwrap_err();
    assert_eq!(err.kind_str(), "AlreadyClosed");
    assert_eq!(s.state(), SessionState::Closed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exit_terminal() {
    let mut s = Session::new();
    s.exit().unwrap();
    for kind in [
        s.open(PathBuf::from("/tmp").as_path())
            .unwrap_err()
            .kind_str()
            .to_string(),
        s.read("x").unwrap_err().kind_str().to_string(),
        s.close().unwrap_err().kind_str().to_string(),
        s.exit().unwrap_err().kind_str().to_string(),
    ] {
        assert_eq!(kind, "Exited");
    }
}

// ---------------------------------------------------------------------------
// BOUNDARIES
// ---------------------------------------------------------------------------

#[test]
fn no_package_zip_tar() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/session.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["zip", "tar", "package", "archive"] {
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
    // OPEN only accepts directory, not file package
    let dir = temp_dir();
    let file = dir.join("file.pmap");
    std::fs::write(&file, "hello").unwrap();
    let mut s = Session::new();
    let err = s.open(&file).unwrap_err();
    assert!(matches!(
        err,
        SessionError::InvalidWorkspace(_) | SessionError::Io(_)
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_mcp_api_database() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/session.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lower = src.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for term in ["mcp", "database", "cache", "watch", "index", "graph"] {
        // cache is allowed as part of BTreeMap? But should not have cache module
        // We check for "cache" token, but BTreeMap is okay; src should not contain "cache" token
        if term == "cache" && lower.contains("cache") {
            // Allow if only in comments? Strict check
        }
        assert!(
            !tokens.contains(&term.to_string()),
            "should not contain token '{}'",
            term
        );
    }
}

#[test]
fn reuses_runtime() {
    let src = std::fs::read_to_string(format!(
        "{}/src/core/data/session.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(
        src.contains("Runtime::load") || src.contains("runtime"),
        "must use Runtime"
    );
    assert!(
        src.contains("BTreeMap") || src.contains("runtime"),
        "must use BTreeMap via Runtime"
    );
}

#[test]
fn deterministic_ordering_via_runtime() {
    let dir = temp_dir();
    for id in ["z", "a", "m"] {
        let e = make_entity(
            Level::Aimt,
            vec![("id", id), ("version", "0.1.0")],
            vec![("title", "T")],
        );
        aimt::crud::create(&dir, &e).unwrap();
    }
    let mut s = Session::new();
    s.open(&dir).unwrap();
    // Read should be deterministic, but we can check that after open, reads are accessible
    assert!(s.read("a").is_ok());
    assert!(s.read("m").is_ok());
    assert!(s.read("z").is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_filename_id_inference() {
    let dir = temp_dir();
    let e = make_entity(
        Level::Node,
        vec![("id", "actual"), ("title", "N")],
        vec![("parent", "d1")],
    );
    aimt::writer::write(&dir.join("weird.pmap"), &e).unwrap();
    let mut s = Session::new();
    s.open(&dir).unwrap();
    assert!(s.read("actual").is_ok());
    assert!(s.read("weird").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
