# AIMT Update — `/aimt update`

Trigger: `/aimt update`

## Architecture

```text
existing .aimt
      +
changed project source
      ↓
affected knowledge
      ↓
updated .aimt
      ↓
validated canonical knowledge
```

- `.aimt` is the canonical AIMT knowledge source.
- Source project files are evidence.
- `.agents/aimt/*` contains instructions only.
- `/aimt update` is an incremental maintenance operation.
- The AIMT development repository must never be required.
- Existing knowledge must be preserved whenever it remains valid.

## Steps

### Step 1 — Open and understand the existing map

Open the existing `.aimt` using the supported AIMT runtime or package mechanism.

Read enough of the existing map to understand:

- project identity
- existing hierarchy
- existing entity identities
- existing ownership
- existing relations
- existing source and evidence references
- existing source paths and locations
- relevant versions, hashes, and timestamps

Do not immediately rewrite anything.

Establish the existing map as the baseline for the update.

If `.aimt` cannot be opened or is invalid, do not silently perform an incremental update against unknown state. Follow the supported failure behavior and do not claim the update succeeded.

**Result:** a known, usable baseline `.aimt` map.

### Step 2 — Inspect the actual project for changes

Inspect the current project against the knowledge represented by the existing `.aimt`.

Look for relevant:

- new files
- removed files
- changed files
- new frames and symbols
- removed frames and symbols
- changed responsibilities
- changed architecture
- changed dependencies
- changed data and control flow
- changed project concepts
- changed relations
- changed source locations
- other source-backed knowledge changes

Use source files as evidence.

Do not assume that every changed file means AIMT knowledge changed.

Do not map temporary files, caches, build outputs, dependency installations, or irrelevant generated artifacts unless they provide meaningful project evidence.

**Result:** a grounded set of candidate project changes.

### Step 3 — Determine the knowledge impact

For each candidate change, determine whether it affects existing AIMT knowledge.

Classify each affected entity or relationship as:

```text
unchanged
changed
new
deleted
```

Use conceptual identity, not only filenames or textual differences.

Examples:

- Source content changed but the same conceptual entity remains → `changed`.
- A function was renamed but represents the same conceptual responsibility → preserve its identity and update its knowledge.
- A genuinely new component or concept appears → `new`.
- A source entity no longer exists → `deleted`.
- A source file changed internally but no represented knowledge changed → `unchanged`.

Do not modify `.aimt` for changes that have no knowledge impact.

**Result:** an explicit set of knowledge changes that must be applied.

### Step 4 — Preserve identity and ownership

Apply identity decisions before writing changes.

Rules:

- Preserve the existing `id` when the conceptual entity remains the same.
- Never generate a new ID merely because source content, path, location, or implementation changed.
- Create a new ID only for a genuinely new conceptual entity.
- Remove obsolete knowledge only when the corresponding project concept or entity is actually gone.
- Do not transfer ownership merely because another entity references the same fact.
- Maintain `One fact, one owner, many references.`
- Do not create duplicate entities to represent the same fact.

If an entity was renamed or moved but remains conceptually the same, preserve its identity and update the appropriate knowledge fields.

If an entity is replaced by a genuinely different concept, treat the old and new identities separately according to the supported AIMT semantics.

**Result:** stable identities and correct ownership decisions for the update.

### Step 5 — Apply affected knowledge, relations, and evidence

Update only the affected portions of `.aimt`.

For changed entities, update applicable information such as:

```text
title
description
summary
parent
type
source
context
path
target
location
hash
evidence
version
created
updated
```

Use the complete 18-field vocabulary defined by `core.md`:

```text
id
title
description
summary
parent
type
source
context
path
target
location
hash
from
to
evidence
version
created
updated
```

Use all applicable fields.

Do not fill fields with meaningless placeholder values merely for completeness.

For new entities:

- create stable IDs
- establish correct ownership
- establish appropriate hierarchy
- capture applicable fields
- capture source evidence

For deleted entities:

- remove or otherwise handle stale knowledge according to the supported AIMT semantics
- do not leave knowledge that falsely represents a deleted project entity

For relations:

- add new meaningful relations
- update relations affected by changed entities
- remove stale relations caused by deleted entities
- preserve unaffected relations
- use supported relation structure with applicable fields such as `type`, `from`, `to`, and `evidence`

For evidence:

- update evidence when the supporting source changed
- remove stale evidence when its source no longer supports the claim
- do not replace missing evidence with assumptions

**Result:** only affected knowledge, relations, and evidence are synchronized.

### Step 6 — Validate the incremental update

Validate the updated knowledge before considering it complete.

Verify:

- existing stable IDs were preserved where identity did not change
- new IDs are unique
- deleted entities no longer leave invalid references
- parent references remain valid
- relation `from` and `to` references remain valid
- affected relations are consistent
- evidence points to real project sources
- paths and locations are valid where recorded
- hashes are correct where used
- versions are coherent
- `created` and `updated` timestamps are coherent
- applicable fields are represented correctly
- no unsupported levels or fields were introduced
- no duplicate ownership was introduced
- no artificial hierarchy was created
- unchanged knowledge was not unnecessarily rewritten
- `.aimt` remains readable by the AIMT runtime

If validation fails:

```text
identify → correct → validate again
```

Do not persist an invalid update.

**Result:** a validated incremental knowledge state.

### Step 7 — Persist and verify

Persist the validated changes to the canonical `.aimt`.

After persistence, verify that:

- `.aimt` can be opened again
- updated entities are present
- deleted knowledge is no longer incorrectly represented
- relations resolve correctly
- evidence remains traceable
- the resulting map reflects the actual current project state
- unaffected knowledge remains intact

Do not report success until persistence and post-persistence verification succeed.

**Result:** a verified `.aimt` containing the current project knowledge.

## Completion

`/aimt update` is complete only when the existing `.aimt` has been compared with the current project, knowledge impact has been determined, affected knowledge has been incrementally synchronized, identities and ownership have been preserved, relations and evidence have been repaired where necessary, validation has succeeded, and the updated `.aimt` has been persisted and verified.

An incremental update must preserve valid existing knowledge and must not rebuild unrelated portions of the map.

## Distinction from `/aimt .`

```text
/aimt .
→ initial complete mapping
→ discovers and builds the map

/aimt update
→ incremental maintenance
→ starts from existing knowledge
→ changes only what project changes require
```

`/aimt update` must not behave like `/aimt .`.

Do not rediscover and rebuild the entire project map when only a small portion changed.

## Edge Cases

### No project changes

If inspection finds no project changes affecting AIMT knowledge:

- do not rewrite the map unnecessarily
- do not create new IDs
- do not change unrelated timestamps or versions
- report that no AIMT knowledge update was required

### Source changed but knowledge did not

If source files changed but the represented AIMT knowledge remains semantically unchanged:

- preserve the existing entity
- update source-tracking fields only when applicable
- do not invent conceptual changes

### Entity renamed or moved

If the conceptual entity remains the same:

- preserve its ID
- update applicable `title`, `path`, `target`, `location`, `hash`, `version`, `updated`, or other applicable fields
- repair affected relations and evidence

### Entity deleted

If the source entity is genuinely deleted:

- remove or handle its stale knowledge according to supported AIMT semantics
- repair or remove affected relations
- remove stale evidence
- do not leave references to nonexistent knowledge

### New entity

Create a new stable identity and establish its ownership, hierarchy, evidence, and meaningful relations.

### Stale or contradictory AIMT knowledge

When existing AIMT knowledge conflicts with current source evidence:

- inspect the source
- treat source evidence as the basis for correcting the knowledge
- preserve identity when the conceptual entity remains the same
- update only the affected knowledge
- validate again
