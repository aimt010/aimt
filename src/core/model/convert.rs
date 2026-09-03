use super::entity::AimtEntity;
use super::relation::Relation;
use super::value::Field;
use crate::parser::{self, ParseError, ParsedPmap};
use crate::syntax::{RELATIONS_FIELD, Span};

impl From<ParsedPmap> for AimtEntity {
    fn from(parsed: ParsedPmap) -> Self {
        Self::from_parsed(parsed)
    }
}

impl AimtEntity {
    /// Primary conversion entry point — infallible for syntactically valid input.
    pub fn from_parsed(parsed: ParsedPmap) -> Self {
        let ParsedPmap {
            level,
            level_marker,
            header,
            body,
            span,
        } = parsed;
        let level_span = Span {
            start_line: span.start_line,
            start_col: 1,
            end_line: span.start_line,
            end_col: level_marker.len() + 1,
        };

        let mut header_fields: Vec<Field> = Vec::new();
        let mut body_fields: Vec<Field> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        for pf in header {
            if pf.name == RELATIONS_FIELD {
                for pr in pf.relations {
                    let rel_fields = pr
                        .fields
                        .into_iter()
                        .map(|f| Field {
                            name: f.name,
                            value: f.raw_value,
                            span: f.span,
                        })
                        .collect();
                    relations.push(Relation {
                        fields: rel_fields,
                        span: pr.span,
                    });
                }
            } else {
                header_fields.push(Field {
                    name: pf.name,
                    value: pf.raw_value,
                    span: pf.span,
                });
            }
        }

        for pf in body {
            if pf.name == RELATIONS_FIELD {
                for pr in pf.relations {
                    let rel_fields = pr
                        .fields
                        .into_iter()
                        .map(|f| Field {
                            name: f.name,
                            value: f.raw_value,
                            span: f.span,
                        })
                        .collect();
                    relations.push(Relation {
                        fields: rel_fields,
                        span: pr.span,
                    });
                }
            } else {
                body_fields.push(Field {
                    name: pf.name,
                    value: pf.raw_value,
                    span: pf.span,
                });
            }
        }

        AimtEntity {
            level,
            level_span,
            span,
            header: header_fields,
            body: body_fields,
            relations,
        }
    }

    /// Convenience: parse `&str` directly to model (parser + conversion).
    pub fn parse_and_convert(input: &str) -> Result<Self, ParseError> {
        let parsed = parser::parse(input)?;
        Ok(Self::from_parsed(parsed))
    }

    /// Bytes variant.
    pub fn parse_bytes_and_convert(input: &[u8]) -> Result<Self, ParseError> {
        let parsed = parser::parse_bytes(input)?;
        Ok(Self::from_parsed(parsed))
    }
}
