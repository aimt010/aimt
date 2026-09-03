use super::Index;
use super::error::{IntegrityError, IntegrityErrorKind};
use crate::index::IndexedEntity;
use crate::syntax::Level;

fn allowed_parent_targets(level: Level) -> Option<&'static [Level]> {
    match level {
        Level::Aimt => None,
        Level::Map => None,
        Level::Domain => Some(&[Level::Map]),
        Level::Region => Some(&[Level::Domain]),
        Level::Node => Some(&[Level::Domain, Level::Region]),
        Level::File => Some(&[Level::Domain, Level::Region, Level::Node]),
        Level::Frame => Some(&[Level::Domain, Level::Region, Level::Node]),
    }
}

pub(crate) fn validate_entities(
    entities: &std::collections::BTreeMap<String, IndexedEntity>,
) -> Result<(), Vec<IntegrityError>> {
    let mut errors: Vec<IntegrityError> = Vec::new();

    for (id, ie) in entities {
        let entity = &ie.entity;
        let level = entity.level;

        if let Some(field) = entity.field("parent") {
            let target = field.value.as_str();
            if target.is_empty() {
                continue;
            }
            match entities.get(target) {
                None => errors.push(IntegrityError {
                    kind: IntegrityErrorKind::UnknownReference,
                    field: Some("parent".to_string()),
                    id: id.clone(),
                    target: target.to_string(),
                    level,
                    target_level: None,
                    span: field.span.clone(),
                    message: format!("parent '{}' not found for '{}'", target, id),
                }),
                Some(target_ie) => {
                    let target_level = target_ie.entity.level;
                    if let Some(allowed) = allowed_parent_targets(level) {
                        if !allowed.contains(&target_level) {
                            errors.push(IntegrityError {
                                kind: IntegrityErrorKind::InvalidParentTarget,
                                field: Some("parent".to_string()),
                                id: id.clone(),
                                target: target.to_string(),
                                level,
                                target_level: Some(target_level),
                                span: field.span.clone(),
                                message: format!(
                                    "parent '{}' level {:?} not allowed for {:?}",
                                    target, target_level, level
                                ),
                            });
                        }
                    } else {
                        errors.push(IntegrityError {
                            kind: IntegrityErrorKind::InvalidParentTarget,
                            field: Some("parent".to_string()),
                            id: id.clone(),
                            target: target.to_string(),
                            level,
                            target_level: Some(target_level),
                            span: field.span.clone(),
                            message: format!("parent not allowed for {:?}", level),
                        });
                    }
                }
            }
        }

        if let Some(field) = entity.field("file") {
            let target = field.value.as_str();
            match entities.get(target) {
                None => errors.push(IntegrityError {
                    kind: IntegrityErrorKind::UnknownReference,
                    field: Some("file".to_string()),
                    id: id.clone(),
                    target: target.to_string(),
                    level,
                    target_level: None,
                    span: field.span.clone(),
                    message: format!("file '{}' not found for '{}'", target, id),
                }),
                Some(target_ie) => {
                    if target_ie.entity.level != Level::File {
                        errors.push(IntegrityError {
                            kind: IntegrityErrorKind::InvalidFileTarget,
                            field: Some("file".to_string()),
                            id: id.clone(),
                            target: target.to_string(),
                            level,
                            target_level: Some(target_ie.entity.level),
                            span: field.span.clone(),
                            message: format!(
                                "file '{}' is {:?}, expected @file",
                                target, target_ie.entity.level
                            ),
                        });
                    }
                }
            }
        }

        for rel in &entity.relations {
            for key in ["from", "to"] {
                if let Some(field) = rel.get(key) {
                    let target = field.value.as_str();
                    let idx = entity
                        .relations
                        .iter()
                        .position(|r| std::ptr::eq(r, rel))
                        .unwrap_or(0);
                    let fname = format!("relations[{}].{}", idx, key);
                    if entities.get(target).is_none() {
                        errors.push(IntegrityError {
                            kind: IntegrityErrorKind::UnknownReference,
                            field: Some(fname),
                            id: id.clone(),
                            target: target.to_string(),
                            level,
                            target_level: None,
                            span: field.span.clone(),
                            message: format!(
                                "relation {} '{}' not found for '{}'",
                                key, target, id
                            ),
                        });
                    }
                }
            }
        }
    }

    errors.sort_by(|a, b| a.id.cmp(&b.id).then(a.field.cmp(&b.field)));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl Index {
    /// Validate referential integrity: parent/file/from/to existence + target level.
    pub fn validate(&self) -> Result<(), Vec<IntegrityError>> {
        validate_entities(&self.entities)
    }
}
