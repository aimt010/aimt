use super::error::{ValidationError, ValidationErrorKind};
use super::rules::{FIELD_RULES, Status, is_valid_id, is_vocab, rule_for, status_for_level};
use crate::model::AimtEntity;

/// Validate one `AimtEntity`. Collects ALL errors, deterministic order.
pub fn validate(entity: &AimtEntity) -> Result<(), Vec<ValidationError>> {
    let mut errors: Vec<ValidationError> = Vec::new();

    let needs_id_shape = |name: &str| matches!(name, "id" | "parent" | "file" | "from" | "to");

    for field in entity.header.iter().chain(entity.body.iter()) {
        if !is_vocab(field.name.as_str()) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::UnknownField,
                field: Some(field.name.clone()),
                span: field.span.clone(),
                message: format!("unknown field '{}'", field.name),
            });
            continue;
        }
        let rule = rule_for(field.name.as_str()).unwrap();
        let status = status_for_level(rule, entity.level);
        if status == Status::NotAllowed {
            errors.push(ValidationError {
                kind: ValidationErrorKind::ForbiddenField,
                field: Some(field.name.clone()),
                span: field.span.clone(),
                message: format!(
                    "field '{}' not allowed for {}",
                    field.name,
                    entity.level.as_str()
                ),
            });
            continue;
        }
        if status == Status::Required && field.value.is_empty() {
            errors.push(ValidationError {
                kind: ValidationErrorKind::EmptyRequiredField,
                field: Some(field.name.clone()),
                span: field.span.clone(),
                message: format!("required field '{}' is empty", field.name),
            });
            continue;
        }
        if needs_id_shape(&field.name) && !field.value.is_empty() && !is_valid_id(&field.value) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidFieldValue,
                field: Some(field.name.clone()),
                span: field.span.clone(),
                message: format!(
                    "field '{}' has invalid ID value '{}'",
                    field.name, field.value
                ),
            });
        }
    }

    for rule in FIELD_RULES {
        let status = status_for_level(rule, entity.level);
        if status != Status::Required {
            continue;
        }
        let present = entity
            .header
            .iter()
            .chain(entity.body.iter())
            .any(|f| f.name == rule.name);
        if !present {
            errors.push(ValidationError {
                kind: ValidationErrorKind::MissingRequiredField,
                field: Some(rule.name.to_string()),
                span: entity.span.clone(),
                message: format!(
                    "missing required field '{}' for {}",
                    rule.name,
                    entity.level.as_str()
                ),
            });
        }
    }

    let relations_rule = rule_for("relations").unwrap();
    let relations_status = status_for_level(relations_rule, entity.level);
    if !entity.relations.is_empty() && relations_status == Status::NotAllowed {
        let span = entity
            .relations
            .first()
            .map(|r| r.span.clone())
            .unwrap_or_else(|| entity.span.clone());
        errors.push(ValidationError {
            kind: ValidationErrorKind::ForbiddenField,
            field: Some("relations".to_string()),
            span,
            message: format!(
                "field 'relations' not allowed for {}",
                entity.level.as_str()
            ),
        });
    }

    for (idx, rel) in entity.relations.iter().enumerate() {
        let mut present: std::collections::HashSet<String> = std::collections::HashSet::new();
        for f in &rel.fields {
            if !is_vocab(f.name.as_str()) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::UnknownField,
                    field: Some(format!("relations[{idx}].{}", f.name)),
                    span: f.span.clone(),
                    message: format!("unknown field '{}' inside relation", f.name),
                });
                continue;
            }
            let rule = rule_for(f.name.as_str()).unwrap();
            if rule.relation_status == Status::NotAllowed {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::ForbiddenField,
                    field: Some(format!("relations[{idx}].{}", f.name)),
                    span: f.span.clone(),
                    message: format!("field '{}' not allowed inside relation", f.name),
                });
                continue;
            }
            if rule.relation_status == Status::Required && f.value.is_empty() {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::EmptyRequiredField,
                    field: Some(format!("relations[{idx}].{}", f.name)),
                    span: f.span.clone(),
                    message: format!("required relation field '{}' is empty", f.name),
                });
                continue;
            }
            if needs_id_shape(&f.name) && !f.value.is_empty() && !is_valid_id(&f.value) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidFieldValue,
                    field: Some(format!("relations[{idx}].{}", f.name)),
                    span: f.span.clone(),
                    message: format!(
                        "relation field '{}' has invalid ID value '{}'",
                        f.name, f.value
                    ),
                });
            }
            if present.contains(&f.name) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::ForbiddenField,
                    field: Some(format!("relations[{idx}].{}", f.name)),
                    span: f.span.clone(),
                    message: format!("duplicate field '{}' inside relation", f.name),
                });
            } else {
                present.insert(f.name.clone());
            }
        }
        for rule in FIELD_RULES {
            if rule.relation_status != Status::Required {
                continue;
            }
            if !present.contains(rule.name) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::MissingRequiredField,
                    field: Some(format!("relations[{idx}].{}", rule.name)),
                    span: rel.span.clone(),
                    message: format!("missing required relation field '{}'", rule.name),
                });
            }
        }
    }

    errors.sort_by(|a, b| {
        a.span
            .start_line
            .cmp(&b.span.start_line)
            .then(a.span.start_col.cmp(&b.span.start_col))
            .then(a.field.cmp(&b.field))
    });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
