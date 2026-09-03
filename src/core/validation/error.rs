use crate::syntax::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErrorKind {
    MissingRequiredField,
    EmptyRequiredField,
    ForbiddenField,
    UnknownField,
    InvalidFieldValue,
}

impl ValidationErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingRequiredField => "MissingRequiredField",
            Self::EmptyRequiredField => "EmptyRequiredField",
            Self::ForbiddenField => "ForbiddenField",
            Self::UnknownField => "UnknownField",
            Self::InvalidFieldValue => "InvalidFieldValue",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    /// Field name or `relations[0].from` style for relation fields.
    pub field: Option<String>,
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} field={:?} at {}:{} — {}",
            self.kind.as_str(),
            self.field,
            self.span.start_line,
            self.span.start_col,
            self.message
        )
    }
}
impl std::error::Error for ValidationError {}
