use crate::syntax::{Level, is_lowercase_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Required,
    Optional,
    NotAllowed,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldRule {
    pub name: &'static str,
    pub per_level: [Status; 7],
    pub relation_status: Status,
}

pub fn level_index(level: Level) -> usize {
    match level {
        Level::Aimt => 0,
        Level::Map => 1,
        Level::Domain => 2,
        Level::Region => 3,
        Level::Node => 4,
        Level::File => 5,
        Level::Frame => 6,
    }
}

pub const VOCAB: &[&str] = &[
    "id",
    "title",
    "description",
    "summary",
    "parent",
    "type",
    "source",
    "context",
    "path",
    "target",
    "location",
    "hash",
    "from",
    "to",
    "evidence",
    "version",
    "created",
    "updated",
    "file",
    "relations",
    "open_to_read",
    "owner_public_key",
];

pub const FIELD_RULES: &[FieldRule] = &[
    FieldRule {
        name: "id",
        per_level: [
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Required,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "title",
        per_level: [
            Status::Optional,
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Required,
            Status::Optional,
            Status::Required,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "description",
        per_level: [
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::Optional,
    },
    FieldRule {
        name: "summary",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "parent",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Required,
            Status::Required,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "type",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Required,
        ],
        relation_status: Status::Required,
    },
    FieldRule {
        name: "source",
        per_level: [
            Status::Optional,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "context",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::NotAllowed,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "path",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Required,
            Status::NotAllowed,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "target",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Required,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "location",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "hash",
        per_level: [
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "file",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Required,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "relations",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "evidence",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::Optional,
    },
    FieldRule {
        name: "version",
        per_level: [
            Status::Required,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "created",
        per_level: [
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "updated",
        per_level: [
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
            Status::Optional,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "from",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
        ],
        relation_status: Status::Required,
    },
    FieldRule {
        name: "to",
        per_level: [
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
        ],
        relation_status: Status::Required,
    },
    FieldRule {
        name: "open_to_read",
        per_level: [
            Status::Optional,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
        ],
        relation_status: Status::NotAllowed,
    },
    FieldRule {
        name: "owner_public_key",
        per_level: [
            Status::Optional,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
            Status::NotAllowed,
        ],
        relation_status: Status::NotAllowed,
    },
];

pub fn rule_for(name: &str) -> Option<&'static FieldRule> {
    FIELD_RULES.iter().find(|r| r.name == name)
}

pub fn status_for_level(rule: &FieldRule, level: Level) -> Status {
    rule.per_level[level_index(level)]
}

pub fn is_vocab(name: &str) -> bool {
    VOCAB.contains(&name)
}

pub fn is_valid_id(v: &str) -> bool {
    !v.is_empty() && is_lowercase_name(v)
}
