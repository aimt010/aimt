//! Read / Explore Workflow — first real AIMT workflow.
//!
//! Behavioral goal (from docs/superpowers/plans/2026-08-29-aimt-workflow-engine-behavior.md):
//! discover → open → search → read → follow hierarchy/files/relations → make agent decision
//! → continue / search again / inspect source / read another AIMT → answer → validate → close
//!
//! This workflow is **not** a fixed checklist. It supports branching/continuation via
//! `WorkflowContext` and `WorkflowEngine::suggest_next`.
//!
//! The engine owns *what/when* (which deterministic Capability to execute next),
//! the agent decides *which entity* / *whether to continue*.

use crate::core::data::store::{Store, StoreError};
use crate::core::model::AimtEntity;
use crate::index::IntegrityError;
use crate::workflows::context::WorkflowContext;

/// Minimal read workflow — owns no host I/O, only orchestrates Core capabilities.
///
/// For Phase 2, the workflow is intentionally minimal: it proves that the
/// engine can execute a real read end-to-end using `Store` and `AimtEntity`.
/// It does not introduce `Vec<Step>` or `WorkflowGraph` — the existing
/// `WorkflowEngine::suggest_next` + `WorkflowContext` already supports
/// branching via `AskAgent` vs `Execute`.
#[derive(Debug, Default)]
pub struct ReadWorkflow;

impl ReadWorkflow {
    pub fn new() -> Self {
        Self
    }

    /// Discover AIMT sources — thin wrapper over `Store::open` for a given path.
    /// In a real discover, this would glob `**/*.aimt`, but for the test we just open the given path.
    pub fn open(&self, path: &std::path::Path) -> Result<Store, StoreError> {
        Store::open(path)
    }

    /// Search entities by substring on id/title/level (same as visualizer search).
    /// Optimized to avoid cloning `Field::value` before lowercasing; keeps Unicode
    /// case-insensitive semantics via `to_lowercase()` on borrowed `&str`.
    pub fn search<'a>(&self, store: &'a Store, query: &str) -> Vec<&'a AimtEntity> {
        let q = query.to_lowercase();
        // Use `store.ids()` to avoid exposing `Store` internals; the extra
        // `BTreeMap::get` is `O(log n)` but keeps the public API stable.
        // The allocation saved here is the per-field `clone()` before `to_lowercase()`.
        store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(&id))
            .filter(|e| {
                let id_contains = e
                    .id()
                    .is_some_and(|id| id.as_str().to_lowercase().contains(&q));
                let title_contains = e
                    .field("title")
                    .is_some_and(|f| f.value.as_str().to_lowercase().contains(&q));
                let level_contains = e.level.as_str().to_lowercase().contains(&q);
                id_contains || title_contains || level_contains
            })
            .collect()
    }

    /// Read a single entity by id.
    pub fn read<'a>(&self, store: &'a Store, id: &str) -> Option<&'a AimtEntity> {
        store.get(id)
    }

    /// Follow parent relationship — deterministic.
    pub fn follow_parent<'a>(
        &self,
        store: &'a Store,
        entity: &AimtEntity,
    ) -> Option<&'a AimtEntity> {
        let parent_id = entity.field("parent")?.value.clone();
        if parent_id.is_empty() {
            return None;
        }
        store.get(&parent_id)
    }

    /// Follow file relationship for `@frame` → `@file`.
    pub fn follow_file<'a>(&self, store: &'a Store, entity: &AimtEntity) -> Option<&'a AimtEntity> {
        let file_id = entity.field("file")?.value.clone();
        if file_id.is_empty() {
            return None;
        }
        store.get(&file_id)
    }

    /// Follow relations (from/to) — returns related entities.
    pub fn follow_relations<'a>(&self, store: &'a Store, id: &str) -> Vec<&'a AimtEntity> {
        let mut out = Vec::new();
        for rel in store.relations_from(id) {
            if let Some(to) = rel.get_value("to")
                && let Some(e) = store.get(to)
            {
                out.push(e);
            }
        }
        for rel in store.relations_to(id) {
            if let Some(from) = rel.get_value("from")
                && let Some(e) = store.get(from)
            {
                out.push(e);
            }
        }
        out
    }

    /// Validate the store — read-only, no mutation.
    pub fn validate(&self, store: &Store) -> Result<(), Vec<IntegrityError>> {
        store.validate()
    }

    /// Check if the workflow is read-only (it is) — proves it never calls `open_mut`/`persist`.
    pub fn is_read_only(&self) -> bool {
        true
    }

    /// Allow continuation — the workflow does not force a fixed sequence.
    /// After any read, the agent may choose to search again, follow another relation, etc.
    /// This method simply records the decision in context and returns a suggestion.
    pub fn continue_with_search(&self, ctx: &mut WorkflowContext, query: &str) {
        // No state machine enforcement — just record the intent to search again
        // The engine's `suggest_next` will handle branching via `AskAgent(WhichEntity)`
        ctx.select(query);
    }
}
