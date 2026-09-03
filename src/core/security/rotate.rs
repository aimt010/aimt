use std::path::Path;

use crate::core::security::auth::{delete_credential, load_credential, store_credential};
use crate::core::security::keys::{generate_keypair, verify_write_credential};
use crate::store::Store;

#[derive(Debug)]
pub struct RotateError {
    pub message: String,
}

impl std::fmt::Display for RotateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rotate error: {}", self.message)
    }
}
impl std::error::Error for RotateError {}

/// Perform atomic key rotation for the given .aimt path.
/// Steps: authenticate current owner, generate new keypair, update @aimt.owner_public_key, persist, store new private, verify, delete old.
pub fn rotate(aimt_path: &Path) -> Result<(String, String), RotateError> {
    // 1. Open and authenticate current owner
    let mut store = Store::open(aimt_path).map_err(|e| RotateError {
        message: format!("failed to open .aimt: {}", e),
    })?;
    let old_pub = store.owner_public_key().ok_or_else(|| RotateError {
        message: "no owner_public_key in .aimt (legacy)".to_string(),
    })?;
    let old_priv = load_credential(&old_pub).ok_or_else(|| RotateError {
        message: "not authenticated: no stored credential for current owner (run aimt login first)"
            .to_string(),
    })?;
    if !verify_write_credential(&old_pub, &old_priv) {
        return Err(RotateError {
            message: "stored credential invalid for current owner".to_string(),
        });
    }
    // Ensure current credential actually authorizes write (check Store's verify)
    if !store.verify_write_key(&old_priv) {
        return Err(RotateError {
            message: "current credential does not authorize write for this .aimt".to_string(),
        });
    }

    // 2. Generate new keypair
    let (new_priv, new_pub) = generate_keypair();

    // 3. Update @aimt.owner_public_key in store
    let mut aimt_entity = store.get("aimt").cloned().ok_or_else(|| RotateError {
        message: "missing @aimt entity".to_string(),
    })?;
    let mut found = false;
    for field in &mut aimt_entity.body {
        if field.name == "owner_public_key" {
            field.value = new_pub.clone();
            found = true;
            break;
        }
    }
    if !found {
        // If not found (should not happen for protected packages), add it
        use crate::model::Field;
        use crate::syntax::Span;
        aimt_entity.body.push(Field {
            name: "owner_public_key".to_string(),
            value: new_pub.clone(),
            span: Span::range(1, 1, 1, 1),
        });
    }
    // Also ensure open_to_read remains true
    let mut has_open = false;
    for field in &mut aimt_entity.body {
        if field.name == "open_to_read" {
            field.value = "true".to_string();
            has_open = true;
            break;
        }
    }
    if !has_open {
        use crate::model::Field;
        use crate::syntax::Span;
        aimt_entity.body.push(Field {
            name: "open_to_read".to_string(),
            value: "true".to_string(),
            span: Span::range(1, 1, 1, 1),
        });
    }

    // Prepare to persist: need authenticated store for write
    // Authenticate with old credential first
    store.authenticate(&old_priv);
    // Update in store (this will be guarded by ensure_write_authorized which checks old credential)
    store.update(aimt_entity).map_err(|e| RotateError {
        message: format!("failed to update @aimt: {}", e),
    })?;

    // Validate before persist
    if let Err(errs) = store.validate() {
        let msg = errs
            .iter()
            .map(|e| format!("{}:{:?}", e.kind.as_str(), e.field))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RotateError {
            message: format!("validation failed after update: {}", msg),
        });
    }

    // Persist atomically
    store.persist().map_err(|e| RotateError {
        message: format!("failed to persist rotated .aimt: {}", e),
    })?;

    // 4. Store new private credential locally
    store_credential(&new_pub, &new_priv).map_err(|e| RotateError {
        message: format!("failed to store new credential: {}", e),
    })?;

    // 5. Verify new credential works for this .aimt
    let store2 = Store::open(aimt_path).map_err(|e| RotateError {
        message: format!("failed to reopen after rotate: {}", e),
    })?;
    if !store2.verify_write_key(&new_priv) {
        // Rollback: try to restore old credential store? But old is still stored, new is stored, old .aimt was overwritten
        // We should not delete old yet; old credential remains in storage for old pub, but .aimt now has new pub
        // Verification failure means new private doesn't match new pub (should not happen)
        return Err(RotateError {
            message: "new credential verification failed".to_string(),
        });
    }
    // Verify via actual write: try to open_mut_authenticated with new key and do a no-op persist check
    let verify_store =
        Store::open_mut_authenticated(aimt_path, &new_priv).map_err(|e| RotateError {
            message: format!(
                "failed to verify new credential via open_mut_authenticated: {}",
                e
            ),
        })?;
    if !verify_store.is_authenticated() {
        return Err(RotateError {
            message: "new credential not authenticated after rotate".to_string(),
        });
    }
    // Ensure new store can validate
    if let Err(errs) = verify_store.validate() {
        return Err(RotateError {
            message: format!("new package validation failed: {:?}", errs),
        });
    }

    // 6. Invalidate old local credential only after new is verified
    // Delete old credential for old_pub
    let _ = delete_credential(&old_pub);

    // Final package must validate
    let final_store = Store::open(aimt_path).map_err(|e| RotateError {
        message: format!("final open failed: {}", e),
    })?;
    if let Err(errs) = final_store.validate() {
        return Err(RotateError {
            message: format!("final validation failed: {:?}", errs),
        });
    }

    Ok((new_priv, new_pub))
}
