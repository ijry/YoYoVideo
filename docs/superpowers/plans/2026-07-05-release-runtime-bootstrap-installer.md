# Release Runtime Bootstrap And Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reproducible release path that bootstraps libmpv runtime files, keeps portable packages working, adds a Windows installer, and records runtime provenance and notices.

**Architecture:** Add a manifest-driven runtime preparation layer before the existing package scripts. Keep `scripts/package.ps1` as the package assembler, strengthen `scripts/verify-package.ps1` as the final package gate, add a Windows NSIS installer wrapper, and keep runtime smoke checks optional so default tests stay fast and network-free.

**Tech Stack:** PowerShell 7 scripts, TOML-like manifest parsed by repository code, GitHub Actions, NSIS for Windows installer generation, Rust/Cargo for optional runtime smoke probing.

## Global Constraints

- This phase is about delivery infrastructure. It does not add new playback features and does not change the runtime behavior of the player.
- This phase excludes macOS `.app` bundling, `.dmg` creation, notarization, or codesigning.
- This phase excludes Linux AppImage, Flatpak, Snap, or distro packaging.
- This phase excludes Windows code signing.
- This phase excludes automatic GitHub Release publishing.
- This phase excludes bundling runtime binaries directly in git.
- This phase excludes replacing legal review for libmpv, FFmpeg, or their dependencies.
- Keep the existing portable archives: `YoYoVideo-windows-x64.zip`, `YoYoVideo-macos-universal.tar.gz`, and `YoYoVideo-linux-x64.tar.gz`.
- Add a Windows installer output named `YoYoVideo-windows-x64-setup.exe`.
- The primary bootstrap script is `scripts/bootstrap-runtime.ps1`.
- The runtime manifest lives at `runtime/manifest.toml`.
- The bootstrap script must support `-Platform windows-x64|macos-universal|linux-x64`, `-Manifest runtime/manifest.toml`, `-DestinationRoot third_party/mpv`, `-DryRun`, `-Force`, and `-AllowUnverifiedOverride`.
- `-DryRun` must not download or modify files.
- Default tests must not require libmpv or network access.
- Runtime binaries remain untracked unless redistribution is explicitly reviewed and approved.

---

## File Structure

- Create `runtime/manifest.toml`: platform runtime entries, required files, provenance text, and environment-backed maintainer archive references.
- Create `scripts/bootstrap-runtime.ps1`: manifest parser, dry-run output, checksum verification, extraction, layout normalization, and required-file validation.
- Create `scripts/test-bootstrap-runtime.ps1`: self-contained fixture tests for dry-run, checksum mismatch, and required-file validation.
- Modify `scripts/package.ps1`: optional runtime bootstrap, actionable missing-runtime guidance, release notes, runtime provenance, and license notice staging.
- Modify `scripts/verify-package.ps1`: require `RELEASE-NOTES.md`, runtime provenance, and strengthened package docs.
- Create `installer/windows/yoyovideo.nsi`: NSIS installer script for the verified Windows package directory.
- Create `scripts/build-installer.ps1`: PowerShell wrapper around `makensis`.
- Create `scripts/smoke-runtime.ps1`: optional runtime backend probe that opens generated WAV media and verifies events.
- Modify `.github/workflows/package.yml`: call bootstrap instead of duplicating archive download commands and upload the Windows installer.
- Modify `docs/development/runtime-dependencies.md`: document manifest, bootstrap, environment overrides, and installer prerequisites.
- Modify `docs/testing/manual-smoke-checklist.md`: add bootstrap, installer, and uninstall smoke checks.

---

### Task 1: Runtime Manifest And Bootstrap Dry-Run

**Files:**
- Create: `runtime/manifest.toml`
- Create: `scripts/bootstrap-runtime.ps1`

**Interfaces:**
- Consumes: existing runtime staging layout under `third_party/mpv/<platform>/`
- Produces: `scripts/bootstrap-runtime.ps1 -Platform <platform> -DryRun` command that prints `Platform`, `Version`, `Source`, `Destination`, and `Required files` without downloading or writing files

- [ ] **Step 1: Write the failing dry-run command**

Run:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun
```

Expected: FAIL because `scripts/bootstrap-runtime.ps1` does not exist.

- [ ] **Step 2: Create `runtime/manifest.toml`**

Create this file exactly:

```toml
[[runtime]]
platform = "windows-x64"
version = "maintainer-windows-x64"
source_url = "env:YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL"
sha256 = "env:YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_SHA256"
archive_format = "zip"
strip_components = 0
destination = "third_party/mpv/windows-x64"
required_files = ["lib/mpv.lib", "bin/mpv-2.dll"]
license_files = ["LICENSE*", "COPYING*", "licenses/*", "doc/*license*"]
notes = "Maintainer-provided normalized Windows x64 libmpv runtime archive."

[[runtime]]
platform = "macos-universal"
version = "maintainer-macos-universal"
source_url = "env:YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL"
sha256 = "env:YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_SHA256"
archive_format = "zip"
strip_components = 0
destination = "third_party/mpv/macos-universal"
required_files = ["lib/libmpv.dylib"]
license_files = ["LICENSE*", "COPYING*", "licenses/*", "doc/*license*"]
notes = "Maintainer-provided normalized macOS universal libmpv runtime archive."

[[runtime]]
platform = "linux-x64"
version = "maintainer-linux-x64"
source_url = "env:YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL"
sha256 = "env:YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_SHA256"
archive_format = "zip"
strip_components = 0
destination = "third_party/mpv/linux-x64"
required_files = ["lib/libmpv.so*"]
license_files = ["LICENSE*", "COPYING*", "licenses/*", "doc/*license*"]
notes = "Maintainer-provided normalized Linux x64 libmpv runtime archive."
```

- [ ] **Step 3: Create dry-run capable `scripts/bootstrap-runtime.ps1`**

Create this script:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [string]$Manifest = "runtime/manifest.toml",

    [string]$DestinationRoot = "third_party/mpv",

    [switch]$DryRun,

    [switch]$Force,

    [switch]$AllowUnverifiedOverride
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

function Parse-ManifestValue([string]$Value) {
    $trimmed = $Value.Trim()
    if ($trimmed.StartsWith("[") -and $trimmed.EndsWith("]")) {
        $inner = $trimmed.Substring(1, $trimmed.Length - 2).Trim()
        if ([string]::IsNullOrWhiteSpace($inner)) {
            return @()
        }
        return @($inner -split "," | ForEach-Object {
            $_.Trim().Trim('"')
        })
    }
    if ($trimmed.StartsWith('"') -and $trimmed.EndsWith('"')) {
        return $trimmed.Trim('"')
    }
    if ($trimmed -match '^\d+$') {
        return [int]$trimmed
    }
    return $trimmed
}

function Read-RuntimeManifest([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Runtime manifest not found at $Path"
    }

    $entries = @()
    $current = $null
    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) {
            continue
        }
        if ($trimmed -eq "[[runtime]]") {
            if ($null -ne $current) {
                $entries += [pscustomobject]$current
            }
            $current = @{}
            continue
        }
        if ($null -eq $current) {
            Fail "Manifest key appears before [[runtime]]: $trimmed"
        }
        if ($trimmed -notmatch '^([A-Za-z0-9_]+)\s*=\s*(.+)$') {
            Fail "Unsupported manifest line: $trimmed"
        }
        $current[$matches[1]] = Parse-ManifestValue $matches[2]
    }
    if ($null -ne $current) {
        $entries += [pscustomobject]$current
    }
    return $entries
}

function Resolve-ManifestToken([string]$Value, [switch]$RequiredForDryRun) {
    if ($Value -like "env:*") {
        $name = $Value.Substring(4)
        $resolved = [Environment]::GetEnvironmentVariable($name)
        if ([string]::IsNullOrWhiteSpace($resolved)) {
            if ($RequiredForDryRun) {
                return "<requires $name>"
            }
            Fail "Runtime manifest value requires environment variable $name"
        }
        return $resolved
    }
    return $Value
}

function Get-RuntimeEntry([object[]]$Entries, [string]$Platform) {
    $entry = $Entries | Where-Object { $_.platform -eq $Platform } | Select-Object -First 1
    if ($null -eq $entry) {
        Fail "No runtime manifest entry for $Platform"
    }
    return $entry
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$manifestPath = if ([System.IO.Path]::IsPathRooted($Manifest)) {
    $Manifest
} else {
    Join-Path $repoRoot $Manifest
}

$entries = Read-RuntimeManifest $manifestPath
$entry = Get-RuntimeEntry $entries $Platform
$sourceUrl = Resolve-ManifestToken $entry.source_url -RequiredForDryRun:$DryRun
$sha256 = Resolve-ManifestToken $entry.sha256 -RequiredForDryRun:$DryRun
$destination = if ([System.IO.Path]::IsPathRooted($entry.destination)) {
    $entry.destination
} else {
    Join-Path $repoRoot $entry.destination
}

if ($DryRun) {
    Write-Host "Runtime bootstrap dry run"
    Write-Host "Platform: $($entry.platform)"
    Write-Host "Version: $($entry.version)"
    Write-Host "Source: $sourceUrl"
    Write-Host "SHA256: $sha256"
    Write-Host "Archive format: $($entry.archive_format)"
    Write-Host "Destination: $destination"
    Write-Host "Required files:"
    foreach ($file in @($entry.required_files)) {
        Write-Host "  - $file"
    }
    exit 0
}

Fail "Runtime bootstrap download and extraction are added in Task 2. Dry-run is available now."
```

- [ ] **Step 4: Verify dry-run succeeds without runtime environment variables**

Run:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun
```

Expected: PASS and output contains:

```text
Runtime bootstrap dry run
Platform: windows-x64
Source: <requires YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL>
Required files:
  - lib/mpv.lib
  - bin/mpv-2.dll
```

- [ ] **Step 5: Commit**

```powershell
git add runtime/manifest.toml scripts/bootstrap-runtime.ps1
git commit -m "feat: add runtime manifest dry run"
```

---

### Task 2: Runtime Bootstrap Download, Checksum, Extraction, And Fixture Tests

**Files:**
- Modify: `scripts/bootstrap-runtime.ps1`
- Create: `scripts/test-bootstrap-runtime.ps1`

**Interfaces:**
- Consumes: Task 1 manifest schema and dry-run entry selection
- Produces: `scripts/bootstrap-runtime.ps1` that downloads `file://`, `http://`, or `https://` archives, verifies SHA-256, extracts zip archives for tests, normalizes output to `third_party/mpv/<platform>/`, and validates required files

- [ ] **Step 1: Write the failing fixture test script**

Create `scripts/test-bootstrap-runtime.ps1`:

```powershell
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-ExpectSuccess([scriptblock]$Command, [string]$Message) {
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw $Message
    }
}

function Invoke-ExpectFailure([scriptblock]$Command, [string]$ExpectedText) {
    $output = & $Command 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "Expected command to fail: $ExpectedText"
    }
    if (($output | Out-String) -notmatch [regex]::Escape($ExpectedText)) {
        throw "Expected failure containing '$ExpectedText'. Actual output: $output"
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-bootstrap-test-" + [Guid]::NewGuid())
$fixtureRoot = Join-Path $tempRoot "fixture"
$archivePath = Join-Path $tempRoot "runtime.zip"
$manifestPath = Join-Path $tempRoot "manifest.toml"
$destinationRoot = Join-Path $tempRoot "stage"

New-Item -ItemType Directory -Force (Join-Path $fixtureRoot "lib"), (Join-Path $fixtureRoot "bin"), (Join-Path $fixtureRoot "licenses") | Out-Null
Set-Content -LiteralPath (Join-Path $fixtureRoot "lib/mpv.lib") -Value "fixture import library"
Set-Content -LiteralPath (Join-Path $fixtureRoot "bin/mpv-2.dll") -Value "fixture runtime dll"
Set-Content -LiteralPath (Join-Path $fixtureRoot "licenses/LICENSE.txt") -Value "fixture license"
Compress-Archive -Path (Join-Path $fixtureRoot "*") -DestinationPath $archivePath -Force

$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
Set-Content -LiteralPath $manifestPath -Value @"
[[runtime]]
platform = "windows-x64"
version = "fixture"
source_url = "file:///$($archivePath.Replace('\', '/'))"
sha256 = "$hash"
archive_format = "zip"
strip_components = 0
destination = "windows-x64"
required_files = ["lib/mpv.lib", "bin/mpv-2.dll"]
license_files = ["licenses/LICENSE.txt"]
notes = "Fixture runtime archive."
"@

Invoke-ExpectSuccess {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/bootstrap-runtime.ps1") -Platform windows-x64 -Manifest $manifestPath -DestinationRoot $destinationRoot -Force
} "bootstrap fixture should succeed"

Assert-True (Test-Path -LiteralPath (Join-Path $destinationRoot "windows-x64/lib/mpv.lib") -PathType Leaf) "mpv.lib was not staged"
Assert-True (Test-Path -LiteralPath (Join-Path $destinationRoot "windows-x64/bin/mpv-2.dll") -PathType Leaf) "mpv-2.dll was not staged"

Set-Content -LiteralPath $manifestPath -Value ((Get-Content -Raw -LiteralPath $manifestPath) -replace $hash, ("0" * 64))
Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/bootstrap-runtime.ps1") -Platform windows-x64 -Manifest $manifestPath -DestinationRoot $destinationRoot -Force
} "Checksum mismatch"

Remove-Item -LiteralPath $tempRoot -Recurse -Force
Write-Host "bootstrap fixture tests passed"
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```powershell
pwsh -NoProfile -File scripts/test-bootstrap-runtime.ps1
```

Expected: FAIL with `Runtime bootstrap download and extraction are added in Task 2`.

- [ ] **Step 3: Extend `scripts/bootstrap-runtime.ps1` with download, checksum, extraction, and required-file validation**

Add these functions above the final non-dry-run path:

```powershell
function Get-CachePath([string]$RepoRoot, [string]$Platform, [string]$Version, [string]$SourceUrl) {
    $cacheDir = Join-Path $RepoRoot ".cache/runtime"
    New-Item -ItemType Directory -Force $cacheDir | Out-Null
    $extension = if ($SourceUrl -match '\.tar\.gz($|\?)') {
        ".tar.gz"
    } elseif ($SourceUrl -match '\.tar\.xz($|\?)') {
        ".tar.xz"
    } elseif ($SourceUrl -match '\.7z($|\?)') {
        ".7z"
    } else {
        ".zip"
    }
    return Join-Path $cacheDir "$Platform-$Version$extension"
}

function Copy-Or-DownloadArchive([string]$SourceUrl, [string]$DestinationPath, [switch]$Force) {
    if ((Test-Path -LiteralPath $DestinationPath -PathType Leaf) -and -not $Force) {
        return
    }
    if ($SourceUrl.StartsWith("file:///")) {
        $localPath = ([System.Uri]$SourceUrl).LocalPath
        Copy-Item -LiteralPath $localPath -Destination $DestinationPath -Force
        return
    }
    if ($SourceUrl -notmatch '^https?://') {
        Fail "Unsupported runtime source URL: $SourceUrl"
    }
    Invoke-WebRequest -Uri $SourceUrl -OutFile $DestinationPath
}

function Assert-Checksum([string]$Path, [string]$ExpectedSha256, [switch]$AllowUnverifiedOverride) {
    if ([string]::IsNullOrWhiteSpace($ExpectedSha256) -or $ExpectedSha256 -like "<requires *") {
        if ($AllowUnverifiedOverride) {
            Write-Warning "Skipping runtime checksum verification because -AllowUnverifiedOverride was supplied."
            return
        }
        Fail "Runtime checksum is required for $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    $expected = $ExpectedSha256.ToLowerInvariant()
    if ($actual -ne $expected) {
        Fail "Checksum mismatch for $Path. Expected $expected but got $actual"
    }
}

function Expand-RuntimeArchive([string]$ArchivePath, [string]$ArchiveFormat, [string]$ExtractDir) {
    if (Test-Path -LiteralPath $ExtractDir) {
        Remove-Item -LiteralPath $ExtractDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force $ExtractDir | Out-Null
    switch ($ArchiveFormat) {
        "zip" {
            Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDir -Force
        }
        "7z" {
            $sevenZip = Get-Command 7z -ErrorAction SilentlyContinue
            if ($null -eq $sevenZip) {
                Fail "7z archive extraction requires the 7z command on PATH"
            }
            & $sevenZip.Source x "-o$ExtractDir" $ArchivePath -y | Out-Host
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        "tar.gz" {
            & tar -xzf $ArchivePath -C $ExtractDir
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        "tar.xz" {
            & tar -xJf $ArchivePath -C $ExtractDir
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        default {
            Fail "Unsupported archive_format: $ArchiveFormat"
        }
    }
}

function Copy-NormalizedRuntime([string]$ExtractDir, [string]$Destination, [int]$StripComponents, [switch]$Force) {
    if ((Test-Path -LiteralPath $Destination) -and $Force) {
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }
    New-Item -ItemType Directory -Force $Destination | Out-Null
    $sourceRoot = $ExtractDir
    for ($index = 0; $index -lt $StripComponents; $index++) {
        $children = @(Get-ChildItem -LiteralPath $sourceRoot -Directory)
        if ($children.Count -ne 1) {
            Fail "Cannot strip component $($index + 1) from $ExtractDir because the directory shape is ambiguous"
        }
        $sourceRoot = $children[0].FullName
    }
    Get-ChildItem -LiteralPath $sourceRoot -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Assert-RequiredFiles([string]$Destination, [object[]]$RequiredFiles) {
    foreach ($file in @($RequiredFiles)) {
        $pattern = Join-Path $Destination $file
        $matches = @(Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue)
        if ($matches.Count -eq 0) {
            Fail "Required runtime file missing after extraction: $file under $Destination"
        }
    }
}
```

Replace the final `Fail "Runtime bootstrap download and extraction are added in Task 2..."` line with:

```powershell
$destinationRootPath = if ([System.IO.Path]::IsPathRooted($DestinationRoot)) {
    $DestinationRoot
} else {
    Join-Path $repoRoot $DestinationRoot
}
$destination = Join-Path $destinationRootPath $Platform
$cachePath = Get-CachePath $repoRoot $Platform $entry.version $sourceUrl
$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-runtime-" + $Platform + "-" + [Guid]::NewGuid())

Write-Host "Bootstrapping runtime for $Platform"
Write-Host "Source: $sourceUrl"
Write-Host "Archive: $cachePath"
Write-Host "Destination: $destination"

Copy-Or-DownloadArchive $sourceUrl $cachePath -Force:$Force
Assert-Checksum $cachePath $sha256 -AllowUnverifiedOverride:$AllowUnverifiedOverride
Expand-RuntimeArchive $cachePath $entry.archive_format $extractDir
Copy-NormalizedRuntime $extractDir $destination ([int]$entry.strip_components) -Force:$Force
Assert-RequiredFiles $destination @($entry.required_files)
Remove-Item -LiteralPath $extractDir -Recurse -Force

Write-Host "Runtime bootstrap complete"
Write-Host "Platform: $($entry.platform)"
Write-Host "Version: $($entry.version)"
Write-Host "Destination: $destination"
```

- [ ] **Step 4: Run fixture tests**

Run:

```powershell
pwsh -NoProfile -File scripts/test-bootstrap-runtime.ps1
```

Expected: PASS with `bootstrap fixture tests passed`.

- [ ] **Step 5: Run manifest dry-run for all platforms**

Run:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform macos-universal -DryRun
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform linux-x64 -DryRun
```

Expected: all PASS and each output contains the selected platform and required files.

- [ ] **Step 6: Commit**

```powershell
git add scripts/bootstrap-runtime.ps1 scripts/test-bootstrap-runtime.ps1
git commit -m "feat: implement runtime bootstrap validation"
```

---

### Task 3: Package Script Bootstrap Integration And Release Metadata

**Files:**
- Modify: `scripts/package.ps1`
- Create: `docs/release/RELEASE-NOTES.template.md`
- Create: `docs/release/LICENSES-README.md`

**Interfaces:**
- Consumes: `scripts/bootstrap-runtime.ps1 -Platform <platform>`
- Produces: `scripts/package.ps1 -RequireRuntime -BootstrapRuntime` and package files `RELEASE-NOTES.md`, `LICENSES/README.md`, and `LICENSES/runtime-provenance.md`

- [ ] **Step 1: Write the failing package command**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -BootstrapRuntime -SkipBuild
```

Expected: FAIL with PowerShell parameter binding error because `-BootstrapRuntime` does not exist.

- [ ] **Step 2: Create release templates**

Create `docs/release/RELEASE-NOTES.template.md`:

```markdown
# YoYoVideo Release Notes

- Version: {{VERSION}}
- Platform: {{PLATFORM}}
- Build date UTC: {{BUILD_DATE_UTC}}
- Runtime version: {{RUNTIME_VERSION}}

## Included Artifacts

- Portable package for {{PLATFORM}}
- Bundled runtime files staged from the runtime manifest
- Runtime provenance and license notice scaffolding

## Known Limitations

- Runtime redistribution still requires review of the exact libmpv and FFmpeg build.
- Platform signing and store distribution are outside this release phase.
```

Create `docs/release/LICENSES-README.md`:

```markdown
# License Notices

This package includes YoYoVideo and platform runtime files needed for libmpv playback.

The bundled runtime provenance is recorded in `runtime-provenance.md`.

Before public redistribution, review the exact libmpv, FFmpeg, codec, and dependency licenses for the runtime archive used to build this package.
```

- [ ] **Step 3: Modify `scripts/package.ps1` parameters**

Change the param block to include:

```powershell
    [switch]$RequireRuntime,

    [switch]$BootstrapRuntime,

    [string]$ReleaseVersion = "dev",

    [switch]$ReleaseMode,

    [switch]$AllowMissingRuntimeLicenseFiles,

    [switch]$SkipBuild
```

- [ ] **Step 4: Add package metadata helper functions**

Add these functions after `Copy-DirectoryFiles`:

```powershell
function Read-RuntimeEntrySummary([string]$Platform) {
    $manifest = Join-Path $repoRoot "runtime/manifest.toml"
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
        return [pscustomobject]@{
            Version = "unknown"
            Source = "unknown"
            Sha256 = "unknown"
            Notes = "Runtime manifest not found."
        }
    }
    $content = Get-Content -Raw -LiteralPath $manifest
    $blocks = $content -split '\[\[runtime\]\]' | Where-Object { $_ -match 'platform\s*=\s*"' }
    foreach ($block in $blocks) {
        if ($block -match 'platform\s*=\s*"' + [regex]::Escape($Platform) + '"') {
            $version = if ($block -match 'version\s*=\s*"([^"]+)"') { $matches[1] } else { "unknown" }
            $source = if ($block -match 'source_url\s*=\s*"([^"]+)"') { $matches[1] } else { "unknown" }
            $sha = if ($block -match 'sha256\s*=\s*"([^"]+)"') { $matches[1] } else { "unknown" }
            $notes = if ($block -match 'notes\s*=\s*"([^"]+)"') { $matches[1] } else { "" }
            return [pscustomobject]@{
                Version = $version
                Source = $source
                Sha256 = $sha
                Notes = $notes
            }
        }
    }
    return [pscustomobject]@{
        Version = "unknown"
        Source = "unknown"
        Sha256 = "unknown"
        Notes = "Runtime manifest entry not found."
    }
}

function Write-ReleaseMetadata([string]$PackageDir, [string]$Platform, [string]$ReleaseVersion) {
    $runtime = Read-RuntimeEntrySummary $Platform
    $buildDate = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
    $templatePath = Join-Path $repoRoot "docs/release/RELEASE-NOTES.template.md"
    $releaseNotes = Get-Content -Raw -LiteralPath $templatePath
    $releaseNotes = $releaseNotes.Replace("{{VERSION}}", $ReleaseVersion)
    $releaseNotes = $releaseNotes.Replace("{{PLATFORM}}", $Platform)
    $releaseNotes = $releaseNotes.Replace("{{BUILD_DATE_UTC}}", $buildDate)
    $releaseNotes = $releaseNotes.Replace("{{RUNTIME_VERSION}}", $runtime.Version)
    Set-Content -LiteralPath (Join-Path $PackageDir "RELEASE-NOTES.md") -Value $releaseNotes

    Copy-Item -LiteralPath (Join-Path $repoRoot "docs/release/LICENSES-README.md") -Destination (Join-Path $PackageDir "LICENSES/README.md") -Force
    Set-Content -LiteralPath (Join-Path $PackageDir "LICENSES/runtime-provenance.md") -Value @"
# Runtime Provenance

- Platform: $Platform
- Runtime version: $($runtime.Version)
- Runtime source: $($runtime.Source)
- Runtime SHA-256: $($runtime.Sha256)
- Package build date UTC: $buildDate

$($runtime.Notes)

Public redistribution requires review of the exact libmpv, FFmpeg, codec, and dependency licenses for this runtime build.
"@
}
```

- [ ] **Step 5: Add bootstrap invocation before runtime validation**

After `$binaryPath = ...`, add:

```powershell
if ($RequireRuntime -and $BootstrapRuntime) {
    $bootstrapScript = Join-Path $repoRoot "scripts/bootstrap-runtime.ps1"
    & pwsh -NoProfile -File $bootstrapScript -Platform $Platform
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
```

- [ ] **Step 6: Improve missing runtime message**

In `Require-File`, replace the failure line with:

```powershell
        if ($Description -like "*runtime*" -or $Description -like "*mpv*") {
            Fail "Missing $Description at $Path. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform $Platform"
        }
        Fail "Missing $Description at $Path"
```

In `Require-Directory`, replace the failure line with:

```powershell
        if ($Description -like "*runtime*") {
            Fail "Missing $Description at $Path. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform $Platform"
        }
        Fail "Missing $Description at $Path"
```

- [ ] **Step 7: Replace inline license README generation**

Delete the existing `Set-Content -Path (Join-Path $packageDir "LICENSES/README.md") -Value @"...` block and call metadata generation after docs are copied:

```powershell
Write-ReleaseMetadata $packageDir $Platform $ReleaseVersion
```

- [ ] **Step 8: Verify missing runtime guidance**

Run in a temporary destination where runtime files are absent:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform macos-universal -Configuration debug -RequireRuntime -SkipBuild
```

Expected: FAIL with text containing:

```text
Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform macos-universal
```

- [ ] **Step 9: Verify Windows package still builds with staged local runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild
```

Expected: PASS and package includes `RELEASE-NOTES.md` and `LICENSES/runtime-provenance.md`.

- [ ] **Step 10: Commit**

```powershell
git add scripts/package.ps1 docs/release/RELEASE-NOTES.template.md docs/release/LICENSES-README.md
git commit -m "feat: add release metadata to packages"
```

---

### Task 4: Package Verification Strengthening

**Files:**
- Modify: `scripts/verify-package.ps1`

**Interfaces:**
- Consumes: Task 3 package metadata files
- Produces: verification failure when `RELEASE-NOTES.md`, `LICENSES/README.md`, or `LICENSES/runtime-provenance.md` is missing

- [ ] **Step 1: Write failing verification scenario**

Run:

```powershell
$package = Join-Path $env:TEMP ("yoyovideo-verify-missing-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force "$package/bin", "$package/docs", "$package/LICENSES" | Out-Null
New-Item -ItemType File -Force "$package/bin/yoyovideo-desktop.exe", "$package/README.md", "$package/docs/runtime-dependencies.md", "$package/docs/manual-smoke-checklist.md" | Out-Null
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir $package
```

Expected: PASS before this task, proving verification does not yet require release metadata.

- [ ] **Step 2: Require release metadata files**

After the existing package README check, add:

```powershell
Require-File (Join-Path $PackageDir "RELEASE-NOTES.md") "package release notes"
Require-File (Join-Path $PackageDir "LICENSES/README.md") "package license notice README"
Require-File (Join-Path $PackageDir "LICENSES/runtime-provenance.md") "runtime provenance notice"
```

- [ ] **Step 3: Verify the missing metadata case fails**

Run the command from Step 1 again.

Expected: FAIL with:

```text
Missing package release notes
```

- [ ] **Step 4: Verify real Windows package passes**

Run:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Expected: PASS with:

```text
Verified YoYoVideo package:
Runtime required: True
```

- [ ] **Step 5: Commit**

```powershell
git add scripts/verify-package.ps1
git commit -m "test: require release metadata in package verification"
```

---

### Task 5: Windows NSIS Installer

**Files:**
- Create: `installer/windows/yoyovideo.nsi`
- Create: `scripts/build-installer.ps1`

**Interfaces:**
- Consumes: verified Windows package directory `dist/YoYoVideo-windows-x64`
- Produces: `dist/YoYoVideo-windows-x64-setup.exe` when `makensis` is installed

- [ ] **Step 1: Write failing installer command**

Run:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
```

Expected: FAIL because `scripts/build-installer.ps1` does not exist.

- [ ] **Step 2: Create NSIS script**

Create `installer/windows/yoyovideo.nsi`:

```nsis
!ifndef PACKAGE_DIR
  !error "PACKAGE_DIR is required"
!endif

!ifndef OUTPUT_EXE
  !error "OUTPUT_EXE is required"
!endif

!ifndef APP_VERSION
  !define APP_VERSION "dev"
!endif

Name "YoYoVideo"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\YoYoVideo"
RequestExecutionLevel user
Unicode true

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${PACKAGE_DIR}\*"
  CreateDirectory "$SMPROGRAMS\YoYoVideo"
  CreateShortcut "$SMPROGRAMS\YoYoVideo\YoYoVideo.lnk" "$INSTDIR\bin\yoyovideo-desktop.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "DisplayName" "YoYoVideo"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "InstallLocation" "$INSTDIR"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\YoYoVideo\YoYoVideo.lnk"
  RMDir "$SMPROGRAMS\YoYoVideo"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo"
  RMDir /r "$INSTDIR"
SectionEnd
```

- [ ] **Step 3: Create installer wrapper**

Create `scripts/build-installer.ps1`:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageDir,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [string]$Version = "dev"
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$packageFullPath = [System.IO.Path]::GetFullPath($PackageDir)
$outputFullPath = [System.IO.Path]::GetFullPath($OutputPath)
$scriptPath = Join-Path $repoRoot "installer/windows/yoyovideo.nsi"

if (-not (Test-Path -LiteralPath $packageFullPath -PathType Container)) {
    Fail "Package directory not found: $packageFullPath"
}
if (-not (Test-Path -LiteralPath (Join-Path $packageFullPath "bin/yoyovideo-desktop.exe") -PathType Leaf)) {
    Fail "Windows package must contain bin/yoyovideo-desktop.exe before installer generation"
}
if (-not (Test-Path -LiteralPath $scriptPath -PathType Leaf)) {
    Fail "NSIS script not found: $scriptPath"
}

$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if ($null -eq $makensis) {
    Fail "NSIS makensis command not found. Install NSIS, then retry: pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir $PackageDir -OutputPath $OutputPath -Version $Version"
}

$outputDir = Split-Path -Parent $outputFullPath
New-Item -ItemType Directory -Force $outputDir | Out-Null

& $makensis.Source "/DPACKAGE_DIR=$packageFullPath" "/DOUTPUT_EXE=$outputFullPath" "/DAPP_VERSION=$Version" $scriptPath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $outputFullPath -PathType Leaf)) {
    Fail "Installer was not created at $outputFullPath"
}

Write-Host "Created installer: $outputFullPath"
```

- [ ] **Step 4: Verify missing NSIS message or installer creation**

Run:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
```

Expected if NSIS is absent: FAIL with `NSIS makensis command not found`.

Expected if NSIS is installed: PASS and creates `dist/YoYoVideo-windows-x64-setup.exe`.

- [ ] **Step 5: Commit**

```powershell
git add installer/windows/yoyovideo.nsi scripts/build-installer.ps1
git commit -m "feat: add windows installer builder"
```

---

### Task 6: Optional Runtime Smoke Script

**Files:**
- Create: `scripts/smoke-runtime.ps1`

**Interfaces:**
- Consumes: staged runtime files under `third_party/mpv/<platform>/`
- Produces: optional smoke command that creates a WAV file, runs a temporary Rust probe, opens media through `build_desktop_backend()`, and requires `DurationChanged`, `TracksChanged`, and `PositionChanged`

- [ ] **Step 1: Write failing smoke command**

Run:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64
```

Expected: FAIL because `scripts/smoke-runtime.ps1` does not exist.

- [ ] **Step 2: Create runtime smoke script**

Create `scripts/smoke-runtime.ps1`:

```powershell
[CmdletBinding()]
param(
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform = "windows-x64",

    [int]$TimeoutSeconds = 5
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$runtimeBin = Join-Path $repoRoot "third_party/mpv/$Platform/bin"
$runtimeLib = Join-Path $repoRoot "third_party/mpv/$Platform/lib"

if ($Platform -eq "windows-x64" -and -not (Test-Path -LiteralPath (Join-Path $runtimeBin "mpv-2.dll") -PathType Leaf)) {
    Fail "Missing Windows runtime DLL. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64"
}

$probeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-runtime-smoke-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Force (Join-Path $probeRoot "src") | Out-Null

Set-Content -LiteralPath (Join-Path $probeRoot "Cargo.toml") -Value @"
[package]
name = "yoyovideo-runtime-smoke"
version = "0.1.0"
edition = "2024"

[dependencies]
yoyovideo_desktop = { package = "yoyovideo-desktop", path = "$(($repoRoot.Path -replace '\\', '/') + '/apps/yoyovideo-desktop')", features = ["mpv-runtime"] }
yoyo_core = { package = "yoyo-core", path = "$(($repoRoot.Path -replace '\\', '/') + '/crates/yoyo-core')" }
"@

Set-Content -LiteralPath (Join-Path $probeRoot "src/main.rs") -Value @'
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use yoyo_core::{BackendEvent, MediaLocator, PlayerBackend};

fn main() {
    let media = std::env::temp_dir().join("yoyovideo-smoke.wav");
    write_wav(&media);
    let mut backend = yoyovideo_desktop::build_desktop_backend().expect("backend init");
    backend.open(&MediaLocator::File(media)).expect("open media");
    let start = Instant::now();
    let mut duration = false;
    let mut position = false;
    let mut tracks = false;
    let mut errors = Vec::new();
    while start.elapsed() < Duration::from_secs(5) {
        for event in backend.drain_events() {
            println!("event={event:?}");
            match event {
                BackendEvent::DurationChanged(Some(value)) if value > 0.0 => duration = true,
                BackendEvent::PositionChanged(value) if value >= 0.0 => position = true,
                BackendEvent::TracksChanged { audio, subtitles: _, video: _ } if !audio.is_empty() => tracks = true,
                BackendEvent::Error(message) => errors.push(message),
                _ => {}
            }
        }
        if duration && position && tracks {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    if !errors.is_empty() {
        panic!("backend errors: {errors:?}");
    }
    if !(duration && position && tracks) {
        panic!("missing expected events: duration={duration} position={position} tracks={tracks}");
    }
    println!("runtime_smoke=ok");
}

fn write_wav(path: &PathBuf) {
    let sample_rate = 44_100u32;
    let seconds = 2u32;
    let samples = sample_rate * seconds;
    let data_size = samples * 2;
    let mut file = File::create(path).expect("wav");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    for index in 0..samples {
        let t = index as f32 / sample_rate as f32;
        let sample = ((t * 440.0 * std::f32::consts::TAU).sin() * 3000.0) as i16;
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}
'@

if ($Platform -eq "windows-x64") {
    $env:PATH = "$runtimeBin;$env:PATH"
}
if ($Platform -eq "macos-universal") {
    $env:DYLD_LIBRARY_PATH = "$runtimeLib;$env:DYLD_LIBRARY_PATH"
}
if ($Platform -eq "linux-x64") {
    $env:LD_LIBRARY_PATH = "$runtimeLib;$env:LD_LIBRARY_PATH"
}

Push-Location $probeRoot
try {
    & cargo run
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
    Remove-Item -LiteralPath $probeRoot -Recurse -Force -ErrorAction SilentlyContinue
}
```

- [ ] **Step 3: Run runtime smoke if local Windows runtime is staged**

Run:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64
```

Expected when runtime is staged: PASS with `runtime_smoke=ok`.

Expected when runtime is absent: FAIL with `Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64`.

- [ ] **Step 4: Commit**

```powershell
git add scripts/smoke-runtime.ps1
git commit -m "test: add optional runtime smoke probe"
```

---

### Task 7: GitHub Actions Package Workflow Bootstrap And Installer Upload

**Files:**
- Modify: `.github/workflows/package.yml`

**Interfaces:**
- Consumes: `scripts/bootstrap-runtime.ps1`, `scripts/package.ps1`, and `scripts/build-installer.ps1`
- Produces: package workflow that uses the bootstrap script for each platform and uploads the Windows installer artifact

- [ ] **Step 1: Inspect current workflow references**

Run:

```powershell
Select-String -Path .github/workflows/package.yml -Pattern "Download maintainer runtime archive","Expand-Archive","build-installer","setup.exe"
```

Expected: current workflow contains `Download maintainer runtime archive` and `Expand-Archive`, but does not contain `build-installer` or `setup.exe`.

- [ ] **Step 2: Update workflow secrets and bootstrap steps**

For each job, add SHA environment variables and replace the manual download step with dry-run plus bootstrap.

Windows job environment should be:

```yaml
    env:
      YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL }}
      YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_SHA256: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_SHA256 }}
```

Windows steps after Rust install should be:

```yaml
      - name: Validate runtime manifest
        shell: pwsh
        run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun

      - name: Bootstrap runtime
        shell: pwsh
        run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force

      - name: Build package
        shell: pwsh
        run: pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime -ReleaseMode -ReleaseVersion $env:GITHUB_REF_NAME

      - name: Install NSIS
        shell: pwsh
        run: choco install nsis -y

      - name: Build installer
        shell: pwsh
        run: pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version $env:GITHUB_REF_NAME
```

macOS job environment should be:

```yaml
    env:
      YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL }}
      YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_SHA256: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_SHA256 }}
```

Linux job environment should be:

```yaml
    env:
      YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL }}
      YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_SHA256: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_SHA256 }}
```

For macOS and Linux, use equivalent dry-run, bootstrap, and package commands with their platform names.

- [ ] **Step 3: Add Windows installer upload**

Add this upload step after the Windows package archive upload:

```yaml
      - name: Upload installer
        uses: actions/upload-artifact@v7
        with:
          name: YoYoVideo-windows-x64-setup
          path: dist/YoYoVideo-windows-x64-setup.exe
          if-no-files-found: error
```

- [ ] **Step 4: Verify workflow text**

Run:

```powershell
Select-String -Path .github/workflows/package.yml -Pattern "bootstrap-runtime.ps1","build-installer.ps1","YoYoVideo-windows-x64-setup","SHA256"
```

Expected: PASS with matches for all four patterns.

- [ ] **Step 5: Commit**

```powershell
git add .github/workflows/package.yml
git commit -m "ci: bootstrap runtime during packaging"
```

---

### Task 8: Docs, Final Verification, And Release Checklist Updates

**Files:**
- Modify: `docs/development/runtime-dependencies.md`
- Modify: `docs/testing/manual-smoke-checklist.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: all previous task commands and artifacts
- Produces: user-facing instructions for runtime bootstrap, portable package creation, Windows installer creation, installer smoke checks, and runtime smoke checks

- [ ] **Step 1: Update runtime dependency docs**

Add this section to `docs/development/runtime-dependencies.md` after "Runtime Staging For Packages":

````markdown
## Runtime Bootstrap

Runtime archives are described by `runtime/manifest.toml`.

Validate the manifest without downloading:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun
````

Prepare local runtime files:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force
```

Maintainer-provided archives are referenced through environment variables:

- `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL`
- `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_SHA256`
- `YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL`
- `YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_SHA256`
- `YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL`
- `YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_SHA256`

The archive must expand into the normalized platform layout expected under `third_party/mpv/<platform>/`.
```

Add this section near package commands:

````markdown
## Windows Installer

After creating a verified Windows package, build the NSIS installer:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
````

If `makensis` is missing, install NSIS and rerun the command. Installer generation is separate from portable zip creation.
```

- [ ] **Step 2: Update manual smoke checklist**

Add these bullets under `Package Artifacts`:

```markdown
- Run `pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun` and confirm it prints the manifest entry without downloading.
- Run `pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force` on a clean machine with maintainer runtime environment variables set.
- Confirm `RELEASE-NOTES.md`, `LICENSES/README.md`, and `LICENSES/runtime-provenance.md` are present in the package.
- Build `dist/YoYoVideo-windows-x64-setup.exe` with `scripts/build-installer.ps1` when NSIS is installed.
- Install the Windows setup package and launch YoYoVideo from the Start Menu shortcut.
- Uninstall YoYoVideo and confirm the installed directory is removed.
- Run `pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64` when runtime files are staged.
```

- [ ] **Step 3: Update README package commands**

In `README.md`, replace the first paragraph under `Packaging` with:

```markdown
Runtime-enabled packages are built from repository-local staging files under `third_party/mpv/<platform>/`. Prepare those files with the runtime bootstrap script before packaging.
```

Add this command before local package commands:

```powershell
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -Force
```

Add this installer note after the `verify-package.ps1` example:

````markdown
On Windows, build an optional NSIS installer after the portable package is verified:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
````
```

- [ ] **Step 4: Run final static checks**

Run:

```powershell
cargo fmt --check
cargo test
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64 -DryRun
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform macos-universal -DryRun
pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform linux-x64 -DryRun
pwsh -NoProfile -File scripts/test-bootstrap-runtime.ps1
```

Expected: all PASS.

- [ ] **Step 5: Run package verification with staged Windows runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild -ReleaseVersion dev
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Expected: both PASS. Package includes runtime DLL, release notes, runtime provenance, docs, and license notice scaffolding.

- [ ] **Step 6: Run optional installer or smoke validation if tools/runtime are available**

Run when NSIS is installed:

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe -Version dev
```

Expected: PASS and creates `dist/YoYoVideo-windows-x64-setup.exe`.

Run when Windows runtime files are staged:

```powershell
pwsh -NoProfile -File scripts/smoke-runtime.ps1 -Platform windows-x64
```

Expected: PASS with `runtime_smoke=ok`.

- [ ] **Step 7: Commit**

```powershell
git add README.md docs/development/runtime-dependencies.md docs/testing/manual-smoke-checklist.md
git commit -m "docs: document runtime bootstrap release flow"
```

---

## Self-Review

**Spec coverage:** This plan covers runtime manifest, bootstrap dry-run, checksum validation, archive extraction, required-file validation, package bootstrap integration, missing-runtime guidance, release notes, runtime provenance, license notice scaffolding, strengthened package verification, Windows NSIS installer generation, GitHub Actions bootstrap reuse, Windows installer upload, optional runtime smoke testing, and docs updates. macOS `.app`, Linux AppImage, signing, GitHub Release publishing, and legal approval remain out of scope as required.

**Placeholder scan:** The plan uses concrete file paths, commands, expected outputs, script parameters, function names, and commit messages. It avoids unresolved implementation markers and keeps fixture-based validation local and deterministic.

**Type consistency:** Platform identifiers are consistently `windows-x64`, `macos-universal`, and `linux-x64`. Script switches match the spec: `-Platform`, `-Manifest`, `-DestinationRoot`, `-DryRun`, `-Force`, and `-AllowUnverifiedOverride`. Package additions use `-BootstrapRuntime`, `-ReleaseVersion`, `-ReleaseMode`, and `-AllowMissingRuntimeLicenseFiles` consistently across tasks.
