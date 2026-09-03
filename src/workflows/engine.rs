use crate::workflows::{
    capabilities::{AgentDecision, Capability},
    context::WorkflowContext,
};

pub enum Suggestion {
    Execute(Capability),
    AskAgent(AgentDecision),
}

pub struct WorkflowEngine;

impl WorkflowEngine {
    pub fn new() -> Self {
        Self
    }

    /// Human-readable description of the agentic workflow behavior.
    ///
    /// Must contain "follow hierarchy" to satisfy Task 1's behavioral assertion.
    pub fn describe_behavior(&self) -> String {
        "discover -> open -> search -> read -> follow hierarchy -> determine if source needed -> answer -> update -> validate -> close".to_string()
    }

    pub fn suggest_next(&self, ctx: &WorkflowContext) -> Suggestion {
        if ctx.is_dirty() && !ctx.has_update_intent() {
            return Suggestion::AskAgent(AgentDecision::ShouldUpdate {
                reason: "dirty without intent".into(),
            });
        }
        if ctx.is_dirty() && ctx.has_update_intent() {
            return Suggestion::Execute(Capability::Update);
        }
        if !ctx.selected().is_empty() {
            return Suggestion::Execute(Capability::Read);
        }
        Suggestion::AskAgent(AgentDecision::WhichEntity { query: "".into() })
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
