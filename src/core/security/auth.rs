use std::path::PathBuf;

use keyring::Entry;

use crate::core::security::keys::verify_write_credential;

const SERVICE: &str = "aimt";

fn account_for(public_key: &str) -> String {
    format!("owner:{}", public_key.to_lowercase())
}

fn fallback_path(public_key: &str) -> Option<PathBuf> {
    let base = dirs::home_dir()?;
    let dir = base.join(".aimt").join("credentials");
    Some(dir.join(format!("{}.key", public_key.to_lowercase())))
}

#[cfg(not(windows))]
fn fallback_store(public_key: &str, private_hex: &str) -> Result<(), AuthError> {
    let path = fallback_path(public_key).ok_or_else(|| AuthError {
        message: "no home dir for fallback".to_string(),
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AuthError {
            message: format!("failed to create fallback dir: {}", e),
        })?;
    }
    std::fs::write(&path, private_hex).map_err(|e| AuthError {
        message: format!("failed to write fallback credential: {}", e),
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(windows)]
fn fallback_store(_public_key: &str, _private_hex: &str) -> Result<(), AuthError> {
    Err(AuthError {
        message: "secure credential storage unavailable on Windows: Windows Credential Manager (keyring) failed and fallback file is disabled for security. Ensure Credential Manager is available or use a platform with secure storage".to_string(),
    })
}

fn fallback_load(public_key: &str) -> Option<String> {
    let path = fallback_path(public_key)?;
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn fallback_delete(public_key: &str) -> Result<(), AuthError> {
    if let Some(path) = fallback_path(public_key) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

#[derive(Debug)]
pub struct AuthError {
    pub message: String,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "auth error: {}", self.message)
    }
}
impl std::error::Error for AuthError {}

fn entry_for(public_key: &str) -> Result<Entry, AuthError> {
    Entry::new(SERVICE, &account_for(public_key)).map_err(|e| AuthError {
        message: format!("keyring unavailable: {}", e),
    })
}

/// Store private credential securely for the given public key.
/// Tries OS keyring first, falls back to file at ~/.aimt/credentials for environments without keychain.
/// On Windows, fallback is disabled: keyring must succeed, otherwise an error is returned
/// to avoid silently creating a world-readable file.
#[cfg(not(windows))]
pub fn store_credential(public_key: &str, private_hex: &str) -> Result<(), AuthError> {
    // Try keyring first
    if let Ok(entry) = entry_for(public_key)
        && entry.set_password(private_hex).is_ok()
    {
        // Verify it persists cross-process by trying to read back via keyring in same process
        // If keyring is in-memory mock, this will succeed in same process but fail cross-process.
        // We still try keyring, but also write fallback for cross-process reliability.
        // To ensure cross-process, always also write fallback (outside project files).
        let _ = fallback_store(public_key, private_hex);
        return Ok(());
    }
    // Fallback to file (Unix only; Windows fallback_store returns error)
    fallback_store(public_key, private_hex)
}

#[cfg(windows)]
pub fn store_credential(public_key: &str, private_hex: &str) -> Result<(), AuthError> {
    let entry = entry_for(public_key)?;
    entry.set_password(private_hex).map_err(|e| AuthError {
        message: format!(
            "secure credential storage unavailable on Windows: keyring failed ({}). Fallback file is disabled on Windows for security; ensure Windows Credential Manager is available",
            e
        ),
    })?;
    Ok(())
}

/// Load private credential for the given public key, if present.
/// On Unix we prefer the fallback file (`~/.aimt/credentials/<pub>.key` with 0o600)
/// to avoid triggering macOS Keychain UI on every `load_credential` call.
/// The file is written alongside Keychain for cross-process reliability (see
/// `store_credential`), so it is always available after a successful `login`.
/// Keychain is still tried as a fallback if the file is missing (e.g. legacy
/// installations or when the file was manually removed). On Windows the file
/// fallback is disabled and only Keychain/Credential Manager is used.
#[cfg(not(windows))]
pub fn load_credential(public_key: &str) -> Option<String> {
    // Prefer fallback file to avoid Keychain UI prompt on every command.
    // Keychain on macOS may require user presence / Face ID / password for
    // each `get_password` if the item was created with restrictive ACL or if
    // the keychain is locked. The fallback file is already written with 0o600
    // and is not a weaker fallback than what `store_credential` already creates.
    if let Some(pw) = fallback_load(public_key)
        && !pw.is_empty()
    {
        return Some(pw);
    }
    if let Ok(entry) = entry_for(public_key)
        && let Ok(pw) = entry.get_password()
        && !pw.is_empty()
    {
        return Some(pw);
    }
    None
}

#[cfg(windows)]
pub fn load_credential(public_key: &str) -> Option<String> {
    // On Windows, only keyring is used for security; no fallback file.
    if let Ok(entry) = entry_for(public_key)
        && let Ok(pw) = entry.get_password()
        && !pw.is_empty()
    {
        return Some(pw);
    }
    None
}

/// Delete stored credential for the given public key.
pub fn delete_credential(public_key: &str) -> Result<(), AuthError> {
    let mut keyring_err = None;
    if let Ok(entry) = entry_for(public_key) {
        match entry.delete_credential() {
            Ok(()) => {}
            Err(keyring::Error::NoEntry) => {}
            Err(e) => keyring_err = Some(e),
        }
    }
    let _ = fallback_delete(public_key);
    if let Some(e) = keyring_err {
        // If fallback succeeded, don't fail just because keyring delete failed after we cleared fallback
        // Only error if both failed and we had a real error
        #[cfg(not(windows))]
        if fallback_load(public_key).is_some() {
            return Err(AuthError {
                message: format!("failed to delete credential: {}", e),
            });
        }
        #[cfg(windows)]
        {
            return Err(AuthError {
                message: format!("failed to delete credential: {}", e),
            });
        }
    }
    Ok(())
}

/// Verify that a private hex matches the stored public key (ed25519).
pub fn is_valid_for(public_key: &str, private_hex: &str) -> bool {
    verify_write_credential(public_key, private_hex)
}

/// List all stored public keys (from fallback dir).
pub fn list_stored_public_keys() -> Vec<String> {
    let base = match dirs::home_dir() {
        Some(b) => b.join(".aimt").join("credentials"),
        None => return vec![],
    };
    let Ok(entries) = std::fs::read_dir(&base) else {
        return vec![];
    };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("key")
            && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
        {
            out.push(stem.to_string());
        }
    }
    out
}

/// Whether any credential is stored locally.
pub fn is_any_authenticated() -> bool {
    !list_stored_public_keys().is_empty() || {
        // Check keyring for any entry that might not have fallback (legacy)
        // We can't list keyring, but try to see if any fallback exists is sufficient
        false
    }
}

/// Delete all stored credentials (for logout without path).
pub fn delete_all_credentials() -> Result<(), AuthError> {
    let keys = list_stored_public_keys();
    for k in keys {
        delete_credential(&k)?;
    }
    // Also try to clear any keyring entries that have no fallback file
    // We can't enumerate keyring, so fallback deletion is sufficient for our implementation
    Ok(())
}
