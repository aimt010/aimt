use std::path::Path;

/// Secure a file containing a private key on Windows by restricting ACL to owner-only.
/// Uses `icacls` to disable inheritance and grant full control only to the current user.
/// Isolated inside `src/core/security/` so Windows-specific logic does not spread.
/// Returns Ok if ACL was successfully restricted, Err with explanation otherwise.
#[cfg(windows)]
pub fn secure_file(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy().to_string();
    // Disable inheritance and grant current user full control only.
    // `%USERNAME%` expands via icacls; we pass explicit user via environment.
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "CURRENT_USER".to_string());
    // First remove inheritance
    let out = std::process::Command::new("icacls")
        .args([&path_str, "/inheritance:r"])
        .output()
        .map_err(|e| format!("failed to run icacls inheritance:r: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "icacls inheritance:r failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // Grant owner-only
    let grant_arg = format!("{}:F", username);
    let out2 = std::process::Command::new("icacls")
        .args([&path_str, "/grant:r", &grant_arg])
        .output()
        .map_err(|e| format!("failed to run icacls grant: {}", e))?;
    if !out2.status.success() {
        return Err(format!(
            "icacls grant failed: {}",
            String::from_utf8_lossy(&out2.stderr)
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn secure_file(_path: &Path) -> Result<(), String> {
    Ok(())
}
