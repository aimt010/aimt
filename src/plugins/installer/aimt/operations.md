# AIMT Operations

How an AI interacts with AIMT during normal project work.

- `index.md` decides that normal project work is the current operating mode.
- `operations.md` defines how AIMT is used during that work.
- `.aimt` is the canonical AIMT knowledge source.
- Source project files are evidence.
- `.agents/aimt/*` contains instructions, not project knowledge.
- AIMT usage must be conditional on task relevance.

Do not redefine the seven levels or 18 fields. `core.md` owns those definitions.

## Steps

### Step 1 — Determine AIMT relevance

Before using AIMT, determine whether the current user request can benefit from project knowledge represented in `.aimt`.

If the task is unrelated to the project knowledge map:

- do not force AIMT usage
- do not open `.aimt`
- perform the task normally

If the task concerns project architecture, source structure, existing functionality, dependencies, relationships, responsibilities, or another concept represented by AIMT:

- use AIMT as a knowledge layer

If `.aimt` does not exist:

- do not invent AIMT knowledge
- perform the requested work using the actual project as evidence
- do not claim that AIMT was read

**Result:** a decision about whether AIMT is relevant to the current task.

### Step 2 — OPEN the AIMT package

When AIMT is relevant, open the project's existing `.aimt` package using the supported AIMT runtime or package mechanism.

Do not inspect or depend on the AIMT development repository.

Do not treat `.agents/aimt/*` as project knowledge.

If `.aimt` cannot be opened:

- do not fabricate knowledge
- use the source project as evidence where necessary
- do not claim successful AIMT access

**Result:** the canonical AIMT knowledge source is available for the current operation.

### Step 3 — READ relevant knowledge

Read only the AIMT knowledge relevant to the current task.

Start with the smallest useful scope.

Use:

- `READ(id)` — deterministic direct retrieval when the entity is known
- `SEARCH(query)` — optional discovery when the entity is not known
- `FOLLOW(reference)` — navigate hierarchy and references
- `FOLLOW_RELATION(...)` — navigate relation structures via `from` and `to`

to locate relevant knowledge. Use direct `READ` when the entity is known. Use `SEARCH` only when discovery is needed. Do not require search before every `READ`.

`READ` is the fundamental access path. Search is optional and must not become a mandatory retrieval layer. Progressive navigation remains the primary way to explore AIMT knowledge. Do not introduce a separate search, index, or retrieval architecture.

Use progressive disclosure and stop when sufficient context is available.

Do not unnecessarily read the entire `.aimt` package.

**Result:** enough existing AIMT knowledge is loaded to understand the task.

### Step 4 — Follow knowledge and source evidence

Progressively navigate from the relevant knowledge to deeper context when needed.

Follow:

```text
parent
↓
child knowledge
↓
relations
↓
source evidence
↓
deeper entities
```

Use source project files when:

- AIMT knowledge is missing
- AIMT knowledge is insufficient
- AIMT knowledge is stale
- AIMT knowledge conflicts with the current source
- the task requires details not represented in AIMT

Treat actual source evidence as the basis for correcting inaccurate knowledge.

Do not replace evidence with assumptions.

Do not traverse unrelated portions of the map merely for completeness.

**Result:** the AI has sufficient contextual and source-backed knowledge to perform the requested work.

### Step 5 — Perform the requested project work

Use the gathered AIMT context and source evidence to perform the user's actual task.

Examples include:

- answering questions
- explaining existing architecture
- modifying source code
- adding functionality
- removing functionality
- renaming or moving source entities
- changing configuration
- changing dependencies
- restructuring project architecture

AIMT does not replace the source project.

AIMT provides structured context for reasoning about the project.

**Result:** the requested project work is completed or the requested information is provided.

### Step 6 — Determine whether AIMT knowledge changed

After project work, determine whether the actual project knowledge represented by `.aimt` changed.

Classify the result as:

```text
no knowledge change
knowledge changed
```

If there is **no knowledge change**:

- do not write AIMT
- do not modify unrelated entities
- do not update timestamps merely because the task ended
- proceed to close

If knowledge **did change**:

- identify only the affected entities and relations
- preserve existing IDs when conceptual identity remains unchanged
- create new IDs only for genuinely new entities
- remove or repair stale knowledge caused by deletion
- update affected source and evidence information
- update affected relations
- update applicable fields
- continue to WRITE

Examples:

```text
"Explain authentication"
→ READ
→ answer
→ no WRITE

"Add OAuth authentication"
→ READ
→ modify project
→ authentication knowledge changed
→ WRITE affected knowledge

"Rename an existing function"
→ READ
→ modify source
→ preserve the frame identity when conceptually unchanged
→ update affected target/location/hash/evidence/etc.
→ WRITE

"Fix a typo in an unrelated comment"
→ perform change
→ no AIMT knowledge change
→ no WRITE
```

Explicitly enforce:

> Do not update AIMT merely because a conversation or task ended. Update AIMT only when actual project knowledge represented by AIMT has changed.

**Result:** a deterministic decision about whether AIMT must be modified.

### Step 7 — WRITE, VALIDATE, and CLOSE when required

When knowledge changed, determine the required mutation:

```text
CREATE
UPDATE
DELETE
CREATE_RELATION
UPDATE_RELATION
DELETE_RELATION
```

Then:

1. verify WRITE capability
2. perform the minimal affected mutation
3. preserve existing IDs where conceptual identity remains unchanged
4. repair affected relations and evidence
5. validate
6. correct validation failures
7. validate again
8. persist
9. close
10. verify persisted state

For affected entities, use all applicable fields from the exact 18-field vocabulary defined by `core.md`:

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

Do not populate fields with meaningless placeholder values.

For affected relations, preserve meaningful relation structures using applicable fields such as:

```text
type
from
to
evidence
```

Maintain:

```text
One fact, one owner, many references.
```

If the Host or Engine is READ-only:

- do not attempt mutation
- do not simulate success
- report the permission failure

If no AIMT knowledge changed:

- do not WRITE
- close the opened `.aimt` without unnecessary modification

Do not persist invalid knowledge. Do not create duplicate ownership. Do not create artificial hierarchy. Do not invent AIMT levels or fields. Do not treat relations as a 19th field. Do not create speculative relations.

**Result:** the AIMT state is either unchanged or safely synchronized with the project change.

### Error handling

If:

```text
OPEN
READ
SEARCH
CREATE
UPDATE
DELETE
VALIDATE
PERSIST
CLOSE
```

fails, stop the affected operation and report the actual failure.

Never claim that AIMT was updated unless persistence and verification succeeded.

## Normal Operation Lifecycle

```text
Determine relevance
      ↓
OPEN
      ↓
READ relevant knowledge
      ↓
follow/navigation
      ↓
READ deeper / inspect source evidence when needed
      ↓
perform requested project work
      ↓
did project knowledge change?
      ├── no → CLOSE
      │
      └── yes
            ↓
          WRITE affected knowledge
            ↓
          VALIDATE
            ↓
          PERSIST
            ↓
          CLOSE
            ↓
          VERIFY
```

This lifecycle is a summary of the seven steps above; do not introduce a second different workflow.

## Stale or Missing Knowledge

AIMT knowledge is a structured representation of the project and may become stale when the source project changes.

When AIMT knowledge is missing, stale, or contradicted by source evidence:

1. inspect the relevant source project
2. establish the current state from source evidence
3. preserve the existing entity identity when the conceptual entity remains the same
4. correct only the affected AIMT knowledge
5. repair affected relations and evidence
6. validate before persisting

Never invent missing project knowledge merely to complete the AIMT map.
