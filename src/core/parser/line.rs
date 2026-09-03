#[derive(Debug, Clone)]
pub(crate) struct LineInfo {
    pub line_no: usize,
    pub indent: usize,
    pub content: String,
    pub is_blank: bool,
    pub has_trailing_space: bool,
}
