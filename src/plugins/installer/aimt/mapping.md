# AIMT Mapping — `/aimt .`

Trigger: `/aimt .`

## Architecture

```
source project
      ↓ evidence
  /aimt .
      ↓
    .aimt   (canonical knowledge source of truth)
```

- `.agents/aimt/*` contains instructions only.
- Source project files are evidence.
- `.aimt` is the canonical AIMT knowledge source.
- `/aimt .` performs the initial complete mapping.
- This is not an AIMT development or repository discovery task.
- The AI must map the user's actual project, not the AIMT implementation project.

## Steps

### Step 1 — Discover the project

Determine what the actual project contains before creating knowledge.

Inspect:

- project root
- relevant directories
- source files
- configuration files
- dependency and package manifests
- build and runtime configuration
- documentation that explains actual project behavior
- relevant generated or configuration files when they provide architectural evidence

Identify:

- project identity
- technology, framework, and language where relevant
- major project areas
- important entry points
- meaningful source boundaries
- files that are relevant to understanding the project

Ignore irrelevant artifacts such as temporary files, caches, build outputs, dependency installations, and generated artifacts unless they are necessary evidence for project behavior.

Do not create AIMT entities yet merely because a file or directory exists. Do not inspect or depend on the AIMT development repository. Do not assume the user has an AIMT source or development project. Do not map `.agents/aimt/*` as project knowledge or map the `.aimt` instructions themselves as application architecture.

**Result:** a grounded understanding of the project's actual structure and relevant evidence sources.

### Step 2 — Understand the project

Analyze the discovered project to understand what it actually does and how its parts work together.

Identify meaningful:

- architecture
- domains
- regions
- nodes
- files
- frames
- responsibilities
- dependencies
- data and control flow
- important boundaries
- project-level concepts

Use source code and relevant project documentation as evidence. Do not infer undocumented behavior when evidence is unavailable. Do not blindly map every file and do not treat every directory, file, class, or function as automatically deserving an AIMT entity. Capture knowledge that is meaningful for understanding, navigation, maintenance, and future AI reasoning. Do not invent project knowledge.

**Result:** a conceptual model of the project grounded in source evidence.

### Step 3 — Build the 7-level knowledge map

Construct the initial AIMT hierarchy using only the levels that are meaningful for this project:

```
@aimt
  → @map
    → @domain
      → @region
        → @node
          → @file
            → @frame
```

Use all applicable levels when the project actually supports them.

Do not force every level into every branch, create empty or artificial entities, create hierarchy only to satisfy the seven-level model, or use `@relation` as a level. Do not create artificial seven-level hierarchy. Do not invent AIMT levels.

Create ownership relationships so that each meaningful fact has a clear owner.

**Result:** an initial hierarchical knowledge structure representing the project's meaningful concepts and source boundaries.

### Step 4 — Establish ownership, identity, and fields

For every created entity:

- assign a stable `id`
- provide an appropriate `title`
- capture `description` and/or `summary` where useful
- establish `parent` where hierarchy applies
- assign `type` where applicable
- record `source` where provenance or origin matters
- record `context` where surrounding meaning matters
- record `path` for physical files
- record `target` for source targets and frames where applicable
- record `location` for precise source anchors where available
- record `hash` for source-backed change detection where applicable
- record `version`
- record `created`
- record `updated`

Use the complete 18-field vocabulary defined by `core.md`:

```
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

Use all fields that are applicable and useful.

Do not populate a field with meaningless placeholder data merely to make every entity contain all 18 fields. Do not invent AIMT fields. Do not treat relations as a 19th field.

Remember:

```
optional ≠ ignore
```

Optional fields should be captured whenever they improve context, ownership, relationships, evidence, traceability, source location, or change tracking.

**Result:** every entity has clear identity, ownership, context, and traceability.

### Step 5 — Connect relations and evidence

Connect meaningful knowledge that cannot be represented by hierarchy alone.

Relations must use the supported AIMT relation structure and applicable fields such as:

```
type
from
to
evidence
```

Create relations for meaningful connections such as:

- dependency
- implementation
- ownership
- composition
- usage
- data and control flow
- architectural connection
- other relationships supported by the actual project evidence

Do not create speculative or decorative relations. Do not create relations without evidence when evidence can be captured. Every important source-backed relation should have evidence whenever evidence can be captured. Do not invent relation fields. Do not replace evidence with assumptions.

Maintain:

```
One fact, one owner, many references.
```

Do not duplicate a fact simply because multiple entities depend on it. Reference the fact through relations or appropriate hierarchy instead. Do not create duplicate ownership for the same fact.

**Result:** the knowledge map becomes connected rather than merely hierarchical.

### Step 6 — Create and persist `.aimt`

Create or open the target `.aimt` package using the supported AIMT runtime or package mechanism.

Build the initial map inside `.aimt`.

Ensure:

- the map represents the actual project
- entities have stable identities
- hierarchy is meaningful
- relations are meaningful
- source-backed evidence is captured
- applicable fields are populated
- `.aimt` is treated as the canonical AIMT knowledge source

Do not create:

```
project.aimt/
aimt-project/
AIMT development project/
```

or any other separate AIMT project or development directory. The user's `.aimt` package is the project knowledge artifact. Do not map the user's actual project as the AIMT implementation project.

**Result:** the complete initial AIMT map exists in the project's `.aimt` package.

### Step 7 — Validate and verify

Before considering `/aimt .` complete, validate the resulting knowledge map.

Verify:

- `.aimt` is readable by the AIMT runtime
- the hierarchy is valid
- entity IDs are stable and unique
- parent references are valid
- relation `from` and `to` references are valid
- required and applicable fields are correctly represented
- evidence points to real project sources
- source paths and locations are valid where recorded
- hashes are correct where used
- timestamps and version information is coherent
- no unsupported levels or fields were introduced
- no artificial hierarchy was created
- no unnecessary duplicate facts were created
- important relationships are represented
- the map is sufficiently connected to support progressive AI navigation

If validation finds an issue:

```
identify → correct → validate again
```

Do not declare `/aimt .` complete until the resulting `.aimt` is valid and persisted. Do not finish before validation and persistence succeed.

**Result:** a validated, persisted, project-grounded canonical `.aimt` knowledge map.

## Completion

`/aimt .` is complete only when the project's initial knowledge map has been discovered, understood, structured, connected with evidence, persisted to `.aimt`, validated, and verified.

The operation must produce a usable knowledge map, not merely a list of files.

The resulting `.aimt` is the canonical project knowledge source for subsequent AIMT-aware operations.
