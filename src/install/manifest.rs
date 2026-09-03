use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Global installation manifest — stored outside project, tracks Engine version.
#[derive(Debug, Clone)]
pub struct GlobalManifest {
    pub version: String,
    pub install_path: String,
    pub installed_at: String,
}

/// Per-project ownership manifest — stored at .agents/.aimt-manifest.json
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub hash: String,
    pub owned: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectManifest {
    pub version: String,
    pub installed_at: String,
    pub files: BTreeMap<String, FileEntry>,
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub fn global_manifest_path() -> PathBuf {
    if let Ok(p) = std::env::var("AIMT_INSTALL_MANIFEST")
        && !p.trim().is_empty()
    {
        return PathBuf::from(p);
    }
    if let Ok(home) = std::env::var("AIMT_HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home).join("install.json");
    }
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".aimt").join("install.json")
}

pub fn project_manifest_path(project_dir: &Path) -> PathBuf {
    // Keep manifest outside the 5-file .agents/aimt directory to avoid breaking
    // existing tests that expect exactly 5 files there. The manifest is
    // installation metadata, not instruction content.
    project_dir.join(".agents").join(".aimt-manifest.json")
}

fn project_manifest_legacy_path(project_dir: &Path) -> PathBuf {
    project_dir
        .join(".agents")
        .join("aimt")
        .join(".manifest.json")
}

// ---------------------------------------------------------------------------
// Hash
// ---------------------------------------------------------------------------

pub fn hash_content(content: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.to_string()
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn unescape_json(s: &str) -> String {
    // minimal: handle \" and \\
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                match n {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    _ => {
                        out.push('\\');
                        out.push(n);
                    }
                }
            } else {
                out.push('\\');
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Global manifest I/O — hand-rolled JSON, no serde
// ---------------------------------------------------------------------------

pub fn load_global_manifest() -> Option<GlobalManifest> {
    let path = global_manifest_path();
    let content = std::fs::read_to_string(&path).ok()?;
    parse_global(&content)
}

pub fn save_global_manifest(m: &GlobalManifest) -> Result<(), String> {
    let path = global_manifest_path();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = format!(
        "{{\n  \"version\": \"{}\",\n  \"install_path\": \"{}\",\n  \"installed_at\": \"{}\"\n}}\n",
        escape_json(&m.version),
        escape_json(&m.install_path),
        escape_json(&m.installed_at)
    );
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ensure_global_manifest() -> GlobalManifest {
    if let Some(m) = load_global_manifest() {
        return m;
    }
    let version = env!("CARGO_PKG_VERSION").to_string();
    let install_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let m = GlobalManifest {
        version,
        install_path,
        installed_at: now_secs(),
    };
    let _ = save_global_manifest(&m);
    m
}

fn parse_global(content: &str) -> Option<GlobalManifest> {
    let version = extract_json_string(content, "version")?;
    let install_path = extract_json_string(content, "install_path").unwrap_or_default();
    let installed_at = extract_json_string(content, "installed_at").unwrap_or_default();
    Some(GlobalManifest {
        version,
        install_path,
        installed_at,
    })
}

// ---------------------------------------------------------------------------
// Project manifest I/O
// ---------------------------------------------------------------------------

pub fn load_project_manifest(project_dir: &Path) -> Option<ProjectManifest> {
    let path = project_manifest_path(project_dir);
    if let Ok(content) = std::fs::read_to_string(&path)
        && let Some(m) = parse_project(&content)
    {
        return Some(m);
    }
    // Fallback to legacy hidden file inside .agents/aimt for backward compat
    let legacy = project_manifest_legacy_path(project_dir);
    let content = std::fs::read_to_string(&legacy).ok()?;
    parse_project(&content)
}

pub fn save_project_manifest(project_dir: &Path, m: &ProjectManifest) -> Result<(), String> {
    let path = project_manifest_path(project_dir);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Clean up legacy path if exists to avoid confusion
    let legacy = project_manifest_legacy_path(project_dir);
    if legacy.exists() {
        let _ = std::fs::remove_file(&legacy);
    }
    // build files JSON
    let mut files_json = String::new();
    for (i, (name, entry)) in m.files.iter().enumerate() {
        if i > 0 {
            files_json.push_str(",\n");
        }
        files_json.push_str(&format!(
            "    \"{}\": {{\"hash\": \"{}\", \"owned\": {}}}",
            escape_json(name),
            escape_json(&entry.hash),
            if entry.owned { "true" } else { "false" }
        ));
    }
    let json = format!(
        "{{\n  \"version\": \"{}\",\n  \"installed_at\": \"{}\",\n  \"files\": {{\n{}\n  }}\n}}\n",
        escape_json(&m.version),
        escape_json(&m.installed_at),
        files_json
    );
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_project(content: &str) -> Option<ProjectManifest> {
    let version = extract_json_string(content, "version")?;
    let installed_at = extract_json_string(content, "installed_at").unwrap_or_default();
    let mut files = BTreeMap::new();
    // crude parse of files object: find "files": { ... }
    if let Some(files_pos) = content.find("\"files\"") {
        let after = &content[files_pos..];
        if let Some(br_open) = after.find('{') {
            // need to find matching closing for outer files object
            // Simplistic: extract inner until next "}}\n" – but we parse entries by scanning for "\"name\": {" pattern
            // Scan for file entries: regex "\"<name>\": {\"hash\""
            let inner = &after[br_open..];
            // iterate over occurrences of "\"hash\""
            // Instead scan for quoted keys that are not version/installed_at/files/hash/owned
            let mut pos = 0;
            while let Some(q1) = inner[pos..].find('"') {
                let s = pos + q1 + 1;
                if let Some(q2) = inner[s..].find('"') {
                    let key = &inner[s..s + q2];
                    // skip known keys
                    if key == "version"
                        || key == "installed_at"
                        || key == "files"
                        || key == "hash"
                        || key == "owned"
                    {
                        pos = s + q2 + 1;
                        continue;
                    }
                    // check if next non-whitespace after key is colon and then {
                    let after_key = &inner[s + q2 + 1..];
                    let colon = after_key.find(':');
                    if let Some(c) = colon {
                        let after_colon = after_key[c + 1..].trim_start();
                        if after_colon.starts_with('{') {
                            // this is a file entry
                            let hash_key = "\"hash\"";
                            let hk_pos = after_colon.find(hash_key);
                            if let Some(hp) = hk_pos {
                                let after_hash = &after_colon[hp + hash_key.len()..];
                                let colon2 = after_hash.find(':')?;
                                let val = after_hash[colon2 + 1..].trim_start();
                                if let Some(stripped) = val.strip_prefix('"') {
                                    let end = stripped.find('"')?;
                                    let hash_val = &stripped[..end];
                                    let owned = if let Some(op) = after_colon.find("\"owned\"") {
                                        let after_owned = &after_colon[op + 7..];
                                        if let Some(c2) = after_owned.find(':') {
                                            let v = after_owned[c2 + 1..].trim_start();
                                            v.starts_with("true")
                                        } else {
                                            true
                                        }
                                    } else {
                                        true
                                    };
                                    files.insert(
                                        unescape_json(key),
                                        FileEntry {
                                            hash: unescape_json(hash_val),
                                            owned,
                                        },
                                    );
                                }
                            }
                            pos = s + q2 + 1;
                            continue;
                        }
                    }
                    pos = s + q2 + 1;
                } else {
                    break;
                }
            }
        }
    }
    Some(ProjectManifest {
        version,
        installed_at,
        files,
    })
}

fn extract_json_string(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let pos = content.find(&needle)?;
    let after = &content[pos + needle.len()..];
    let colon = after.find(':')?;
    let val_start = &after[colon + 1..];
    let trimmed = val_start.trim_start();
    if let Some(stripped) = trimmed.strip_prefix('"') {
        // find closing unescaped quote
        let mut escaped = false;
        for (i, c) in stripped.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                return Some(unescape_json(&stripped[..i]));
            }
        }
        None
    } else {
        // numeric or bool
        let end = trimmed.find([',', '\n', '}'])?;
        Some(trimmed[..end].trim().to_string())
    }
}

pub fn project_manifest_any_exists(project_dir: &Path) -> bool {
    project_manifest_path(project_dir).exists()
        || project_manifest_legacy_path(project_dir).exists()
}

// ---------------------------------------------------------------------------
// Version helpers
// ---------------------------------------------------------------------------

pub fn normalize_version(v: &str) -> String {
    v.trim().trim_start_matches('v').trim().to_string()
}

pub fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let n = normalize_version(v);
    let parts: Vec<&str> = n.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse::<u64>().ok()?;
    let minor = parts[1].parse::<u64>().ok()?;
    let patch = parts[2].parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let pa = parse_semver(a)?;
    let pb = parse_semver(b)?;
    Some(pa.cmp(&pb))
}

pub fn is_compatible(current: &str, target: &str) -> Result<(), String> {
    let cur =
        parse_semver(current).ok_or_else(|| format!("invalid current version '{}'", current))?;
    let tgt = parse_semver(target).ok_or_else(|| format!("invalid target version '{}'", target))?;
    if tgt < cur {
        return Err(format!(
            "target version {} is older than current {} (downgrade not supported)",
            target, current
        ));
    }
    if tgt == cur {
        return Ok(());
    }
    // Compatibility rule: major version must match.
    // This keeps 0.1.0 -> 0.2.0 compatible, but 0.x -> 1.x incompatible
    if tgt.0 != cur.0 {
        return Err(format!(
            "incompatible version: major version mismatch {} -> {}",
            current, target
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hash() {
        assert_eq!(hash_content("hello"), hash_content("hello"));
        assert_ne!(hash_content("hello"), hash_content("world"));
    }
    #[test]
    fn test_version_compare() {
        assert!(compare_versions("0.1.0", "0.2.0") == Some(std::cmp::Ordering::Less));
        assert!(compare_versions("0.2.0", "0.1.0") == Some(std::cmp::Ordering::Greater));
    }
    #[test]
    fn test_compatible() {
        assert!(is_compatible("0.1.0", "0.2.0").is_ok());
        assert!(is_compatible("0.1.0", "1.0.0").is_err());
        assert!(is_compatible("0.2.0", "0.1.0").is_err());
    }
}
