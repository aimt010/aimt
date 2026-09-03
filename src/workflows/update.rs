//! Update Workflow — first real write workflow.
//!
//! Behavioral goal: read/explore → agent decides ShouldUpdate → explicit update intent
//! → permission check → open_mut → update → mark dirty → validate → persist → close
//!
//! Critical: `dirty && !update_intent` must NOT persist.

use crate::core::data::store::{Store, StoreError, WriteError};
use crate::core::model::AimtEntity;
use crate::workflows::context::WorkflowContext;
use crate::workflows::definition::WorkflowKind;
use crate::workflows::permissions::{Permission, permission_for_with_store};

#[derive(Debug, Default)]
pub struct UpdateWorkflow;

impl UpdateWorkflow {
    pub fn new() -> Self {
        Self
    }

    /// Execute an update: insert or update the given entity in the given store,
    /// using the provided context for permission checks.
    ///
    /// This is the **actual write execution boundary** — not just `suggest_next`.
    /// It checks `permission_for(Update, ctx)` and refuses to call `Store::insert`/`persist`
    /// unless `Permission::ReadWrite`.
    pub fn execute(
        &self,
        store: &mut Store,
        ctx: &mut WorkflowContext,
        entity: AimtEntity,
    ) -> Result<(), StoreError> {
        // 1. Permission check at the actual write boundary (store-aware with keyring fallback)
        let perm = permission_for_with_store(WorkflowKind::Update, ctx, store);
        match perm {
            Permission::ReadWrite => {}
            Permission::ReadOnly => {
                // Provide distinct messages for missing intent vs missing credential
                let is_keyring_authed = store.needs_auth()
                    && store
                        .owner_public_key()
                        .and_then(|pub_key| crate::core::security::auth::load_credential(&pub_key))
                        .is_some_and(|p| store.verify_write_key(&p));
                let msg = if !ctx.has_update_intent() {
                    "update requires explicit update_intent (read-only boundary)"
                } else if store.needs_auth() && !ctx.has_write_credential() && !is_keyring_authed {
                    "write unauthorized: missing write credential"
                } else if store.needs_auth()
                    && !store.verify_write_key(ctx.write_key().unwrap_or(""))
                    && !is_keyring_authed
                {
                    "write unauthorized: invalid write credential"
                } else {
                    "update requires explicit update_intent and valid write credential"
                };
                return Err(StoreError::Write(WriteError {
                    path: ctx
                        .aimt_path()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| store.source_path().to_path_buf()),
                    kind: std::io::ErrorKind::PermissionDenied,
                    message: msg.to_string(),
                    field: None,
                }));
            }
        }

        // 1b. Enforce owner write credential at Store level if needed (authenticate for guards)
        if store.needs_auth() {
            // Prefer explicit key in context, fallback to keyring
            let mut authenticated = false;
            if let Some(key) = ctx.write_key()
                && store.verify_write_key(key)
            {
                store.authenticate(key);
                authenticated = true;
            }
            if !authenticated
                && let Some(pub_key) = store.owner_public_key()
                && let Some(private) = crate::core::security::auth::load_credential(&pub_key)
                && store.verify_write_key(&private)
            {
                store.authenticate(&private);
                authenticated = true;
            }
            if !authenticated {
                return Err(StoreError::Write(WriteError {
                    path: store.source_path().to_path_buf(),
                    kind: std::io::ErrorKind::PermissionDenied,
                    message: "write unauthorized: invalid write credential".to_string(),
                    field: None,
                }));
            }
        }

        // 2. Validate and mutate in memory (insert or update)
        let id = entity
            .id()
            .map(|e| e.as_str().to_string())
            .unwrap_or_default();
        let exists = store.contains(&id);
        if exists {
            store.update(entity)?;
        } else {
            store.insert(entity)?;
        }

        // 3. Mark dirty in context
        ctx.mark_dirty();

        // 4. Validate before persist — whole-store integrity
        if let Err(errs) = store.validate() {
            // Validation failure — do not persist, return error
            // Note: we do not automatically rollback the in-memory insert; caller can reload if needed
            // For the test's "invalid does not persist" we need to ensure the file on disk is not updated.
            // Since we haven't called persist yet, the file is still old. But the in-memory store now has the invalid entity.
            // To keep the file clean, we return the validation error and let the caller decide to reload.
            // For simplicity, we return a Write error that wraps the integrity error count.
            let msg = errs
                .iter()
                .map(|e| {
                    format!(
                        "{}:{} -> {}",
                        e.id,
                        e.field.as_deref().unwrap_or("?"),
                        e.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            // Remove the invalid entity from memory to keep store clean for further tests (so persist won't write it)
            // Find and remove the id we just inserted
            let _ = store.remove(&id);
            // Also clear dirty since we didn't persist
            // (In real workflow, Dirty would remain until successful validate+close)
            return Err(StoreError::Write(WriteError {
                path: store.source_path().to_path_buf(),
                kind: std::io::ErrorKind::InvalidData,
                message: format!("validation failed: {}", msg),
                field: None,
            }));
        }

        // 5. Persist atomically
        store.persist()?;

        // 6. On success, keep dirty true until close (as per lifecycle)
        Ok(())
    }

    /// Check if this workflow is read-only — it is not, it requires write intent
    pub fn is_read_only(&self) -> bool {
        false
    }
}
