# Contributing to AIMT

## About AIMT

AIMT (AI Mapping Taxonomy) is a small knowledge-mapping format and runtime that makes project knowledge understandable to both humans and AI. This repository contains the Rust runtime, `.pmap` parser, `.aimt` package handling, CLI (`aimt`), installer scripts, and plugin integrations. AIMT organizes knowledge; AI provides intelligence.

## Development requirements

- **Rust toolchain** (stable) + **Cargo** — `rustup` recommended
- **Git**

No other tools are required. Works on macOS, Linux, and Windows.

## Repository structure

```
src/          Rust source (core, storage, security, hosted, plugins, workflows, visualizer)
tests/        Integration and contract tests
scripts/      Release installers (install.sh, install.ps1)
.github/      CI and release workflow (release.yml)
Cargo.toml    Package manifest (package `aimt`, binary `aimt`, version 0.1.0)
README.md     User documentation
docs/         Specifications (pmap, levels, fields) and architecture
```

Only documented paths exist in the repository.

## Development workflow

```
fork / branch
  → make focused change
  → cargo fmt --check
  → cargo check
  → cargo clippy --all-targets --all-features -- -D warnings
  → cargo test -- --test-threads=1
  → git diff (review)
  → commit
  → pull request
```

For release builds: `cargo build --release` and `bash -n scripts/install.sh` (and `Get-Content scripts/install.ps1` where PowerShell is available).

## Testing

Add or update tests for behavioral changes. Existing tests live under `tests/` (see `tests/install_script/`, `tests/project/`, etc.). Use `cargo test -- --test-threads=1` for deterministic integration tests.

## CLI changes

Changes to user-facing CLI (`aimt init`, `search`, `read`, `follow-*`, `validate`, `status`, `login`/`logout`, `key backup`/`rotate`, `view`, `install`) must:

- update tests,
- update `README.md` and help text when behavior changes,
- preserve documented project-discovery semantics (explicit `[project.aimt]` wins; 0/1/2+ discovery),
- avoid reintroducing redundant syntax (`aimt aimt`, `--aimt`).

## Security-sensitive code

Changes involving authentication, owner keys, `Store` authorization, `.aimt` storage, `HostedEngine` read-only guarantees, `~/.aimt/credentials`, installers, release artifacts (`aimt-*.tar.gz`/`*.zip`, `checksums.txt`, attestations), or plugin authorization require extra care and tests. **Do not report vulnerabilities via public issues — see `SECURITY.md`.**

## Pull requests

PRs should explain:

- **what** changed and **why**,
- tests performed (`cargo test`, `cargo clippy`, etc.),
- documentation changes if user-facing,
- security implications if applicable.

Keep PRs focused; avoid unrelated refactors.

## Commit guidance

No strict convention is enforced. Use clear, focused commits (e.g. `feat: …`, `fix: …`) that are easy to review.

## Scope

Avoid unrelated refactors in focused PRs. For v0.1.0, do not redesign `.aimt` format, `MAGIC`/`VERSION`, or core authorization without discussion.
