#!/usr/bin/env bash
set -euo pipefail

# AIMT installer for macOS and Linux
# Downloads prebuilt binary from GitHub Releases, installs to user-owned directory.
# Does NOT compile from source, does NOT require sudo, does NOT modify shell configs.

AIMT_REPO="${AIMT_REPO:-https://github.com/aimt010/aimt}"
AIMT_VERSION="${AIMT_VERSION:-}"
AIMT_INSTALL_DIR="${AIMT_INSTALL_DIR:-}"

# Installation directory: prefer ~/.local/bin
if [ -n "$AIMT_INSTALL_DIR" ]; then
  INSTALL_DIR="$AIMT_INSTALL_DIR"
else
  INSTALL_DIR="$HOME/.local/bin"
fi

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Darwin) OS="darwin" ;;
  Linux) OS="linux" ;;
  *) echo "Unsupported operating system: $OS" >&2; echo "AIMT supports macOS and Linux only for this installer. Use Windows PowerShell installer on Windows." >&2; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64) ARCH="x86_64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; echo "AIMT supports arm64 (aarch64) and x86_64 (amd64) only." >&2; exit 1 ;;
esac

# Map OS+ARCH to Rust target triple (must match release workflow)
TARGET=""
case "${OS}-${ARCH}" in
  darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
  darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  linux-arm64)   TARGET="aarch64-unknown-linux-gnu" ;;
  linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  *) echo "Unsupported platform: ${OS}-${ARCH}" >&2; exit 1 ;;
esac

# Artifact name: aimt-<target>.tar.gz (must match release workflow)
ARTIFACT="aimt-${TARGET}.tar.gz"

# Build download URL
if [ -n "$AIMT_VERSION" ]; then
  # Normalize version: ensure leading v
  case "$AIMT_VERSION" in
    v*) VERSION="$AIMT_VERSION" ;;
    *) VERSION="v$AIMT_VERSION" ;;
  esac
  DOWNLOAD_URL="${AIMT_REPO}/releases/download/${VERSION}/${ARTIFACT}"
else
  DOWNLOAD_URL="${AIMT_REPO}/releases/latest/download/${ARTIFACT}"
  VERSION="latest"
fi

echo "Installing AIMT ${VERSION} for ${TARGET}..."
echo "Repository: ${AIMT_REPO}"
if [ "$VERSION" != "latest" ]; then
  echo "Version: ${VERSION}"
fi

# Check existing installation
if command -v aimt >/dev/null 2>&1; then
  EXISTING_VERSION="$(aimt --version 2>/dev/null || echo "unknown")"
  echo "Existing installation found: ${EXISTING_VERSION} at $(command -v aimt)"
fi

# Create install directory if needed
if [ ! -d "$INSTALL_DIR" ]; then
  echo "Creating installation directory: $INSTALL_DIR"
  mkdir -p "$INSTALL_DIR" || { echo "Failed to create installation directory: $INSTALL_DIR" >&2; exit 1; }
fi

# Create temporary directory with cleanup trap
TMPDIR="$(mktemp -d 2>/dev/null || mktemp -d -t aimt-install)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT INT TERM

ARCHIVE_PATH="$TMPDIR/$ARTIFACT"

echo "Downloading AIMT from: $DOWNLOAD_URL"
DOWNLOAD_OK=0
if command -v curl >/dev/null 2>&1; then
  if curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$DOWNLOAD_URL" --output "$ARCHIVE_PATH"; then
    DOWNLOAD_OK=1
  fi
elif command -v wget >/dev/null 2>&1; then
  if wget --https-only -q -O "$ARCHIVE_PATH" "$DOWNLOAD_URL"; then
    DOWNLOAD_OK=1
  fi
else
  echo "Failed to download AIMT: neither curl nor wget found. Please install curl or wget." >&2
  exit 1
fi
if [ "$DOWNLOAD_OK" -ne 1 ]; then
  echo "Failed to download AIMT from $DOWNLOAD_URL" >&2
  if [ -n "$AIMT_VERSION" ]; then
    echo "AIMT release not found: $VERSION for ${TARGET}" >&2
    echo "Check available releases at ${AIMT_REPO}/releases" >&2
  else
    echo "Failed to download latest AIMT release. Check your network and ${AIMT_REPO}/releases" >&2
  fi
  exit 1
fi

if [ ! -s "$ARCHIVE_PATH" ]; then
  echo "Failed to download AIMT: archive is empty" >&2
  exit 1
fi

# Download and verify checksums.txt (mandatory)
if [ -n "$AIMT_VERSION" ]; then
  CHECKSUMS_URL="${AIMT_REPO}/releases/download/${VERSION}/checksums.txt"
else
  CHECKSUMS_URL="${AIMT_REPO}/releases/latest/download/checksums.txt"
fi
CHECKSUMS_PATH="$TMPDIR/checksums.txt"
echo "Downloading checksums from: $CHECKSUMS_URL"
CHECKSUMS_OK=0
if command -v curl >/dev/null 2>&1; then
  if curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$CHECKSUMS_URL" --output "$CHECKSUMS_PATH"; then
    CHECKSUMS_OK=1
  fi
elif command -v wget >/dev/null 2>&1; then
  if wget --https-only -q -O "$CHECKSUMS_PATH" "$CHECKSUMS_URL"; then
    CHECKSUMS_OK=1
  fi
fi
if [ "$CHECKSUMS_OK" -ne 1 ] || [ ! -s "$CHECKSUMS_PATH" ]; then
  echo "Failed to download checksums.txt from $CHECKSUMS_URL" >&2
  echo "Checksum verification failed and installation was aborted." >&2
  echo "Do not proceed with unverified installation." >&2
  exit 1
fi

# Verify checksum: exact filename, 64 hex, case-insensitive
EXPECTED_LINE="$(grep -F "  $ARTIFACT" "$CHECKSUMS_PATH" | head -n 1 || true)"
if [ -z "$EXPECTED_LINE" ]; then
  echo "Checksum verification failed: expected artifact $ARTIFACT not found in checksums.txt" >&2
  echo "Installation was aborted." >&2
  cat "$CHECKSUMS_PATH" >&2
  exit 1
fi
EXPECTED_HASH="$(echo "$EXPECTED_LINE" | awk '{print $1}')"
# Validate 64 hex
if ! echo "$EXPECTED_HASH" | grep -Eq '^[0-9a-fA-F]{64}$'; then
  echo "Checksum verification failed: malformed checksum for $ARTIFACT: $EXPECTED_HASH" >&2
  echo "Installation was aborted." >&2
  exit 1
fi
# Ensure the line is exactly "64hex  filename" with no extra
if ! echo "$EXPECTED_LINE" | grep -Eq '^[0-9a-fA-F]{64}  '"$ARTIFACT"'$'; then
  echo "Checksum verification failed: malformed checksum line for $ARTIFACT" >&2
  echo "Installation was aborted." >&2
  exit 1
fi
# Calculate actual hash
ACTUAL_HASH=""
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_HASH="$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL_HASH="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"
else
  echo "Checksum verification failed: neither sha256sum nor shasum found" >&2
  echo "Installation was aborted." >&2
  exit 1
fi
if [ -z "$ACTUAL_HASH" ]; then
  echo "Checksum verification failed: could not calculate local checksum" >&2
  exit 1
fi
# Compare case-insensitively
EXPECTED_NORM="$(echo "$EXPECTED_HASH" | tr '[:upper:]' '[:lower:]')"
ACTUAL_NORM="$(echo "$ACTUAL_HASH" | tr '[:upper:]' '[:lower:]')"
if [ "$EXPECTED_NORM" != "$ACTUAL_NORM" ]; then
  echo "Checksum verification failed for $ARTIFACT" >&2
  echo "  Expected: $EXPECTED_HASH" >&2
  echo "  Actual:   $ACTUAL_HASH" >&2
  echo "Installation was aborted. The downloaded archive may be corrupted or tampered." >&2
  exit 1
fi
echo "Checksum verified: $ARTIFACT"

echo "Extracting archive..."
if ! tar -xzf "$ARCHIVE_PATH" -C "$TMPDIR" 2>/dev/null; then
  echo "Failed to extract AIMT archive: $ARCHIVE_PATH" >&2
  echo "The downloaded file may be corrupted. Try again or check ${AIMT_REPO}/releases" >&2
  exit 1
fi

# Locate aimt binary (may be at top level or nested)
BIN_SRC=""
if [ -f "$TMPDIR/aimt" ]; then
  BIN_SRC="$TMPDIR/aimt"
else
  BIN_SRC="$(find "$TMPDIR" -type f -name "aimt" | head -n 1 || true)"
fi

if [ -z "$BIN_SRC" ] || [ ! -f "$BIN_SRC" ]; then
  echo "Failed to extract AIMT: aimt executable not found in archive" >&2
  ls -la "$TMPDIR" >&2 || true
  exit 1
fi

# Install binary atomically
DEST="$INSTALL_DIR/aimt"
TMP_DEST="${DEST}.tmp.$$"

echo "Installing to $DEST..."
if ! cp "$BIN_SRC" "$TMP_DEST"; then
  echo "Failed to install AIMT to $DEST" >&2
  exit 1
fi

chmod 755 "$TMP_DEST" || { echo "Failed to set executable permissions on $DEST" >&2; exit 1; }

# Atomic replace
if ! mv -f "$TMP_DEST" "$DEST"; then
  echo "Failed to install AIMT: unable to move binary to $DEST" >&2
  rm -f "$TMP_DEST"
  exit 1
fi

# Never touch .aimt, ~/.aimt/, credentials, or project files (installer only touches binary)

# Verify installation
echo "Verifying installation..."
if [ ! -x "$DEST" ]; then
  echo "Failed to install AIMT: $DEST is not executable" >&2
  exit 1
fi

if ! VERSION_OUTPUT="$("$DEST" --version 2>&1)"; then
  echo "Failed to verify AIMT installation: $DEST --version failed" >&2
  echo "$VERSION_OUTPUT" >&2
  exit 1
fi

echo "Installed: $VERSION_OUTPUT"

# If specific version requested, verify match (best-effort, allow v prefix variance)
if [ -n "$AIMT_VERSION" ]; then
  REQUESTED_NORM="$(echo "$VERSION" | sed 's/^v//')"
  INSTALLED_NORM="$(echo "$VERSION_OUTPUT" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1 || true)"
  if [ -n "$INSTALLED_NORM" ] && [ "$INSTALLED_NORM" != "$REQUESTED_NORM" ]; then
    echo "Warning: installed version ($INSTALLED_NORM) does not match requested version ($REQUESTED_NORM)" >&2
  fi
fi

echo ""
echo "AIMT installed successfully."
echo ""
echo "Binary:"
echo "  $DEST"
echo ""

# PATH handling: do not modify shell configs, just instruct
# Explain that child installer cannot modify parent shell PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    echo "Your PATH already contains $INSTALL_DIR."
    ;;
  *)
    echo "Your PATH does not currently contain $INSTALL_DIR."
    echo ""
    echo "Add it with:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then restart your shell or reload your profile."
    if [ "$OS" = "darwin" ]; then
      echo "On macOS (zsh), you can add the line to ~/.zshrc"
    else
      echo "On Linux:"
      echo "  bash/zsh: add to ~/.bashrc, ~/.zshrc, or ~/.profile"
      echo "  fish:     fish_add_path ~/.local/bin  # or: set -Ux fish_user_paths ~/.local/bin \$fish_user_paths"
    fi
    echo ""
    echo "Note: shell PATH changes made by a child process do not affect the current shell."
    ;;
esac

echo ""
echo "Run: aimt --help"
