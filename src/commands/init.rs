use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use aimt::core::security::keys::generate_keypair;
use aimt::model::{AimtEntity, Field};
use aimt::package::{self, PackageError};
use aimt::store::Store;
use aimt::syntax::{Level, Span};
use aimt::writer::{self, WriterError};

fn print_init_help() {
    println!("AIMT — AI Mapping Taxonomy (init)");
    println!();
    println!("USAGE:");
    println!("  aimt init <path>");
    println!();
    println!("ARGS:");
    println!("  <path>    Path to the .aimt package to create");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help    Show this help");
}

#[derive(Debug)]
pub struct InitError {
    pub path: PathBuf,
    pub kind: std::io::ErrorKind,
    pub message: String,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "init {:?} at {}: {}",
            self.kind,
            self.path.display(),
            self.message
        )
    }
}
impl std::error::Error for InitError {}

impl InitError {
    #[allow(dead_code)]
    pub fn kind_str(&self) -> &'static str {
        match self.kind {
            std::io::ErrorKind::AlreadyExists => "AlreadyExists",
            std::io::ErrorKind::NotFound => "NotFound",
            std::io::ErrorKind::IsADirectory => "IsADirectory",
            std::io::ErrorKind::InvalidInput => "InvalidInput",
            _ => "Io",
        }
    }
}

/// Official AIMT source repository.
///
/// Recorded as `@aimt.source` during `aimt init` so an AI agent reading the
/// `.aimt` knowledge can follow it to learn more about AIMT itself. This is
/// AIMT provenance, not the user's project repository.
pub const AIMT_OFFICIAL_SOURCE: &str = "https://github.com/aimt010/aimt";

fn make_field(name: &str, value: &str) -> Field {
    Field {
        name: name.to_string(),
        value: value.to_string(),
        span: Span::range(1, 1, 1, 1),
    }
}

fn minimal_entities(owner_public_key: &str) -> Vec<AimtEntity> {
    let span = Span::range(1, 1, 1, 1);
    let level_span = Span::range(1, 1, 1, 1);
    let aimt = AimtEntity {
        level: Level::Aimt,
        level_span: level_span.clone(),
        span: span.clone(),
        header: vec![make_field("id", "aimt"), make_field("version", "0.1.0")],
        body: vec![
            make_field("title", "AIMT"),
            make_field("description", "Initialized AIMT package"),
            make_field("open_to_read", "true"),
            make_field("owner_public_key", owner_public_key),
            make_field("source", AIMT_OFFICIAL_SOURCE),
        ],
        relations: vec![],
    };
    let map = AimtEntity {
        level: Level::Map,
        level_span,
        span,
        header: vec![make_field("id", "map"), make_field("title", "Map")],
        body: vec![make_field("description", "Default map")],
        relations: vec![],
    };
    vec![aimt, map]
}

/// Create a new `.aimt` package at `package_path` using existing infrastructure.
/// Validates via `Store::open` + `validate` before reporting success.
/// Generates ed25519 keypair, sets @aimt `open_to_read=true` and `owner_public_key`,
/// prints private key to stderr (never written to .aimt).
pub fn init_package(package_path: &Path) -> Result<String, InitError> {
    if package_path.as_os_str().is_empty() {
        return Err(InitError {
            path: package_path.to_path_buf(),
            kind: std::io::ErrorKind::InvalidInput,
            message: "package path must not be empty".to_string(),
        });
    }
    if let Ok(md) = std::fs::metadata(package_path) {
        if md.is_dir() {
            return Err(InitError {
                path: package_path.to_path_buf(),
                kind: std::io::ErrorKind::IsADirectory,
                message: "package path is a directory".to_string(),
            });
        }
        if md.is_file() {
            return Err(InitError {
                path: package_path.to_path_buf(),
                kind: std::io::ErrorKind::AlreadyExists,
                message: "package already exists; will not overwrite".to_string(),
            });
        }
        return Err(InitError {
            path: package_path.to_path_buf(),
            kind: std::io::ErrorKind::AlreadyExists,
            message: "package path already exists".to_string(),
        });
    }
    if let Some(parent) = package_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(InitError {
                path: package_path.to_path_buf(),
                kind: std::io::ErrorKind::NotFound,
                message: format!("package parent does not exist: {}", parent.display()),
            });
        }
        if !parent.as_os_str().is_empty()
            && let Ok(md) = std::fs::metadata(parent)
            && !md.is_dir()
        {
            return Err(InitError {
                path: package_path.to_path_buf(),
                kind: std::io::ErrorKind::NotFound,
                message: "package parent is not a directory".to_string(),
            });
        }
    }
    let (private_hex, public_hex) = generate_keypair();
    let tmp_base = std::env::temp_dir();
    let uniq = format!(
        "aimt_init_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        rand_suffix()
    );
    let tmp_dir = tmp_base.join(uniq);
    std::fs::create_dir_all(&tmp_dir).map_err(|e| InitError {
        path: tmp_dir.clone(),
        kind: e.kind(),
        message: e.to_string(),
    })?;
    let cleanup = |dir: &Path| {
        let _ = std::fs::remove_dir_all(dir);
    };
    for entity in minimal_entities(&public_hex) {
        let id = entity
            .id()
            .map(|i| i.as_str().to_string())
            .unwrap_or_default();
        let file_name = format!("{}.pmap", id);
        let dest = tmp_dir.join(&file_name);
        if let Err(e) = writer::write(&dest, &entity) {
            cleanup(&tmp_dir);
            let (kind, msg) = match e {
                WriterError::Io(io) => (io.kind, io.message),
                WriterError::Serialize(se) => (std::io::ErrorKind::InvalidInput, se.message),
            };
            return Err(InitError {
                path: dest,
                kind,
                message: msg,
            });
        }
    }
    let pkg_result = package::create(&tmp_dir, package_path);
    if let Err(e) = pkg_result {
        cleanup(&tmp_dir);
        let _ = std::fs::remove_file(package_path);
        let (kind, msg, path) = match e {
            PackageError::Io(io) => (io.kind, io.message, io.path),
            PackageError::InvalidWorkspace(inv) => (inv.kind, inv.message, inv.path),
            PackageError::InvalidPackage(inv) => {
                (std::io::ErrorKind::InvalidData, inv.message, inv.path)
            }
            PackageError::InvalidEntry(inv) => {
                (std::io::ErrorKind::InvalidInput, inv.message, inv.path)
            }
            PackageError::DuplicateEntry(dup) => {
                (std::io::ErrorKind::AlreadyExists, dup.message, dup.path)
            }
            PackageError::Read(re) => (
                std::io::ErrorKind::InvalidData,
                re.to_string(),
                re.path()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| package_path.to_path_buf()),
            ),
            PackageError::ExtractionFailed(io) => (io.kind, io.message, io.path),
        };
        return Err(InitError {
            path,
            kind,
            message: msg,
        });
    }
    cleanup(&tmp_dir);
    let store = Store::open(package_path).map_err(|e| InitError {
        path: package_path.to_path_buf(),
        kind: std::io::ErrorKind::InvalidData,
        message: e.to_string(),
    })?;
    if let Err(errs) = store.validate() {
        let _ = std::fs::remove_file(package_path);
        let msg = errs
            .iter()
            .map(|er| format!("{}:{:?}", er.kind.as_str(), er.field))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(InitError {
            path: package_path.to_path_buf(),
            kind: std::io::ErrorKind::InvalidData,
            message: format!("validation failed: {}", msg),
        });
    }
    // Ensure private key is never embedded in the package file
    debug_assert!({
        let raw = std::fs::read(package_path).unwrap_or_default();
        let raw_str = String::from_utf8_lossy(&raw);
        !raw_str.contains(&private_hex)
    });
    Ok(private_hex)
}

/// Deterministic suffix for temp-dir uniqueness within a process.
/// Not cryptographic; combined with pid + nanos for collision avoidance.
fn rand_suffix() -> String {
    let mut h = DefaultHasher::new();
    std::thread::current().id().hash(&mut h);
    format!("{:x}", h.finish())
}

pub fn run(args: &[String]) {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help" || args[0] == "help") {
        print_init_help();
        return;
    }
    if args.len() > 1 {
        eprintln!("error: too many arguments for init");
        eprintln!("Usage: aimt init <path>");
        std::process::exit(1);
    }
    if !args.is_empty() && (args[0] == "-h" || args[0] == "--help") {
        print_init_help();
        return;
    }
    let raw = if args.is_empty() {
        match crate::commands::project::resolve_init_project(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        crate::commands::project::normalize_project_name(&args[0])
    };
    let path = raw;
    match init_package(&path) {
        Ok(private_hex) => {
            println!("Initialized AIMT package at {}", path.display());
            eprintln!(
                "Owner private key (keep secret, store securely): {}",
                private_hex
            );
            // Private key is printed only to stderr/stdout and never stored in .aimt
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
