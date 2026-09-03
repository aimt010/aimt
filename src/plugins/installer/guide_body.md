This project uses AIMT (AI Mapping Taxonomy) — a small knowledge-mapping format and runtime that makes project knowledge understandable to both humans and AI.

Core principle: AIMT organizes knowledge; AI provides intelligence.

### What AIMT is
- AIMT maps project knowledge into levels: `@aimt`, `@map`, `@domain`, `@region`, `@node`, `@file`, `@frame` (and `@relation` inside `relations`).
- One `.pmap` per entity; a `.aimt` package contains multiple logical `.pmap` entries.
- AIMT is deterministic, human-readable, and AI-readable; it does not duplicate source bytes.

### Where the .aimt file is
- Look for `*.aimt` in the project root (e.g., `aimt-test-project.aimt`) or a directory containing `.aimt`.
- Use `aimt view <path>` to explore or `Store::open` via workflows.
- If no `.aimt` exists, the agent may offer `aimt init <path>`.

### When AIMT should be used
- Before reading source files, answering architecture questions, or searching with grep/glob.
- For any question about project structure, components, or how to add/modify/find code — AIMT is the primary map.
- After modifying code, consider whether AIMT knowledge should be updated (hash changed, new node/file).

### How to discover AIMT knowledge
- Discover: glob `**/*.aimt` or use `Store::open(path)` / `ReadWorkflow::open`.
- Search: `aimt aimt search --aimt <path> --query "<q>"` or `ReadWorkflow::search(&store, query)` — prefer semantic search over grep.
- Read: `aimt aimt read --aimt <path> --id <id>` or `Store::get(id)`.

### How to search / read / follow relationships
- `search` by id/title/level substring; then `read` the selected entity.
- `follow_parent`: follow `parent` field to traverse hierarchy (`@region`/`@node` parent chain).
- `follow_file`: for `@frame`, follow `file` field to its `@file`.
- `follow_relation`: follow `relations` → `@relation` `from`/`to`/`evidence`/`source` to trace dependencies.
- `validate`: `aimt aimt validate --aimt <path>` or `Store::validate()` — check required/forbidden fields and reference integrity.

### How to validate
- Run `validate` before persisting or before commit.
- Validation checks: required fields per level, forbidden fields, `id` shape `^[a-z][a-z0-9_]*$`, `from`/`to` inside `@relation`, and that `parent`/`file`/`from`/`to` point to existing ids with correct level.

### Authorization before writes (required)
- Understand AIMT → Use AIMT for project knowledge → Read/search normally → Before modification → Check authorization.
- AIMT read access is public: `discover`, `search`, `read`, `follow`, `validate` are always allowed, even when not authorized. Do not block reads.
- Before any write/CRUD (`insert`, `update`, `remove`, `persist`), check AIMT authorization.
- If not authorized: STOP the write. Do NOT attempt `insert`, `update`, `remove`, or `persist`. Do NOT try to bypass AIMT authorization. Do NOT ask the user for their private key. Do NOT request the private key through an AI or plugin prompt. Do NOT modify `owner_public_key`, `open_to_read`, or security metadata. Tell the user: `AIMT write access is not authorized on this environment. Please authorize AIMT first (for example, \`aimt login\`), then I can perform the requested change.`
- If authorized: check normal workflow permissions (`update_intent`, `HostedEngine` restrictions, validation). Authorization does not bypass workflow permissions. `HostedEngine` remains read-only even when authorized.
- `open_to_read` and `owner_public_key` changes alone never grant write access; cryptographic verification is required.

### How to perform updates when permitted
- Read-only by default. `Store::open` succeeds for search/read/validate.
- Update only when authorized and with explicit intent: `aimt status` shows `Logged in` → `Store::open_mut` → modify `AimtEntity` → `validate` → `persist`.
- The workflow engine enforces this via `WorkflowContext { dirty, update_intent }` and `permissions::permission_for`.
- Never auto-persist on hooks; ask the agent via `ShouldUpdate` decision before writing.
- If `aimt status` shows `Not logged in`, do not attempt CRUD — ask user to authorize first.

### How AIMT fits into the AI's development workflow
- Discover → Open → Search → Read → Follow hierarchy/files/relations → Determine if source needed → Answer/perform task → If knowledge changed, check authorization → if authorized, Update → Validate → Close; if not authorized, ask user to run `aimt login`.
- AIMT is consulted iteratively and in loops; the agent may search again, follow another relation, or inspect source only when necessary.
- After code changes, run validation and keep the map current.

### Hosted vs Install
- **Hosted (read-only)**: Published `.aimt` packages are portable and read-only. Any tool can `Store::open` for search/read/validate without installation. No `open_mut` or `persist` — `permissions::permission_for` enforces `ReadOnly`. `HostedEngine` remains read-only even when authorized.
- **Install (development)**: `aimt install <tool>` installs project integration (plugin + guide) for local development. Updates require authorization (`aimt login`) + explicit `update_intent` + `validate` before `persist`, never auto-persist on hooks.

Trigger: if this guide is installed via `aimt install <tool>`, the corresponding plugin/skill exposes the `aimt` workflow (e.g., OpenCode `AimPlugin` tool, Claude Code via `CLAUDE.md` guidance).
