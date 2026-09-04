<div align="center">

![AIMT](assets/images/logo-aimt.png)

**AIMT - AI Mapping Taxonomy**

**Building the Knowledge Brain of AI**

[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green)](LICENSE)

</div>

---

AIMT is a small knowledge-mapping format and runtime that makes project knowledge understandable to both humans and AI.

> AIMT organizes knowledge, AI provides intelligence.

AIMT stays small, deterministic, and human readable while keeping AI readable. Intelligence such as search, ranking, and planning stays outside AIMT. AIMT provides the structured knowledge that intelligence operates on.

## What is AIMT?

AIMT maps the real knowledge in your project, like domains, regions, concepts, files, symbols, and their relationships, into a single portable file called `.aimt`. Instead of understanding being scattered across source files, you get one canonical map that people and agents can discover, search, and follow.

## Why use AIMT?

* **For developers:** keep a stable map of what your project contains and where things live, without copying source code.
* **For AI agents:** give the agent accurate context before it reads source. The agent navigates the map first, then checks source evidence only when needed.
* **For teams:** share the same `.aimt` file with the repository or release so everyone sees the same structure.

## What does AIMT provide?

* One `.aimt` artifact per project (many logical entries in one file)
* Seven fixed levels: `@aimt` → `@map` → `@domain` → `@region` → `@node` → `@file` → `@frame` (`@relation` is nested relation data, not a level)
* A 20 field vocabulary for identity, hierarchy, and traceability
* A CLI to create, read, search, follow, validate, and manage `.aimt` knowledge
* Secure owner write access with operating system credential storage
* Guidance for AI agents via `aimt install <tool>`

## How the `.aimt` file works

Your source code stays where it is. AIMT stores a mapping to it, not a copy.

* Each logical entry in `.aimt` represents one entity, one level and one `id`
* Hierarchy uses `parent` and `file` references, not file system layout
* Relations use `from` and `to` to connect entities across the hierarchy
* The same knowledge can be viewed as a directory of `.pmap` files during development or as a single `.aimt` file for distribution

```
my-project.aimt
  ├── @aimt             # project identity
  ├── @map              # top level map
  ├── @domain           # for example, payments
  ├── @region / @node   # concepts
  ├── @file             # path to src/...
  └── @frame            # symbol inside a file
```

AIMT is the canonical knowledge source, source files are evidence. If the map is missing, stale, or insufficient, inspect the source.

## Installation

### Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aimt010/aimt/releases/latest/download/install.sh | sh
```

### macOS

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aimt010/aimt/releases/latest/download/install.sh | sh
```

### Windows

```powershell
irm https://github.com/aimt010/aimt/releases/latest/download/install.ps1 | iex
```

## Quick start

**1. Install AIMT**

Install for your platform using the commands in [Installation](#installation), then verify:

```bash
aimt --version
aimt --help
```

**2. Install AIMT integration for your AI agent**

```bash
aimt install <platform>
# example
aimt install opencode
```

This installs the required prompts and instructions into your project so the AI agent can work with your `.aimt` knowledge. See [AI agent integration](#ai-agent-integration) for all supported agents and commands.

**3. Initialize AIMT**

In your terminal, run:

```bash
aimt init
```

This initializes AIMT in the current project and creates `my-project.aimt`. The private key is printed once to stderr and never written into `.aimt`, so keep it secret.

**4. Open the project with your AI agent**

Open the same project folder in OpenCode, Claude Code, Cursor, or your chosen supported agent.

**5. In the AI agent chat, type:**

```
/aimt .
```

This is not a terminal command. Type it inside the agent's chat input. It performs the first time project knowledge mapping and creates the initial `.aimt` content from your source evidence.

## AI agent integration

`aimt install <tool>` installs the prompts and instructions your agent needs to work with the project's `.aimt` knowledge.

| AI agent | Install command |
| --- | --- |
| OpenCode | `aimt install opencode` |
| Claude Code | `aimt install claude` |
| Codex | `aimt install codex` |
| Antigravity | `aimt install antigravity` |
| Kilo | `aimt install kilo` |
| GitHub Copilot | `aimt install copilot` |
| Aider | `aimt install aider` |
| Cursor | `aimt install cursor` |
| Gemini | `aimt install gemini` |
| Kimi | `aimt install kimi` |

For a specific project directory:

```bash
aimt install claude --path .
```

## Commands

All examples auto discover the project when a single `.aimt` exists, otherwise pass the path explicitly.

```bash
# Create and manage the package
aimt init my-project.aimt
aimt status my-project.aimt
aimt status

# Authenticate for writes (stores credential securely, prompts if --key omitted)
aimt login my-project.aimt
aimt login my-project.aimt --key <64-hex>
aimt logout my-project.aimt
aimt logout

# Read and explore, no login required
aimt search my-project.aimt --query payments
aimt search --query auth
aimt read my-project.aimt node_payments
aimt follow-parent my-project.aimt node_payments
aimt follow-file my-project.aimt frame_pay_provider
aimt follow-relation my-project.aimt node_payments
aimt validate my-project.aimt
aimt validate

# Visual explorer
aimt view my-project.aimt
aimt view --port 3000 --no-open

# Key lifecycle, requires login
aimt key backup my-project.aimt --output ~/.secrets/demo-backup.json
aimt key rotate my-project.aimt

# Installation lifecycle
aimt install opencode --path .
aimt upgrade --path . --target 0.2.0
aimt upgrade
aimt uninstall --path . --yes

# Help and version
aimt --help
aimt --version
```

Project discovery:

* explicit path always wins, error if missing
* 0 files: `No AIMT project found. Create one with: aimt init <name>`
* 1 file: used automatically
* 2 or more files: interactive selection or `Please specify a project path` when not interactive

### AIMT knowledge lifecycle

These are AI agent chat instructions, not terminal commands. They run via the guidance installed under `.agents/aimt/`.

* `/aimt .`: create the initial map from the actual project, establishing stable ids
* `/aimt update`: incrementally maintain an existing `.aimt`, preserving stable ids

Update the map only when project knowledge actually changes, not just because a conversation ended.

### Installation lifecycle

These are terminal commands that manage the AIMT software itself:

* `aimt install <tool> [--path <DIR>]`: install guidance for a supported agent
* `aimt upgrade [--path <DIR>] [--target <VERSION>]`: upgrade the AIMT engine and owned guidance
* `aimt uninstall [--path <DIR>] [--yes]`: remove the AIMT runtime and owned guidance

Upgrading or uninstalling AIMT never deletes your `.aimt` project knowledge or source files. The engine version (for example `0.2.0`) is separate from your map version.

## Security

**Read is public.** Anyone with the `.aimt` file can use `search`, `read`, `follow-*`, and `validate` without logging in.

**Write requires owner authorization.** Creating or changing knowledge requires proving ownership with the private key. The private key is never stored inside `.aimt`.

* After `aimt init`, store the printed private key securely and do not commit it.
* `aimt login` stores the credential in the OS secure store (keyring or Windows Credential Manager). On Unix it may fall back to `~/.aimt/credentials/<pub>.key` with `0o600`, on Windows the fallback is refused to avoid an insecure file.
* When not authorized, the agent must stop before any write and ask you to run `aimt login`. The agent must never ask for the private key or request it in chat.
* `open_to_read` does not grant write access, only the key does.
* `aimt key backup` writes a versioned JSON file outside the project with `0o600` (Unix) or owner only ACL (Windows). Possession grants write access, so keep it secure and do not share it.
* `aimt key rotate` creates a new keypair, updates the package, and verifies before removing the old credential. If verification fails, the old key remains usable.

Installers never touch `.aimt` files or credentials and never request private keys.

## Documentation

* Project docs: [Security](SECURITY.md) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md)

Source repository: [https://github.com/aimt010/aimt](https://github.com/aimt010/aimt)
Clone: `git clone https://github.com/aimt010/aimt.git`

## Issues

Found a bug, installation problem, unexpected behavior, or have a feature request?

Please report it on the official issue tracker:

[https://github.com/aimt010/aimt/issues](https://github.com/aimt010/aimt/issues)

Include your platform, AIMT version (`aimt --version`), and steps to reproduce if possible.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, test layout (`tests/` organized by feature), and contribution guidelines. Please use the official repository at [https://github.com/aimt010/aimt](https://github.com/aimt010/aimt) for issues and pull requests.

## License

[Apache-2.0](LICENSE)
