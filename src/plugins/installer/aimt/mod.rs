// Consolidation mapping (2026-09-02 minimal architecture):
//   index.md      -> navigation only — sorted table Title|File|Description alphabetical by Title (Core, Mapping, Operations, Update)
//   core.md       -> what AIMT means, 7 levels, 18 fields, ownership "One fact, one owner, many references.", knowledge quality
//   operations.md -> OPEN, READ, follow/navigation, WRITE, VALIDATE, CLOSE plus generic rule: initial mapping creates or opens target .aimt via supported runtime/package mechanism; updates open existing .aimt (search optional, not required before READ)
//   mapping.md    -> what /aimt . must accomplish (task-focused)
//   update.md     -> what /aimt update must accomplish (task-focused, incremental, preserve stable IDs)
// OpenCode command: one template .opencode/commands/aimt.md with $ARGUMENTS strict dispatch "." -> mapping, "update" -> update, otherwise unsupported (must read actual prompt files)
// AGENTS.md wording: AIMT = primary knowledge map when available, source = evidence; inspect source when AIMT missing/stale/insufficient
// Deleted files removed entirely: authorization.md, project.md, fields.md, levels.md, search.md, read.md, follow.md, write.md, validate.md, examples.md, errors.md, knowledge.md, schema.md, security.md, README.md

pub const INDEX_MD: &str = include_str!("index.md");
pub const CORE_MD: &str = include_str!("core.md");
pub const MAPPING_MD: &str = include_str!("mapping.md");
pub const OPERATIONS_MD: &str = include_str!("operations.md");
pub const UPDATE_MD: &str = include_str!("update.md");

pub const MODULES: &[(&str, &str)] = &[
    ("index.md", INDEX_MD),
    ("core.md", CORE_MD),
    ("mapping.md", MAPPING_MD),
    ("operations.md", OPERATIONS_MD),
    ("update.md", UPDATE_MD),
];
