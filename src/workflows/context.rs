use std::path::{Path, PathBuf};
#[derive(Debug, Default)]
pub struct WorkflowContext {
    aimt_path: Option<PathBuf>,
    selected_ids: Vec<String>,
    dirty: bool,
    update_intent: bool,
    write_key: Option<String>,
}
impl WorkflowContext {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn has_update_intent(&self) -> bool {
        self.update_intent
    }
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    pub fn grant_update_intent(&mut self) {
        self.update_intent = true;
    }
    pub fn select(&mut self, id: &str) {
        self.selected_ids.push(id.to_string());
    }
    pub fn selected(&self) -> &[String] {
        &self.selected_ids
    }
    pub fn set_path(&mut self, p: &Path) {
        self.aimt_path = Some(p.to_path_buf());
    }
    pub fn aimt_path(&self) -> Option<&Path> {
        self.aimt_path.as_deref()
    }
    pub fn set_write_key(&mut self, k: &str) {
        self.write_key = Some(k.to_string());
    }
    pub fn write_key(&self) -> Option<&str> {
        self.write_key.as_deref()
    }
    pub fn has_write_credential(&self) -> bool {
        self.write_key.as_ref().is_some_and(|s| !s.is_empty())
    }
}
