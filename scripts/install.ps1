# AIMT installer for Windows PowerShell
# Downloads prebuilt binary from GitHub Releases, installs to user-level directory.
# Does NOT require Administrator, does NOT compile from source.

param(
    [string]$Version = $env:AIMT_VERSION,
    [string]$Repo = $env:AIMT_REPO,
    [string]$InstallDir = $env:AIMT_INSTALL_DIR
)

$ErrorActionPreference = "Stop"

# Single configurable repository variable
if (-not $Repo -or $Repo -eq "") {
    $Repo = "https://github.com/aimt010/aimt"
}
# Normalize repo: remove trailing slash
$Repo = $Repo.TrimEnd('/')

# Version handling: allow AIMT_VERSION with or without v prefix
$RequestedVersion = $null
if ($Version -and $Version -ne "") {
    if ($Version.StartsWith("v")) {
        $RequestedVersion = $Version
    } else {
        $RequestedVersion = "v$Version"
    }
}

# Detect architecture: map to Rust target triple
$Arch = $env:PROCESSOR_ARCHITECTURE
# On 64-bit PowerShell running on ARM, PROCESSOR_ARCHITEW6432 may indicate true arch
if ($env:PROCESSOR_ARCHITEW6432) {
    $Arch = $env:PROCESSOR_ARCHITEW6432
}

$Target = $null
switch ($Arch.ToLower()) {
    "amd64" { $Target = "x86_64-pc-windows-msvc" }
    "x86_64" { $Target = "x86_64-pc-windows-msvc" }
    "arm64" { $Target = "aarch64-pc-windows-msvc" }
    "aarch64" { $Target = "aarch64-pc-windows-msvc" }
    default {
        Write-Error "Unsupported architecture: $Arch. AIMT supports x64 (amd64) and ARM64."
        exit 1
    }
}

# Validate Windows: we are on Windows
if (-not $IsWindows -and $env:OS -notlike "*Windows*") {
    # PowerShell Core may not have IsWindows on older versions, but we are in install.ps1 so assume Windows
}

# Artifact name: aimt-<target>.zip (must match release workflow)
$Artifact = "aimt-$Target.zip"

if ($RequestedVersion) {
    $DownloadUrl = "$Repo/releases/download/$RequestedVersion/$Artifact"
    $DisplayVersion = $RequestedVersion
} else {
    $DownloadUrl = "$Repo/releases/latest/download/$Artifact"
    $DisplayVersion = "latest"
}

Write-Host "Installing AIMT $DisplayVersion for $Target..."
Write-Host "Repository: $Repo"
if ($RequestedVersion) {
    Write-Host "Version: $RequestedVersion"
}

# Installation directory: prefer $env:LOCALAPPDATA\AIMT\bin
if (-not $InstallDir -or $InstallDir -eq "") {
    $LocalAppData = $env:LOCALAPPDATA
    if (-not $LocalAppData -or $LocalAppData -eq "") {
        $LocalAppData = [Environment]::GetFolderPath("LocalApplicationData")
    }
    $InstallDir = Join-Path $LocalAppData "AIMT\bin"
}

Write-Host "Installation directory: $InstallDir"

# Check existing installation
$Existing = Get-Command aimt -ErrorAction SilentlyContinue
if ($Existing) {
    try {
        $ExistingVersion = & aimt --version 2>$null
        Write-Host "Existing installation found: $ExistingVersion at $($Existing.Source)"
    } catch {
        Write-Host "Existing installation found at $($Existing.Source)"
    }
}

# Create install directory if needed
if (-not (Test-Path $InstallDir)) {
    Write-Host "Creating installation directory: $InstallDir"
    try {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    } catch {
        Write-Error "Failed to create installation directory: $InstallDir. $_"
        exit 1
    }
}

# Create temporary directory
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "aimt-install-$(Get-Random)"
try {
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
} catch {
    Write-Error "Failed to create temporary directory: $TempDir. $_"
    exit 1
}

$ArchivePath = Join-Path $TempDir $Artifact

# Download with TLS 1.2, HTTPS only
try {
    Write-Host "Downloading AIMT from: $DownloadUrl"
    # Enforce TLS 1.2
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    # Use Invoke-WebRequest with fail on error
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing -ErrorAction Stop
} catch {
    Write-Error "Failed to download AIMT from $DownloadUrl. $_"
    if ($RequestedVersion) {
        Write-Error "AIMT release not found: $RequestedVersion for $Target. Check $Repo/releases"
    } else {
        Write-Error "Failed to download latest AIMT release. Check your network and $Repo/releases"
    }
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

if (-not (Test-Path $ArchivePath) -or (Get-Item $ArchivePath).Length -eq 0) {
    Write-Error "Failed to download AIMT: archive is empty or missing"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

# Download and verify checksums.txt (mandatory)
if ($RequestedVersion) {
    $ChecksumsUrl = "$Repo/releases/download/$RequestedVersion/checksums.txt"
} else {
    $ChecksumsUrl = "$Repo/releases/latest/download/checksums.txt"
}
$ChecksumsPath = Join-Path $TempDir "checksums.txt"
Write-Host "Downloading checksums from: $ChecksumsUrl"
try {
    Invoke-WebRequest -Uri $ChecksumsUrl -OutFile $ChecksumsPath -UseBasicParsing -ErrorAction Stop
} catch {
    Write-Error "Failed to download checksums.txt from $ChecksumsUrl. $_"
    Write-Error "Checksum verification failed and installation was aborted."
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
if (-not (Test-Path $ChecksumsPath) -or (Get-Item $ChecksumsPath).Length -eq 0) {
    Write-Error "Failed to download checksums.txt: file is empty or missing"
    Write-Error "Checksum verification failed and installation was aborted."
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
# Parse checksums.txt: exact filename, 64 hex, case-insensitive
$ExpectedLine = Get-Content $ChecksumsPath | Where-Object { $_ -match "^[0-9a-fA-F]{64}  $([regex]::Escape($Artifact))$" } | Select-Object -First 1
if (-not $ExpectedLine) {
    # Try loose match to give better error for malformed or missing
    $Loose = Get-Content $ChecksumsPath | Where-Object { $_ -like "*$Artifact*" } | Select-Object -First 1
    if ($Loose) {
        Write-Error "Checksum verification failed: malformed checksum line for $Artifact : $Loose"
    } else {
        Write-Error "Checksum verification failed: expected artifact $Artifact not found in checksums.txt"
        Get-Content $ChecksumsPath | ForEach-Object { Write-Host $_ }
    }
    Write-Error "Installation was aborted."
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
$ExpectedHash = ($ExpectedLine -split '\s+')[0]
if ($ExpectedHash -notmatch '^[0-9a-fA-F]{64}$') {
    Write-Error "Checksum verification failed: malformed checksum for $Artifact : $ExpectedHash"
    Write-Error "Installation was aborted."
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
# Calculate actual hash
try {
    $ActualHash = (Get-FileHash -Algorithm SHA256 -Path $ArchivePath).Hash
} catch {
    Write-Error "Checksum verification failed: could not calculate local checksum. $_"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
if ($ExpectedHash.ToLower() -ne $ActualHash.ToLower()) {
    Write-Error "Checksum verification failed for $Artifact"
    Write-Error "  Expected: $ExpectedHash"
    Write-Error "  Actual:   $ActualHash"
    Write-Error "Installation was aborted. The downloaded archive may be corrupted or tampered."
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
Write-Host "Checksum verified: $Artifact"

Write-Host "Extracting archive..."
try {
    Expand-Archive -Path $ArchivePath -DestinationPath $TempDir -Force
} catch {
    Write-Error "Failed to extract AIMT archive: $ArchivePath. $_"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

# Locate aimt.exe
$BinSrc = Get-ChildItem -Path $TempDir -Recurse -Filter "aimt.exe" | Select-Object -First 1
if (-not $BinSrc) {
    # Fallback: look for aimt without extension (unlikely on Windows but handle)
    $BinSrc = Get-ChildItem -Path $TempDir -Recurse -Filter "aimt" | Select-Object -First 1
}
if (-not $BinSrc -or -not (Test-Path $BinSrc.FullName)) {
    Write-Error "Failed to extract AIMT: aimt.exe not found in archive"
    Get-ChildItem -Path $TempDir -Recurse | ForEach-Object { Write-Host $_.FullName }
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

$Dest = Join-Path $InstallDir "aimt.exe"
$TmpDest = "$Dest.tmp"

Write-Host "Installing to $Dest..."

try {
    Copy-Item -Path $BinSrc.FullName -Destination $TmpDest -Force
    Move-Item -Path $TmpDest -Destination $Dest -Force
} catch {
    Write-Error "Failed to install AIMT to $Dest. $_"
    Remove-Item -Force $TmpDest -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

# Never touch .aimt, ~/.aimt/, credentials, or project files

# Verify installation
Write-Host "Verifying installation..."
if (-not (Test-Path $Dest)) {
    Write-Error "Failed to install AIMT: $Dest not found"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

try {
    $VersionOutput = & $Dest --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "aimt --version exited with $LASTEXITCODE"
    }
    Write-Host "Installed: $VersionOutput"
    # If specific version requested, verify match (best-effort)
    if ($RequestedVersion) {
        $RequestedNorm = $RequestedVersion.TrimStart('v')
        $InstalledNorm = ($VersionOutput | Select-String -Pattern '[0-9]+\.[0-9]+\.[0-9]+' | ForEach-Object { $_.Matches[0].Value } | Select-Object -First 1)
        if ($InstalledNorm -and $InstalledNorm -ne $RequestedNorm) {
            Write-Warning "Installed version ($InstalledNorm) does not match requested version ($RequestedNorm)"
        }
    }
} catch {
    Write-Error "Failed to verify AIMT installation: $Dest --version failed. $_"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}

# PATH handling: update user-level PATH, preserve existing, avoid duplicates, handle empty/case-insensitive
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $UserPath) { $UserPath = "" }

$AlreadyInPath = $false
$PathEntries = $UserPath -split ';' | Where-Object { $_ -ne "" }
foreach ($Entry in $PathEntries) {
    if ($Entry.TrimEnd('\').ToLower() -eq $InstallDir.TrimEnd('\').ToLower()) {
        $AlreadyInPath = $true
        break
    }
}
# Also check machine and process PATH via $env:Path for already effective
if (-not $AlreadyInPath) {
    $ProcessPath = $env:Path
    if ($ProcessPath) {
        $ProcessEntries = $ProcessPath -split ';' | Where-Object { $_ -ne "" }
        foreach ($Entry in $ProcessEntries) {
            if ($Entry.TrimEnd('\').ToLower() -eq $InstallDir.TrimEnd('\').ToLower()) {
                $AlreadyInPath = $true
                break
            }
        }
    }
}

if ($AlreadyInPath) {
    Write-Host "Your PATH already contains $InstallDir."
} else {
    Write-Host "Adding $InstallDir to user PATH..."
    try {
        $NewUserPath = if ($UserPath -and $UserPath.Trim() -ne "") {
            "$UserPath;$InstallDir"
        } else {
            $InstallDir
        }
        [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
        # Also update current process PATH for immediate use
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "Added $InstallDir to user PATH."
    } catch {
        Write-Error "Unable to update user PATH: $_"
        Write-Host "Please add $InstallDir to your PATH manually."
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
        exit 1
    }
}

# Clean up temp
Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "AIMT installed successfully."
Write-Host "Restart your terminal before running aimt."
Write-Host ""
Write-Host "Run: aimt --help"
