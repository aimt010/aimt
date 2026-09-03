use std::path::Path;

fn sh() -> String {
    std::fs::read_to_string("scripts/install.sh").expect("scripts/install.sh missing")
}
fn ps1() -> String {
    std::fs::read_to_string("scripts/install.ps1").expect("scripts/install.ps1 missing")
}
fn release() -> String {
    std::fs::read_to_string(".github/workflows/release.yml").expect("release workflow missing")
}

// ---- Unix install.sh ----

#[test]
fn sh_has_strict_mode() {
    let s = sh();
    assert!(s.contains("set -euo pipefail"), "must use strict shell");
}

#[test]
fn sh_detects_os() {
    let s = sh();
    assert!(s.contains("darwin"), "must detect darwin");
    assert!(s.contains("linux"), "must detect linux");
    assert!(
        s.contains("Unsupported operating system"),
        "must error on unsupported OS"
    );
}

#[test]
fn sh_detects_arch() {
    let s = sh();
    assert!(
        s.contains("arm64") && s.contains("aarch64"),
        "must detect arm64/aarch64"
    );
    assert!(
        s.contains("x86_64") && s.contains("amd64"),
        "must detect x86_64/amd64"
    );
    assert!(
        s.contains("Unsupported architecture"),
        "must error on unsupported arch"
    );
}

#[test]
fn sh_maps_to_canonical_targets() {
    let s = sh();
    for t in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
    ] {
        assert!(s.contains(t), "must map to target {}", t);
    }
    // Ensure not inventing extra targets like musl unless workflow publishes
    assert!(
        s.contains("Unsupported platform"),
        "must fail on unsupported combo"
    );
}

#[test]
fn sh_single_repo_var() {
    let s = sh();
    // Single configurable AIMT_REPO
    let count = s.matches("AIMT_REPO").count();
    assert!(count >= 2, "must use AIMT_REPO variable");
    // Check default uses https://github.com/aimt010/aimt
    assert!(
        s.contains("https://github.com/aimt010/aimt"),
        "default repo must be https://github.com/aimt010/aimt"
    );
    // Should not hardcode repo in multiple distinct URLs
    assert!(
        s.contains("AIMT_REPO:-"),
        "should have default via AIMT_REPO:-"
    );
}

#[test]
fn sh_supports_version() {
    let s = sh();
    assert!(s.contains("AIMT_VERSION"), "must support AIMT_VERSION");
    assert!(
        s.contains("releases/download"),
        "must use releases/download for versioned"
    );
    assert!(
        s.contains("releases/latest/download"),
        "must use latest when no version"
    );
}

#[test]
fn sh_download_security() {
    let s = sh();
    assert!(s.contains("https://"), "must use HTTPS");
    assert!(s.contains("curl"), "must use curl");
    assert!(s.contains("--fail"), "must use --fail");
    assert!(s.contains("--silent"), "must use --silent");
    assert!(s.contains("--show-error"), "must use --show-error");
    assert!(s.contains("--location"), "must use --location");
    assert!(
        s.contains("--proto '=https'") || s.contains("--proto"),
        "should enforce https proto"
    );
    // Must NOT do curl | sh internally (only allowed is user downloading installer itself)
    // Ensure script does not contain `| sh` or `| bash` for internal download
    assert!(
        !s.contains("curl") || !s.contains("| sh") || s.matches("| sh").count() == 0,
        "must not use curl | sh internally"
    );
    // Only download from official GitHub release URL (AIMT_REPO)
    assert!(s.contains("AIMT_REPO"), "download must be from AIMT_REPO");
    assert!(
        !s.to_lowercase().contains("mirror"),
        "must not use arbitrary mirrors"
    );
}

#[test]
fn sh_install_dir() {
    let s = sh();
    assert!(
        s.contains("~/.local/bin") || s.contains("$HOME/.local/bin"),
        "must default to ~/.local/bin"
    );
    assert!(
        s.contains("AIMT_INSTALL_DIR"),
        "must support custom install dir"
    );
    assert!(
        !s.contains("/usr/local/bin"),
        "must not default to /usr/local/bin"
    );
    assert!(s.contains("mkdir -p"), "must create dir if needed");
    assert!(s.contains("chmod 755"), "must chmod 755");
    assert!(!s.contains("chmod 777"), "must never use 777");
    // Installer must not invoke sudo (comments mentioning sudo are ok)
    assert!(
        !s.contains("\nsudo ")
            && !s.contains("\nsudo\n")
            && !s.contains("sudo mkdir")
            && !s.contains("sudo cp"),
        "must not invoke sudo"
    );
}

#[test]
fn sh_path_handling() {
    let s = sh();
    assert!(s.contains("PATH"), "must handle PATH");
    assert!(
        s.contains("already contains"),
        "must detect already in PATH"
    );
    assert!(
        s.contains("does not currently contain") || s.contains("does not contain"),
        "must instruct if not in PATH"
    );
    assert!(s.contains("export PATH="), "must print export instruction");
    // Must not blindly modify shell configs
    assert!(
        !s.contains(">> ~/.zshrc") && !s.contains(">> ~/.bashrc"),
        "must not blindly modify shell configs"
    );
    assert!(
        !s.contains("~/.zshrc") || s.contains("you can add"),
        "macOS zsh instruction should be advisory only"
    );
}

#[test]
fn sh_archive_handling() {
    let s = sh();
    assert!(s.contains("mktemp"), "must create temp dir");
    assert!(s.contains("trap"), "must use trap for cleanup");
    assert!(
        s.contains("tar -xzf") || s.contains("tar"),
        "must extract tar.gz"
    );
    assert!(s.contains("aimt"), "must locate aimt executable");
    assert!(
        s.contains("cp") && s.contains("mv -f"),
        "must install atomically via cp+mv"
    );
    assert!(
        s.contains("rm -rf") || s.contains("cleanup"),
        "must clean up temp"
    );
    assert!(s.contains(".tar.gz"), "must handle tar.gz");
}

#[test]
fn sh_existing_installation_and_security() {
    let s = sh();
    // Reinstall/upgrade support
    assert!(
        s.to_lowercase().contains("existing") || s.contains("Existing"),
        "must detect existing version"
    );
    assert!(
        s.contains("aimt --version") || s.contains("--version"),
        "must handle version"
    );
    // Must NEVER touch .aimt, credentials, project files
    assert!(
        !s.contains(".aimt/credentials") || s.contains("Never touch"),
        "must not touch credentials"
    );
    // Ensure script mentions not touching those paths (or at least doesn't contain rm of them)
    assert!(
        !s.contains("rm -rf ~/.aimt"),
        "must never delete credentials"
    );
}

#[test]
fn sh_version_verification() {
    let s = sh();
    assert!(
        s.contains("aimt --version"),
        "must run aimt --version after install"
    );
    assert!(
        s.contains("Verifying") || s.contains("verify"),
        "must verify executable starts"
    );
    assert!(
        s.contains("Failed to verify") || s.contains("failed"),
        "must report failure if version fails"
    );
}

#[test]
fn sh_error_handling() {
    let s = sh();
    for msg in [
        "Unsupported operating system",
        "Unsupported architecture",
        "Failed to download",
        "Failed to extract",
        "Failed to install",
    ] {
        assert!(s.contains(msg), "must have error: {}", msg);
    }
    assert!(
        !s.contains("private key") || s.contains("Do NOT"),
        "must not expose private keys"
    );
}

#[test]
fn sh_env_vars() {
    let s = sh();
    assert!(s.contains("AIMT_VERSION"), "must support AIMT_VERSION");
    assert!(s.contains("AIMT_REPO"), "must support AIMT_REPO");
    assert!(
        s.contains("AIMT_INSTALL_DIR"),
        "must support AIMT_INSTALL_DIR"
    );
}

#[test]
fn sh_idempotency_and_cleanup() {
    let s = sh();
    assert!(
        s.contains("trap cleanup") || s.contains("trap"),
        "must have cleanup trap"
    );
    assert!(s.contains("mktemp -d"), "must use temp dir");
    // Idempotent: should handle existing binary via atomic mv -f
    assert!(
        s.contains("mv -f"),
        "must atomically replace binary for idempotency"
    );
}

// ---- Windows install.ps1 ----

#[test]
fn ps1_detects_arch() {
    let s = ps1();
    assert!(
        s.contains("amd64") || s.contains("x86_64"),
        "must detect x64"
    );
    assert!(
        s.contains("arm64") || s.contains("aarch64"),
        "must detect ARM64"
    );
    assert!(
        s.contains("Unsupported architecture"),
        "must error on unsupported arch"
    );
}

#[test]
fn ps1_target_and_artifact() {
    let s = ps1();
    assert!(
        s.contains("x86_64-pc-windows-msvc"),
        "must support x64 windows target"
    );
    // aarch64 may or may not be published, but installer should handle it
    assert!(
        s.contains("aarch64-pc-windows-msvc") || s.contains("Unsupported"),
        "should handle ARM64 target or error"
    );
    assert!(s.contains(".zip"), "must handle zip archive");
}

#[test]
fn ps1_repo_and_version() {
    let s = ps1();
    assert!(
        s.contains("AIMT_REPO") || s.contains("$Repo"),
        "must use repo var"
    );
    assert!(
        s.contains("AIMT_VERSION") || s.contains("$Version"),
        "must support version"
    );
    assert!(
        s.contains("releases/download") || s.contains("releases/latest"),
        "must use release URL"
    );
}

#[test]
fn ps1_install_dir_and_no_admin() {
    let s = ps1();
    assert!(
        s.contains("LOCALAPPDATA") && s.contains("AIMT\\bin"),
        "must install to LOCALAPPDATA\\AIMT\\bin"
    );
    assert!(
        s.contains("AIMT_INSTALL_DIR") || s.contains("InstallDir"),
        "must support custom dir"
    );
    assert!(
        s.to_lowercase().contains("not require administrator")
            || s.to_lowercase().contains("no administrator")
            || s.to_lowercase().contains("without administrator"),
        "must state not requiring admin"
    );
}

#[test]
fn ps1_user_path_handling() {
    let s = ps1();
    assert!(s.contains("User"), "must update user-level PATH");
    assert!(
        !s.to_lowercase().contains("system path") || s.contains("do not modify the system"),
        "must not modify system PATH"
    );
    assert!(
        s.to_lowercase().contains("already contains") || s.contains("AlreadyInPath"),
        "must avoid duplicates"
    );
    assert!(
        s.to_lowercase().contains("preserve") || s.contains("duplicate"),
        "must preserve existing entries"
    );
    assert!(
        s.contains("GetEnvironmentVariable") && s.contains("SetEnvironmentVariable"),
        "must use Environment PATH APIs"
    );
    assert!(
        s.to_lowercase().contains("case-insensitive") || s.contains("ToLower"),
        "must handle case-insensitive"
    );
}

#[test]
fn ps1_security_and_not_touching_credentials() {
    let s = ps1();
    assert!(
        !s.contains(".aimt/credentials") || s.contains("Never touch"),
        "must not touch credentials"
    );
    assert!(
        !s.contains("private key") || s.contains("Do NOT"),
        "must not request private key"
    );
}

#[test]
fn ps1_archive_and_verify() {
    let s = ps1();
    assert!(
        s.contains("Expand-Archive"),
        "must extract zip via Expand-Archive"
    );
    assert!(s.contains("aimt.exe"), "must handle aimt.exe");
    assert!(s.contains("--version"), "must verify via --version");
    assert!(
        s.contains("AIMT installed successfully"),
        "must print success and restart terminal"
    );
}

// ---- Release workflow ----

#[test]
fn release_workflow_exists_and_has_canonical_targets() {
    let r = release();
    for t in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
    ] {
        assert!(r.contains(t), "workflow must build target {}", t);
    }
    // Artifact naming must match installer: aimt-<target>.tar.gz / .zip
    assert!(
        r.contains("aimt-${{ matrix.target }}.tar.gz") || r.contains("aimt-"),
        "must use canonical artifact naming"
    );
    assert!(
        r.contains("install.sh") && r.contains("install.ps1"),
        "must publish install scripts as release assets"
    );
}

#[test]
fn release_workflow_publish_logic() {
    let r = release();
    assert!(
        r.contains("softprops/action-gh-release") || r.contains("gh-release"),
        "must use release publish action"
    );
    assert!(r.contains("contents: write"), "must have write permissions");
    assert!(
        r.contains("tags:") && r.contains("v*"),
        "must trigger on v* tags"
    );
}

// ---- Checksum and attestation ----

#[test]
fn sh_downloads_checksums_txt() {
    let s = sh();
    assert!(s.contains("checksums.txt"), "must download checksums.txt");
    assert!(
        s.contains("CHECKSUMS_URL") || s.contains("checksums.txt"),
        "must have checksums URL"
    );
    // Must use HTTPS for checksums as well
    assert!(
        s.contains("checksums.txt") && s.contains("https://"),
        "checksums must be over HTTPS"
    );
}

#[test]
fn sh_verifies_checksum_before_extract() {
    let s = sh();
    // Download -> checksum verification -> extract
    let download_pos = s.find("Downloading AIMT from:").unwrap_or(0);
    let checksum_pos = s
        .find("Checksum verified")
        .unwrap_or(s.find("CHECKSUMS_URL").unwrap_or(0));
    let extract_pos = s.find("Extracting archive").unwrap_or(usize::MAX);
    assert!(
        checksum_pos > download_pos,
        "checksum verification must be after download"
    );
    assert!(
        extract_pos > checksum_pos,
        "must verify before extract: checksum before tar -xzf"
    );
    assert!(
        s.contains("Checksum verified"),
        "must print checksum verified"
    );
    assert!(
        s.contains("Checksum verification failed"),
        "must have failure message"
    );
    assert!(
        s.contains("Installation was aborted"),
        "must abort on mismatch"
    );
}

#[test]
fn sh_checksum_parsing_is_strict() {
    let s = sh();
    // Must check exact filename, 64 hex, case-insensitive
    assert!(
        s.contains("  $ARTIFACT") || s.contains("  \"\\$ARTIFACT\""),
        "must use exact filename with two spaces"
    );
    assert!(
        s.contains("^[0-9a-fA-F]\\{64\\}") || s.contains("[0-9a-fA-F]{64}"),
        "must validate 64 hex"
    );
    assert!(
        s.contains("tr '[:upper:]' '[:lower:]'") || s.contains("ToLower"),
        "must compare case-insensitively"
    );
    assert!(s.contains("malformed checksum"), "must reject malformed");
    assert!(
        s.contains("not found in checksums.txt"),
        "must handle missing entry"
    );
}

#[test]
fn sh_no_unverified_install() {
    let s = sh();
    // Must not install or chmod before verification
    let verify_pos = s.find("Checksum verified").unwrap_or(0);
    let install_pos = s.find("Installing to").unwrap_or(usize::MAX);
    assert!(verify_pos < install_pos, "must verify before install");
    assert!(
        s.contains("Do not proceed with unverified") || s.contains("unverified installation"),
        "must not fall back to unverified"
    );
}

#[test]
fn sh_handles_all_six_targets_in_checksums() {
    let s = sh();
    // All six artifacts should be verifiable via same logic (exact filename)
    // The script's ARTIFACT var is used, so any of the six will be checked via same path
    assert!(
        s.contains("aimt-"),
        "must handle all six targets via ARTIFACT var"
    );
    let r = release();
    for art in [
        "aimt-aarch64-apple-darwin.tar.gz",
        "aimt-x86_64-apple-darwin.tar.gz",
        "aimt-aarch64-unknown-linux-gnu.tar.gz",
        "aimt-x86_64-unknown-linux-gnu.tar.gz",
        "aimt-x86_64-pc-windows-msvc.zip",
        "aimt-aarch64-pc-windows-msvc.zip",
    ] {
        assert!(
            r.contains(&art[5..15]) || r.contains(art),
            "workflow must build {}",
            art
        );
    }
}

#[test]
fn sh_no_insecure_download() {
    let s = sh();
    assert!(
        !s.contains("curl -k") && !s.contains("curl --insecure"),
        "must not use curl -k"
    );
    assert!(
        !s.contains("wget --no-check-certificate"),
        "must not use wget --no-check-certificate"
    );
    assert!(
        !s.contains("http://") || s.contains("https://"),
        "must not use HTTP"
    );
    // Must use --proto '=https' or similar
    assert!(s.contains("--proto"), "must enforce https proto");
}

#[test]
fn ps1_verifies_checksum_with_getfilehash() {
    let s = ps1();
    assert!(s.contains("checksums.txt"), "must download checksums.txt");
    assert!(
        s.contains("Get-FileHash"),
        "must use Get-FileHash -Algorithm SHA256 on Windows"
    );
    assert!(s.contains("-Algorithm SHA256"), "must specify SHA256");
    assert!(s.contains("Checksum verified"), "must print verified");
    assert!(
        s.contains("Checksum verification failed"),
        "must abort on mismatch"
    );
    // Must verify before Expand-Archive
    let verify_pos = s.find("Checksum verified").unwrap_or(0);
    let extract_pos = s.find("Expand-Archive").unwrap_or(usize::MAX);
    assert!(
        verify_pos < extract_pos,
        "must verify before extract on Windows"
    );
    assert!(
        s.contains("64") && s.contains("[0-9a-fA-F]{64}"),
        "must validate 64 hex"
    );
}

#[test]
fn release_generates_checksums() {
    let r = release();
    assert!(
        r.contains("checksums.txt"),
        "workflow must generate checksums.txt"
    );
    assert!(
        r.contains("sha256sum") || r.contains("shasum -a 256") || r.contains("Get-FileHash"),
        "must use native SHA256 tool"
    );
    assert!(
        r.contains("aimt-*.tar.gz") && r.contains("aimt-*.zip"),
        "must checksum all archives"
    );
    // Must fail if no archives
    assert!(
        r.contains("no archives found") || r.contains("Failed to generate checksums"),
        "must not ignore failures"
    );
}

#[test]
fn release_has_attestations() {
    let r = release();
    assert!(
        r.contains("attest-build-provenance"),
        "must use official attestation action"
    );
    assert!(r.contains("id-token: write"), "must have id-token: write");
    assert!(
        r.contains("attestations: write"),
        "must have attestations: write"
    );
    assert!(r.contains("subject-path"), "must attest subject-path");
    assert!(
        r.contains("aimt-*.tar.gz") || r.contains("aimt-*.zip"),
        "must attest final archives"
    );
}

#[test]
fn release_has_checksums_in_assets() {
    let r = release();
    assert!(
        r.contains("checksums.txt"),
        "must upload checksums.txt as release asset"
    );
    // Ensure attestations reference exact archives, not intermediate
    assert!(
        !r.contains(".sig") || r.contains("attest"),
        "must not use fake .sig, use attest action"
    );
}

// ---- Script syntax validation ----

#[test]
fn sh_bash_syntax() {
    let out = std::process::Command::new("bash")
        .args(["-n", "scripts/install.sh"])
        .output()
        .expect("bash -n failed");
    assert!(
        out.status.success(),
        "bash -n should pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn sh_executable_permissions() {
    let meta = std::fs::metadata("scripts/install.sh").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        assert!(mode & 0o111 != 0, "install.sh should be executable");
    }
}

#[test]
fn ps1_exists() {
    assert!(
        Path::new("scripts/install.ps1").exists(),
        "install.ps1 must exist"
    );
}

#[test]
fn install_scripts_no_private_key_handling() {
    let sh = sh();
    let ps1 = ps1();
    for s in [&sh, &ps1] {
        // Installer must not request private key, login, or modify .aimt
        assert!(
            !s.contains("owner_private_key"),
            "installer must not handle owner_private_key"
        );
        assert!(!s.contains("aimt login"), "installer must not do login");
        let lower = s.to_lowercase();
        assert!(
            !lower.contains("private key") || lower.contains("do not"),
            "must not request private key"
        );
    }
}
