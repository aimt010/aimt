# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `v0.1.x` (latest `v0.1.0` line) | Yes — security fixes |
| `< 0.1.0` (pre-release development) | No |

Only the latest `v0.1.x` release line receives security fixes. Users should keep AIMT updated to the latest patch.

## Reporting a vulnerability

**Do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

Please use **GitHub Security Advisories — Private vulnerability reporting**:

- Go to the repository on GitHub → **Security** → **Report a vulnerability** (or `https://github.com/aimt010/aimt/security/advisories/new` if the repository is at `aimt010/aimt`; adjust for your fork's actual owner/name).

If private reporting is not available in your view, contact the repository maintainers through GitHub's private channels and request a private channel.

Do not include private owner keys, `~/.aimt/credentials` contents, backup files, or other secrets in the report. Redact them.

## What to include

Please include, without revealing secrets:

- affected AIMT version (`aimt --version`, `Cargo.toml` `0.1.0`) and platform (macOS/Linux/Windows, arch)
- affected component (CLI, `Store`, `HostedEngine`, installer, release artifact, plugin)
- reproduction steps (minimal `.aimt` or command sequence)
- impact (what an attacker can or cannot do)
- relevant logs or output (sanitized)
- proof-of-concept when safe
- whether the issue is already publicly known

Tell reporters not to include private keys, credentials, or sensitive user data.

## Security boundaries

AIMT's security model (enforced in Rust, not in prompts):

- **Public reads** are allowed where intended (`Store::open`, `HostedEngine::open`, `search`/`read`/`follow`/`validate`, `discover`).
- **Writes require authorization** (`Store::ensure_write_authorized`, `verify_write_credential` via `ed25519`). Without a valid owner private key, `insert`/`update`/`remove`/`persist` must fail.
- **Private owner keys are never stored inside `.aimt`**. The private key is printed once at `aimt init` and kept in the OS credential store (`keyring` / Windows Credential Manager, fallback `~/.aimt/credentials/<pub>.key` `0o600` on Unix; no insecure fallback on Windows). Only `owner_public_key` is stored inside `@aimt` where applicable.
- **Hosted/read-only access must not expose write operations**. `HostedEngine` exposes only `open`/`search`/`read`/`validate` etc.; it has no `open_mut`/`persist` and cannot be made writable even when authorized.
- **Plugins must not request or handle private owner keys**. Prompts under `.agents/aimt/` are public operational guidance; they must say *do not ask for the private key*.
- **Authorization failures must stop the operation** rather than bypass it (e.g. `AIMT write access is not authorized … Please authorize with aimt login`).

Do not publish private keys. Treat `key backup` output as highly sensitive.

## Release security

Release artifacts are built through **GitHub Actions** (`.github/workflows/release.yml`) and published as `aimt-<target>.tar.gz` / `.zip` for 6 targets.

- Each release publishes `checksums.txt` (`<sha256>  <filename>`, generated with `sha256sum`/`shasum -a 256`/`Get-FileHash` from the exact uploaded archives). Installers (`scripts/install.sh`, `scripts/install.ps1`) download `checksums.txt` over HTTPS and verify the artifact's SHA256 (exact filename, 64 hex, case-insensitive) **before extraction**, aborting on missing, malformed, or mismatched checksums.
- Release archives also have **GitHub artifact attestations** (`actions/attest-build-provenance`, `id-token`/`attestations` permissions) for verifiable build provenance.

SHA256 means *the downloaded artifact matches the published release checksum*. Attestation means *the artifact has verifiable provenance associated with the GitHub Actions build*. They are distinct.

## Disclosure process

```
private report
  → maintainer acknowledgement
  → investigation
  → fix
  → security release (patch)
  → public disclosure when appropriate
```

We do not promise specific response times, but we will acknowledge receipt and keep the reporter informed.

## Security best practices for users

- Keep AIMT updated (`aimt --version`).
- Download releases only from the official repository/release page (`https://github.com/aimt010/aimt/releases`) and verify `checksums.txt`.
- For provenance, optionally verify attestations: `gh attestation verify <archive> -R aimt010/aimt`.
- Protect owner credentials and `key backup` files (`0o600` on Unix, owner-only ACL on Windows); do not publish or commit private keys.
- Report suspicious release artifacts or installers privately as above.
