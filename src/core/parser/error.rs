use crate::syntax::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    BomPresent,
    InvalidUtf8,
    LoneCr,
    TabPresent,
    MissingTrailingNewline,
    TooManyTrailingBlankLines,
    TooManyLeadingBlankLines,
    MissingLevel,
    MultipleLevels,
    UnknownLevel,
    InvalidLevelCase,
    TrailingContentAfterLevel,
    InvalidLevelIndent,
    MissingSection,
    DuplicateSection,
    WrongSectionOrder,
    InvalidSection,
    TooManyBlankLinesBetweenSections,
    InvalidIndent,
    InvalidIndentJump,
    MissingColon,
    TrailingContentAfterColon,
    InvalidFieldName,
    InlineValueNotAllowed,
    DuplicateField,
    ConsecutiveBlankLines,
    UnexpectedAtMarkerInPlainText,
    RelationOutsideRelations,
    InvalidRelationPlacement,
    UnexpectedNode,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BomPresent => "BomPresent",
            Self::InvalidUtf8 => "InvalidUtf8",
            Self::LoneCr => "LoneCr",
            Self::TabPresent => "TabPresent",
            Self::MissingTrailingNewline => "MissingTrailingNewline",
            Self::TooManyTrailingBlankLines => "TooManyTrailingBlankLines",
            Self::TooManyLeadingBlankLines => "TooManyLeadingBlankLines",
            Self::MissingLevel => "MissingLevel",
            Self::MultipleLevels => "MultipleLevels",
            Self::UnknownLevel => "UnknownLevel",
            Self::InvalidLevelCase => "InvalidLevelCase",
            Self::TrailingContentAfterLevel => "TrailingContentAfterLevel",
            Self::InvalidLevelIndent => "InvalidLevelIndent",
            Self::MissingSection => "MissingSection",
            Self::DuplicateSection => "DuplicateSection",
            Self::WrongSectionOrder => "WrongSectionOrder",
            Self::InvalidSection => "InvalidSection",
            Self::TooManyBlankLinesBetweenSections => "TooManyBlankLinesBetweenSections",
            Self::InvalidIndent => "InvalidIndent",
            Self::InvalidIndentJump => "InvalidIndentJump",
            Self::MissingColon => "MissingColon",
            Self::TrailingContentAfterColon => "TrailingContentAfterColon",
            Self::InvalidFieldName => "InvalidFieldName",
            Self::InlineValueNotAllowed => "InlineValueNotAllowed",
            Self::DuplicateField => "DuplicateField",
            Self::ConsecutiveBlankLines => "ConsecutiveBlankLines",
            Self::UnexpectedAtMarkerInPlainText => "UnexpectedAtMarkerInPlainText",
            Self::RelationOutsideRelations => "RelationOutsideRelations",
            Self::InvalidRelationPlacement => "InvalidRelationPlacement",
            Self::UnexpectedNode => "UnexpectedNode",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ErrorKind,
    pub span: Span,
    pub message: String,
}

impl ParseError {
    pub(crate) fn new(kind: ErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}:{} — {}",
            self.kind.as_str(),
            self.span.start_line,
            self.span.start_col,
            self.message
        )
    }
}
impl std::error::Error for ParseError {}
