use crate::syntax::{Level as ParsedLevel, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedField {
    pub name: String,
    pub raw_value: String,
    pub relations: Vec<ParsedRelation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRelation {
    pub fields: Vec<ParsedField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPmap {
    pub level: ParsedLevel,
    pub level_marker: String,
    pub header: Vec<ParsedField>,
    pub body: Vec<ParsedField>,
    pub span: Span,
}
