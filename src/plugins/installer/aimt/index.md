# AIMT Operating Protocol

## Purpose

AIMT is the project's structured knowledge layer.

AIMT organizes knowledge; AI provides intelligence.

`.aimt` is the canonical project knowledge source.

`index.md` is the operating protocol and router.

`core.md` defines the AIMT knowledge model.

The other files define procedures.

The AI must not depend on the AIMT development repository.

## Operating Modes

### Initial mapping

```text
/aimt .
```

Purpose: create the initial complete AIMT knowledge map for the actual project.

Instruction source:

```text
mapping.md
```

### Incremental update

```text
/aimt update
```

Purpose: maintain an existing `.aimt` knowledge map when project knowledge changes.

Instruction source:

```text
update.md
```

### Normal project work

This is the default mode when the user is doing ordinary project work and has not explicitly invoked `/aimt .` or `/aimt update`.

Instruction source:

```text
operations.md
```

Normal project work must treat AIMT as a background knowledge layer rather than requiring the user to explicitly invoke AIMT for every task.

## Mode Routing

```text
User explicitly invokes /aimt .
    → read core.md
    → read mapping.md
    → execute initial mapping

User explicitly invokes /aimt update
    → read core.md
    → read update.md
    → execute incremental update

Normal project request
    → determine whether AIMT is relevant
    → if relevant and .aimt exists, read core.md + operations.md
    → use AIMT according to the normal-operation lifecycle
    → perform the requested project work
    → determine whether project knowledge changed
    → update AIMT only when affected knowledge actually changed
```

If the task is unrelated to project knowledge, do not force AIMT usage.

## Normal Project Work

Before changing project code or configuration:

- If `.aimt` exists and the task is relevant to mapped project knowledge, READ the relevant AIMT knowledge first.
- Navigate progressively; do not load the entire `.aimt` unnecessarily.
- Follow relations and source evidence when the current knowledge is insufficient.
- If AIMT knowledge is missing, stale, or contradicted by source evidence, inspect the source project and use the source as evidence.
- Do not invent project knowledge.

After project work:

- Determine whether the change affected knowledge represented by `.aimt`.
- If no mapped knowledge changed, do not modify `.aimt`.
- If mapped knowledge changed, update only the affected knowledge.
- Preserve stable identities whenever the conceptual entity remains the same.
- Repair affected relations, evidence, locations, hashes, versions, timestamps, or other applicable fields.
- Validate the affected knowledge before persisting.
- Persist the updated `.aimt`.

Do not update AIMT merely because a chat or task ended. Update AIMT when the actual project knowledge represented by AIMT has changed.

## Instruction Files

| File | Responsibility |
|---|---|
| `index.md` | AIMT operating protocol and mode routing |
| `core.md` | AIMT knowledge model and conceptual contract |
| `mapping.md` | Initial complete mapping for `/aimt .` |
| `operations.md` | Normal AIMT-aware project operations |
| `update.md` | Incremental maintenance for `/aimt update` |

Read only the instruction files relevant to the current mode and task rather than blindly loading every file.

## Operating Rules

- `.aimt` is the canonical AIMT knowledge source.
- Source project files are evidence, not a replacement for AIMT knowledge.
- `.agents/aimt/*` contains instructions, not project knowledge.
- One fact, one owner, many references.
- Do not invent AIMT levels, fields, or relation fields.
- Use all applicable fields when creating or updating knowledge; do not populate fields with meaningless values merely for completeness.
- Do not create artificial hierarchy.
- Preserve stable IDs unless the entity's identity has genuinely changed.
- Prefer source evidence over assumptions.
- Do not rebuild the entire map for an incremental change.
- Do not write AIMT for unrelated tasks.
- Do not update AIMT merely because a conversation ended.
- Validate before persisting changes.
