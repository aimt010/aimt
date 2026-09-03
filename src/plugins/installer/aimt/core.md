# AIMT Core

Core principle: **AIMT organizes knowledge; AI provides intelligence.**

Principle: `One fact, one owner, many references.`

This file is self-contained. An AI agent can use AIMT from the installed `.agents/aimt/*` and the `.aimt` package alone, without access to the AIMT development repository or its specification documents. It is the conceptual contract; mapping and runtime procedures are in `mapping.md`, `update.md`, and `operations.md`.

## 1. AIMT identity

AIMT = AI Mapping Taxonomy.

AIMT is a structured knowledge-mapping layer for AI agents. It stores structured knowledge about a project so an AI can navigate, reason, and act with accurate context.

```
Source project
    ↓ evidence
AIMT knowledge map
    ↓ structured context
AI agent/model
    ↓ reasoning/actions
```

AIMT does not replace the source project and does not perform the AI model's reasoning. Source files remain the evidence from which knowledge is derived. `.aimt` is the canonical project knowledge source. `.agents/aimt/*` are instructions, not project knowledge.

## 2. Seven AIMT levels

Exactly seven levels, no others:

```
@aimt
  ↓
@map
  ↓
@domain
  ↓
@region
  ↓
@node
  ↓
@file
  ↓
@frame
```

- `@aimt` — complete project knowledge-map identity and context.
- `@map` — major project-level map and its context.
- `@domain` — meaningful functional or conceptual area.
- `@region` — meaningful subdivision within a domain.
- `@node` — meaningful architectural or conceptual unit.
- `@file` — physical source file.
- `@frame` — meaningful symbol or target within a file, such as class, function, method, variable, or similar source-level target.

Not every project requires every level equally. Model the actual project; do not create empty or artificial hierarchy to satisfy the seven-level shape.

## 3. Level ownership

Each fact should have one appropriate owner at the lowest and most specific level where that fact naturally belongs, while other entities may reference it.

`One fact, one owner, many references.`

Hierarchy (`parent`, file and frame placement) provides navigability. Other relationships connect knowledge across the hierarchy. Do not duplicate the same fact merely to make it visible from multiple locations.

## 4. Exact 18-field vocabulary

Exactly 18 AIMT fields:

`id`, `title`, `description`, `summary`, `parent`, `type`, `source`, `context`, `path`, `target`, `location`, `hash`, `from`, `to`, `evidence`, `version`, `created`, `updated`.

These are the only AIMT fields. Do not invent additional fields. Do not introduce `relations`, `file`, `symbol`, `children`, `name`, or `status` as fields unless the concept is expressed with the 18 fields, the seven levels, or the supported relation structure above.

## 5. Field semantics

Practical guidance for populating the vocabulary. If a field is context-dependent, use it when useful; do not invent required or optional rules beyond what is meaningful.

- `id` — stable identity of the knowledge entity. Use for every entity; referenced by `parent`, `from`, `to`.
- `title` — human-readable name. Short, specific, useful for navigation.
- `description` — fuller explanation of what the entity is and why it matters. Longer than title.
- `summary` — concise one-line gist. Shorter than description; complements title when a brief context is useful.
- `parent` — hierarchical parent reference. Use to place the entity under its natural owner.
- `type` — kind or category. Use to distinguish conceptual or source kinds where helpful.
- `source` — where the knowledge came from at a conceptual level. Use to record provenance or origin.
- `context` — surrounding context that helps interpret the entity. Use when additional framing improves understanding.
- `path` — filesystem path. Use for `@file` to point to the physical file.
- `target` — symbol or target name within a file. Use for `@frame` to point to the specific symbol.
- `location` — precise source location. Use when a line, range, or anchor helps traceability.
- `hash` — content hash of the source at mapping time. Use to detect change and support staleness checks.
- `from` — relation source entity. Use only inside a relation structure to mark the origin.
- `to` — relation destination entity. Use only inside a relation structure to mark the target.
- `evidence` — traceability and support. Use to ground important knowledge in its source evidence.
- `version` — version of the entity or map. Use when tracking evolution matters.
- `created` — creation time. Use to record when the mapped knowledge was first captured.
- `updated` — last update time. Use to record when the mapped knowledge was last verified or changed.

Distinctions: `title` is a name, `summary` is a gist, `description` is the full account; `source` is provenance, `evidence` is support; `path` is the file, `location` is the precise anchor in that file, `target` is the symbol; `parent` is hierarchy, `from` and `to` are relation endpoints; `created` is first capture, `updated` is last change; `hash` is content fingerprint, `version` is semantic versioning.

## 6. Relations

Relations are not a 19th AIMT field. `@relation` is a supported nested relation structure. Relations use the existing fields that apply to the relationship, especially `type`, `from`, `to`, `evidence`.

- `type` — the kind of relationship.
- `from` — the source entity of the relationship.
- `to` — the destination entity of the relationship.
- `evidence` — support for why the relationship exists.

Relations should represent meaningful project knowledge, not arbitrary links. Do not create a `relations` AIMT field.

## 7. Identity and IDs

`id` identifies a knowledge entity. A conceptual entity that remains the same should retain its existing ID. Changing source content does not automatically mean creating a new entity. A new ID should represent genuinely new knowledge. This stability is what allows `update.md` to preserve identity across changes. Do not prescribe a specific ID-generation algorithm.

## 8. Source and evidence

Source project is evidence; `.aimt` is canonical structured knowledge. AIMT knowledge must be grounded in the actual project. `evidence` should provide traceability for important mapped knowledge when applicable. Do not invent project facts because they seem reasonable. If something cannot be established from available evidence, do not represent speculation as established knowledge.

## 9. Context and traceability

Fields such as `context`, `source`, `evidence`, `location`, `path`, and `target` exist to make the map useful, not merely to create a tree of names. A useful AIMT map should allow an AI agent to understand what something is, where it exists, why it exists, what it belongs to, what it relates to, what evidence supports it, and how it connects to other project knowledge.

## 10. Optional fields

Optional does not mean ignore. When a field is applicable and useful, populate it. Use all applicable fields from the 18-field vocabulary when they improve context, ownership, relationships, evidence, traceability, source location, or change tracking. Do not blindly populate meaningless values to fill every field; the objective is complete useful knowledge, not artificial verbosity.

## 11. Knowledge quality rules

A valid AIMT map is grounded in project evidence, structurally coherent, connected, traceable, non-duplicative, appropriately scoped, internally consistent, and useful for progressive AI navigation.

Avoid invented facts, duplicate ownership, meaningless relations, empty placeholder entities, unsupported fields, unsupported levels, source-code duplication when a mapping or reference is sufficient, and stale or contradictory knowledge. Do not store entire source-code implementations in AIMT; map and describe knowledge and point to source targets.

## 12. Hierarchy vs references

Hierarchy `@aimt → @map → @domain → @region → @node → @file → @frame` is structural. References and relations may connect entities across different branches or levels when meaningful. Do not force every relationship into the parent hierarchy. Do not duplicate an entity simply because multiple entities need to reference it.

## 13. Canonical `.aimt` source

`.aimt` is the canonical project knowledge source. `.agents/aimt/*` are instructions for the AI agent, not project knowledge. The source project is evidence. The `.aimt` package contains the structured project knowledge. Do not introduce a separate AIMT development or project directory.

## 14. Operating modes

AIMT can operate in three modes:

- **Initial mapping** — `/aimt .` creates the initial project knowledge map from the actual project.
- **Incremental update** — `/aimt update` maintains an existing `.aimt` map without rebuilding it unnecessarily.
- **Normal project work** — the AI uses AIMT as a background knowledge layer: read relevant knowledge when useful, navigate to deeper context or source evidence when needed, and update AIMT only when the project knowledge actually changes.

`index.md` is the operating protocol that determines which mode applies. `core.md` defines the knowledge model; `mapping.md`, `update.md`, and `operations.md` define the corresponding procedures.

A normal session must not update AIMT merely because a conversation ended. AIMT is updated when a project change creates, removes, or changes knowledge represented by the map.

## 15. Runtime trust and access model

### Host

The Host is the application, agent environment, or runtime environment that uses AIMT. The Host is not an AIMT knowledge level and must not become one of the seven levels. The Host does not own project knowledge merely because it uses AIMT. The Host provides the environment from which AIMT operations are requested.

### Installed AIMT Engine

The AIMT Engine is the installed runtime implementation responsible for opening and operating on `.aimt`. The Engine is the execution mechanism for AIMT operations, not project knowledge. Users of AIMT must not need access to the AIMT development repository or development project. The installed Engine must operate from the distributed AIMT package or runtime available to the Host. Do not introduce an AIMT development directory or project into the user's project model.

### Abstract Engine operation contract

The installed AIMT Engine provides an abstract operation surface. These are conceptual AIMT operations, not a transport requirement. Do not prescribe REST, HTTP, MCP, CLI, sockets, JSON-RPC, or another transport in `core.md`.

```
OPEN(path, mode)
READ(id)
SEARCH(query)
FOLLOW(reference)
CREATE(entity)
UPDATE(id, patch)
DELETE(id)
CREATE_RELATION(relation)
UPDATE_RELATION(relation)
DELETE_RELATION(relation)
VALIDATE()
PERSIST()
CLOSE()
```

- `OPEN` establishes access to an `.aimt` package.
- `READ` retrieves known knowledge by stable identity.
- `SEARCH` is optional discovery, not a mandatory retrieval architecture.
- `FOLLOW` navigates hierarchy, references, and relations.
- `CREATE` creates genuinely new knowledge.
- `UPDATE` modifies existing knowledge while preserving identity when appropriate.
- `DELETE` removes obsolete knowledge.
- relation operations modify supported relation structures.
- `VALIDATE` checks the resulting knowledge state.
- `PERSIST` commits validated changes.
- `CLOSE` ends the current Engine access lifecycle.

The Engine returns either the requested result or a structured error. Do not define a specific programming-language API.

### CRUD semantics

- **CREATE** — creates genuinely new knowledge, requires a new stable `id`, establishes valid hierarchy and ownership, uses applicable fields from the exact 18-field vocabulary, must be grounded in evidence when project knowledge requires evidence, must not create duplicate conceptual entities.
- **READ** — retrieves existing knowledge without modifying it, `READ(id)` is deterministic direct retrieval, READ does not imply WRITE permission.
- **UPDATE** — modifies an existing entity, preserves `id` when conceptual identity remains unchanged, is patch-based conceptually where only changed or applicable fields need modification, must not silently replace an entity with a new identity, must maintain valid hierarchy, relations, evidence, and traceability.
- **DELETE** — removes obsolete knowledge, must remove or repair affected references and relations so no dangling knowledge remains, must not silently delete knowledge merely because source text changed, deletion must represent actual conceptual removal.

Do not introduce CRUD as new AIMT fields or levels.

### Relation operations

Relations remain nested supported structures and are not a 19th field.

Conceptual operations:

```
CREATE_RELATION
UPDATE_RELATION
DELETE_RELATION
```

Relations use applicable existing fields, especially `type`, `from`, `to`, `evidence`. Conceptual discovery:

```
FOLLOW_RELATION
LIST_RELATIONS
```

Relations must reference valid entity IDs, have meaningful relationship types, have evidence when the relationship is source-backed and evidence is applicable, not create duplicate or speculative relationships, not leave dangling `from` or `to` references, and not be used to replace hierarchy where hierarchy is the correct ownership mechanism. Do not create a fixed closed list of relation types unless the existing AIMT contract already defines one. Treat relation `type` as a validated open vocabulary.

### Field applicability

Keep exactly 18 fields. Applicable means useful and level-appropriate, not mandatory for every entity:

```
id/title/description/summary → entity-level fields as applicable
parent → hierarchy placement
path → primarily @file source location
target → primarily @frame source target
location → precise source anchor
hash → source and change tracking
from/to → relation endpoints only
evidence → support and traceability
version/created/updated → lifecycle and change tracking
```

Optional does not mean ignored. Applicable fields should be populated when useful. Meaningless placeholders are prohibited. `from` and `to` are not ordinary entity fields. `path` and `target` have source-level semantics and should not be arbitrarily copied onto unrelated levels. Do not invent required fields beyond what is already established.

### ID guarantees

- `id` is globally unique within the `.aimt` knowledge map.
- `id` is stable and opaque.
- Preserve an existing `id` across rename or move when conceptual identity remains the same.
- Create a new `id` only for genuinely new conceptual knowledge.
- Never silently reuse an existing ID for a different concept.

Do not prescribe a generation algorithm.

### Evidence contract

Evidence must be verifiable. Conceptual examples that can be verified by the source or runtime:

```
path
path:line
path:line-range
URL
```

Only use formats that can actually be verified. Evidence supports mapped knowledge. Stale evidence must be repaired or removed. When an entity's source location changes, affected `path`, `location`, `hash`, and `evidence` should be updated together. Evidence is not credentials. Evidence must never contain private secrets. Do not prescribe a particular parser or storage format for evidence.

### Query and retrieval

Do not create a separate search, index, or retrieval architecture.

```
READ(id) → deterministic direct retrieval
SEARCH(query) → optional discovery operation
```

For `SEARCH`, implementation-specific matching or indexing is allowed, but AIMT behavior must remain deterministic enough for the Host to understand query input, matching scope, result entities, empty result, and failure. Do not require semantic search, vector search, embeddings, or an index. Progressive navigation remains the primary retrieval model.

### Host ↔ Engine contract

```
Host
  ↓
requests AIMT operation
  ↓
Installed AIMT Engine
  ↓
.aimt
  ↓
result/error
  ↓
Host
```

The Host provides the `.aimt` path and requested access or mode. The Engine verifies compatibility, opens the package, executes the requested operation, enforces access capability, validates mutations, persists only valid state, and returns a result or structured error. The Host must not simulate AIMT behavior when the installed Engine is unavailable. The Host must not depend on the AIMT development repository.

### Access enforcement

```
READ capability → READ, SEARCH, FOLLOW operations
WRITE capability → CREATE, UPDATE, DELETE, relation mutation, PERSIST
```

A READ-only Engine or Host receiving a mutation request must reject it. The rejection must happen before mutation or persistence. Do not assume that because the Host can read `.aimt`, it can modify `.aimt`.

### Authentication and authorization mapping

Authentication verifies the requester or credential. Authorization determines the permitted operation. Keep them separate.

- Ordinary READ may be unauthenticated unless the package or runtime protects it.
- Protected WRITE operations require appropriate authorization.
- `/aimt .` may require WRITE authorization when creating a protected `.aimt`.
- `/aimt update` requires WRITE authorization when modifying a protected `.aimt`.
- Normal project-work WRITE requires WRITE authorization when applicable.
- Failed authentication or authorization must reject the protected operation.
- Credentials and private keys never belong in AIMT knowledge.

Do not define cryptographic algorithms, key formats, or secret storage in the prompt files.

### Version compatibility

Engine and `.aimt` package must be compatible. Incompatible versions must produce a clear compatibility error. Do not silently downgrade or reinterpret unknown knowledge. The `version` field remains an AIMT knowledge and version field and is not an Engine credential.

### Validation and persistence contract

```
mutation
  ↓
VALIDATE
  ↓
if invalid → do not PERSIST
  ↓
if valid
  ↓
PERSIST
```

`VALIDATE` should conceptually return structured failures containing enough information to identify entity `id`, field or relation when applicable, and reason. Do not prescribe a programming-language exception type. Persistence must not leave a partially written canonical `.aimt` state. Do not prescribe a specific filesystem algorithm unless already defined by the Engine implementation.

### Runtime error categories

Conceptual common error categories. Do not require these to be literal API enum names if the runtime uses another representation; they are conceptual categories.

```
NOT_FOUND
INVALID
AUTH_REQUIRED
PERMISSION_DENIED
VALIDATION_FAILED
VERSION_INCOMPATIBLE
ENGINE_UNAVAILABLE
PERSIST_FAILED
```

Errors are returned to the Host. Failed operations must not be reported as successful. Errors must not expose private credentials or secrets. Mutation failures must not silently produce partial knowledge.

### Access modes, authentication, authorization, and ownership (consolidated)

Access is capability-based. READ-only access must not imply WRITE permission. A Host or AI agent should receive only the access capability required for the requested operation. Do not imply that authentication is automatically required for every READ operation. READ and WRITE permissions must be independently enforceable. WRITE permission must never be assumed merely because READ access succeeded. Owner or protected operations may require stronger authorization than ordinary READ access. Do not make a private or owner key an AIMT field. Do not add a 19th field. If ownership credentials are part of the AIMT package or runtime, describe them conceptually as access or security material, separate from project knowledge. Never treat credentials, private keys, or secrets as project knowledge or evidence. Do not prescribe where secrets are stored unless that behavior is already defined elsewhere in the AIMT runtime contract.

### Relationship to operating modes

- `/aimt .` uses the installed AIMT Engine to create and persist the initial map.
- `/aimt update` uses the installed AIMT Engine to read, modify, validate, and persist the existing map.
- Normal project work uses the installed AIMT Engine according to `operations.md`.
- `mapping.md`, `update.md`, and `operations.md` define procedures; `core.md` only defines the conceptual contract and access model.

### Security and trust boundaries

- `.aimt` is canonical project knowledge, but it is not itself a credential.
- Source project files are evidence, not credentials.
- `.agents/aimt/*` are instructions, not credentials or project knowledge.
- Authentication must not be confused with evidence or entity identity.
- Authorization must not change the meaning of AIMT levels or fields.
- Never expose or copy private credentials into AIMT entities, fields, evidence, relations, or source mappings.
