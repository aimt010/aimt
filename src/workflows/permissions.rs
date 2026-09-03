use crate::core::data::store::Store;
use crate::workflows::{context::WorkflowContext, definition::WorkflowKind};

/// Portability / read-only boundary for published `.aimt`.
///
/// Published `.aimt` (single-file package or `.aimt` on GitHub) is
/// **read-only by default**. `Store::open` succeeds for read/search/validate;
/// `Store::open_mut` + `persist` is opt-in via the `update` workflow.
/// Plugin must never auto-persist; write requires explicit `update_intent`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Permission {
    ReadOnly,
    ReadWrite,
}

/// Returns the permission for a given workflow kind in the given context.
///
/// - `ReadOnly` is the default for every kind, including `Update` without intent.
/// - `ReadWrite` only when `kind == Update` **and** `ctx.has_update_intent()`
///   **and** `ctx.has_write_credential()` (owner write credential).
///
///   Store verification (`store.verify_write_key`) is enforced in `Store` guards
///   and `UpdateWorkflow`; this check ensures credential presence at permission layer.
///   Legacy packages without `owner_public_key` are handled via
///   `permission_for_with_store` or `Store::needs_auth` fallback (allow without credential).
pub fn permission_for(kind: WorkflowKind, ctx: &WorkflowContext) -> Permission {
    match kind {
        WorkflowKind::Update => {
            if ctx.has_update_intent() && ctx.has_write_credential() {
                Permission::ReadWrite
            } else {
                Permission::ReadOnly
            }
        }
        _ => Permission::ReadOnly,
    }
}

/// Store-aware permission check with legacy fallback.
///
/// - If `store.needs_auth() == false` (no owner_public_key), allows `ReadWrite` with just
///   `has_update_intent()` for backward compat.
/// - Otherwise requires `has_write_credential()` and `store.verify_write_key`,
///   with fallback to OS keyring login (secure storage) when context has no key.
pub fn permission_for_with_store(
    kind: WorkflowKind,
    ctx: &WorkflowContext,
    store: &Store,
) -> Permission {
    match kind {
        WorkflowKind::Update => {
            if !ctx.has_update_intent() {
                return Permission::ReadOnly;
            }
            if !store.needs_auth() {
                return Permission::ReadWrite;
            }
            // Explicit credential in context
            if ctx.has_write_credential() {
                let key = ctx.write_key().unwrap_or("");
                if store.verify_write_key(key) {
                    return Permission::ReadWrite;
                }
            }
            // Fallback: check OS keyring / fallback file for stored owner credential (login)
            if let Some(pub_key) = store.owner_public_key()
                && let Some(private) = crate::core::security::auth::load_credential(&pub_key)
                && store.verify_write_key(&private)
            {
                return Permission::ReadWrite;
            }
            Permission::ReadOnly
        }
        _ => Permission::ReadOnly,
    }
}
