/// 1-indexed source location, reusable across parser and model.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn single(line: usize, col: usize) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }
    pub fn range(sl: usize, sc: usize, el: usize, ec: usize) -> Self {
        Self {
            start_line: sl,
            start_col: sc,
            end_line: el,
            end_col: ec,
        }
    }
}
