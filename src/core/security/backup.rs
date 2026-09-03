use std::path::Path;

use crate::core::security::auth::{is_valid_for, list_stored_public_keys, load_credential};

#[derive(Debug)]
pub struct BackupError {
    pub message: String,
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "backup error: {}", self.message)
    }
}
impl std::error::Error for BackupError {}

/// Backup file format version.
pub const BACKUP_VERSION: u32 = 1;

fn secure_permissions(path: &Path) -> Result<(), BackupError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
            BackupError {
                message: format!("failed to set permissions 0o600: {}", e),
            }
        })?;
        Ok(())
    }
    #[cfg(windows)]
    {
        crate::core::security::windows_acl::secure_file(path).map_err(|e| {
            // Delete the insecure file rather than leaving world-readable private key
            let _ = std::fs::remove_file(path);
            BackupError {
                message: format!("failed to secure backup file with owner-only ACL: {}. Backup deleted for security", e),
            }
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Ok(())
    }
}

/// Create a backup file at `output` containing the owner's private key.
/// Requires authentication (stored credential). `output` must be explicit and not overwritten silently.
pub fn create_backup(output: &Path) -> Result<(), BackupError> {
    if output.exists() {
        return Err(BackupError {
            message: format!(
                "backup file already exists at {} (will not overwrite)",
                output.display()
            ),
        });
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(BackupError {
                message: format!("backup parent does not exist: {}", parent.display()),
            });
        }
        if !parent.as_os_str().is_empty()
            && let Ok(md) = std::fs::metadata(parent)
            && !md.is_dir()
        {
            return Err(BackupError {
                message: format!("backup parent is not a directory: {}", parent.display()),
            });
        }
    }
    // Require authentication: at least one stored credential
    let keys = list_stored_public_keys();
    if keys.is_empty() {
        return Err(BackupError {
            message: "not authenticated: no stored credential (run aimt login first)".to_string(),
        });
    }
    // Pick first stored credential (cross-machine: ownership is per key, backup that owner)
    // If multiple, we backup the first; user can specify via --aimt path in future.
    let public_key = keys[0].clone();
    let private_hex = load_credential(&public_key).ok_or_else(|| BackupError {
        message: "failed to load stored credential".to_string(),
    })?;
    if !is_valid_for(&public_key, &private_hex) {
        return Err(BackupError {
            message: "stored credential is invalid (mismatch)".to_string(),
        });
    }
    // Build versioned JSON
    let created = chrono_like_now();
    let content = format!(
        "{{\n  \"version\": {},\n  \"type\": \"aimt-owner-key\",\n  \"public_key\": \"{}\",\n  \"private_key\": \"{}\",\n  \"created\": \"{}\",\n  \"warning\": \"This file contains the AIMT owner private key. Possession grants write access. Keep secure, do not share, do not commit.\"\n}}\n",
        BACKUP_VERSION, public_key, private_hex, created
    );
    // Ensure output not in project directory by default? We allow any outside .aimt, but warn if inside project
    // Do not place it in project directory by default is enforced by requiring explicit --output, not defaulting to project dir
    std::fs::write(output, &content).map_err(|e| BackupError {
        message: format!("failed to write backup: {}", e),
    })?;
    secure_permissions(output)?;
    Ok(())
}

/// Create backup for a specific .aimt's owner (if authenticated for that .aimt).
pub fn create_backup_for(public_key: &str, output: &Path) -> Result<(), BackupError> {
    if output.exists() {
        return Err(BackupError {
            message: format!(
                "backup file already exists at {} (will not overwrite)",
                output.display()
            ),
        });
    }
    let private_hex = load_credential(public_key).ok_or_else(|| BackupError {
        message: "not authenticated for this .aimt owner".to_string(),
    })?;
    if !is_valid_for(public_key, &private_hex) {
        return Err(BackupError {
            message: "stored credential invalid for this owner".to_string(),
        });
    }
    let created = chrono_like_now();
    let content = format!(
        "{{\n  \"version\": {},\n  \"type\": \"aimt-owner-key\",\n  \"public_key\": \"{}\",\n  \"private_key\": \"{}\",\n  \"created\": \"{}\",\n  \"warning\": \"This file contains the AIMT owner private key. Possession grants write access. Keep secure.\"\n}}\n",
        BACKUP_VERSION, public_key, private_hex, created
    );
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| BackupError {
            message: format!("failed to create parent: {}", e),
        })?;
    }
    std::fs::write(output, &content).map_err(|e| BackupError {
        message: format!("failed to write backup: {}", e),
    })?;
    secure_permissions(output)?;
    Ok(())
}

fn chrono_like_now() -> String {
    // Use std time to produce ISO8601 without extra dep
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Simple timestamp; not crucial for backup
    format!("{}", now.as_secs())
}

/// Load private key from backup file and verify it matches public.
pub fn load_from_backup(path: &Path) -> Result<(String, String), BackupError> {
    let content = std::fs::read_to_string(path).map_err(|e| BackupError {
        message: format!("failed to read backup: {}", e),
    })?;
    // Simple JSON parsing without serde: extract private_key and public_key and version
    let version = extract_json_string(&content, "version").unwrap_or_else(|| "1".to_string());
    if version != "1" && version != "1.0" && version != BACKUP_VERSION.to_string() {
        // Allow version 1 for now
    }
    let private_key = extract_json_string(&content, "private_key").ok_or_else(|| BackupError {
        message: "backup missing private_key".to_string(),
    })?;
    let public_key = extract_json_string(&content, "public_key").ok_or_else(|| BackupError {
        message: "backup missing public_key".to_string(),
    })?;
    if !is_valid_for(&public_key, &private_key) {
        return Err(BackupError {
            message: "backup private does not match public".to_string(),
        });
    }
    Ok((private_key, public_key))
}

fn extract_json_string(content: &str, key: &str) -> Option<String> {
    // Find `"key": "value"` or `"key": 1
    let needle = format!("\"{}\"", key);
    let pos = content.find(&needle)?;
    let after = &content[pos + needle.len()..];
    let colon = after.find(':')?;
    let val_start = &after[colon + 1..];
    let trimmed = val_start.trim_start();
    if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        // numeric version
        let end = trimmed.find([',', '\n', '}'])?;
        Some(trimmed[..end].trim().to_string())
    }
}
