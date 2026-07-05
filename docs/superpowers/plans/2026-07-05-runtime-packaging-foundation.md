# Runtime Packaging Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a repeatable local and GitHub Actions packaging foundation for YoYoVideo runtime-enabled desktop artifacts.

**Architecture:** Keep the player code unchanged and add packaging infrastructure around it. Runtime files are staged under `third_party/mpv/<platform>/`, local and CI package creation share `scripts/package.ps1`, and package validation is centralized in `scripts/verify-package.ps1`.

**Tech Stack:** Rust workspace, Slint desktop app, optional `libmpv-sys` runtime feature, PowerShell 7 scripts, GitHub Actions, zip archives for Windows, tar.gz archives for macOS and Linux.

## Global Constraints

- Supported platform identifiers are exactly `windows-x64`, `macos-universal`, and `linux-x64`.
- Package formats are exactly `.zip` for Windows and `.tar.gz` for macOS and Linux.
- Runtime binary files under `third_party/mpv/**/bin/` and `third_party/mpv/**/lib/` must stay untracked unless redistribution licensing is reviewed and explicitly approved.
- Repository-owned runtime staging tracks README files and `.gitkeep` placeholders only.
- `scripts/package.ps1 -RequireRuntime` must fail non-zero with a concise actionable message when required runtime files are missing.
- Default `cargo test` must continue to work without libmpv.
- Runtime feature checks must include `cargo check -p yoyo-mpv --features mpv-runtime` and `cargo check -p yoyovideo-desktop --features mpv-runtime`.
- This phase does not implement real video embedding, full keyboard routing, settings UI completion, playlist/history UI completion, release upload automation, signing, or official installer creation.

---

## File Structure

- `.gitignore`: ignore generated `dist/` output and unreviewed runtime binaries while allowing README and `.gitkeep` runtime staging files.
- `third_party/mpv/README.md`: root instructions for maintainers staging libmpv link/runtime files and license notices.
- `third_party/mpv/windows-x64/README.md`: Windows-specific expected `mpv.lib`, `mpv-2.dll`, FFmpeg DLL, and license staging notes.
- `third_party/mpv/windows-x64/bin/.gitkeep`: tracked placeholder for Windows runtime DLL staging.
- `third_party/mpv/windows-x64/lib/.gitkeep`: tracked placeholder for Windows import-library staging.
- `third_party/mpv/macos-universal/README.md`: macOS-specific expected `libmpv.dylib` staging notes.
- `third_party/mpv/macos-universal/lib/.gitkeep`: tracked placeholder for macOS runtime/link library staging.
- `third_party/mpv/linux-x64/README.md`: Linux-specific expected `libmpv.so*` staging notes.
- `third_party/mpv/linux-x64/lib/.gitkeep`: tracked placeholder for Linux runtime/link library staging.
- `scripts/verify-package.ps1`: validates package directories without launching the GUI.
- `scripts/package.ps1`: builds the desktop app, copies runtime/docs/license files, validates the package directory, and creates the archive.
- `.github/workflows/ci.yml`: runs formatting, default tests, and runtime feature type checks.
- `.github/workflows/package.yml`: runs platform package jobs and uploads produced archives.
- `README.md`: documents local runtime staging and packaging commands.
- `docs/development/runtime-dependencies.md`: documents platform runtime file requirements and CI packaging expectations.
- `docs/testing/manual-smoke-checklist.md`: documents package inspection and launch smoke checks.

---

### Task 1: Runtime Staging Layout And Ignore Rules

**Files:**
- Modify: `.gitignore`
- Create: `third_party/mpv/README.md`
- Create: `third_party/mpv/windows-x64/README.md`
- Create: `third_party/mpv/windows-x64/bin/.gitkeep`
- Create: `third_party/mpv/windows-x64/lib/.gitkeep`
- Create: `third_party/mpv/macos-universal/README.md`
- Create: `third_party/mpv/macos-universal/lib/.gitkeep`
- Create: `third_party/mpv/linux-x64/README.md`
- Create: `third_party/mpv/linux-x64/lib/.gitkeep`

**Interfaces:**
- Consumes: Existing workspace layout and runtime packaging design.
- Produces: `third_party/mpv/<platform>/` directories that `scripts/package.ps1` will read.

- [ ] **Step 1: Run the failing runtime staging check**

Run:

```powershell
$required = @(
  ".gitignore",
  "third_party/mpv/README.md",
  "third_party/mpv/windows-x64/README.md",
  "third_party/mpv/windows-x64/bin/.gitkeep",
  "third_party/mpv/windows-x64/lib/.gitkeep",
  "third_party/mpv/macos-universal/README.md",
  "third_party/mpv/macos-universal/lib/.gitkeep",
  "third_party/mpv/linux-x64/README.md",
  "third_party/mpv/linux-x64/lib/.gitkeep"
)
$missing = $required | Where-Object { -not (Test-Path -LiteralPath $_) }
if ($missing) {
  Write-Error ("Missing runtime staging files: " + ($missing -join ", "))
  exit 1
}
```

Expected: FAIL listing the missing `third_party/mpv/...` files.

- [ ] **Step 2: Update `.gitignore`**

Append these lines:

```gitignore
dist/

# Runtime staging keeps documentation and placeholders in git, not binary payloads.
third_party/mpv/**/bin/*
third_party/mpv/**/lib/*
!third_party/mpv/**/bin/.gitkeep
!third_party/mpv/**/lib/.gitkeep
```

- [ ] **Step 3: Create the root runtime staging README**

Create `third_party/mpv/README.md`:

```markdown
# libmpv Runtime Staging

This directory is the repository-owned staging layout used by `scripts/package.ps1`.

The repository tracks README files and `.gitkeep` placeholders only. Do not commit libmpv, FFmpeg, or platform runtime binaries until redistribution licensing has been reviewed and explicitly approved.

## Platforms

- `windows-x64`: place `mpv.lib` in `lib/` and runtime DLLs such as `mpv-2.dll` in `bin/`.
- `macos-universal`: place `libmpv.dylib` in `lib/`.
- `linux-x64`: place `libmpv.so` or versioned `libmpv.so*` files in `lib/`.

## Licensing

Before publishing packages externally, record the exact source of the runtime binaries and include license notices for libmpv, FFmpeg, and every bundled dependency in the generated package `LICENSES/` directory.
```

- [ ] **Step 4: Create Windows runtime staging files**

Create `third_party/mpv/windows-x64/README.md`:

```markdown
# Windows x64 libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/mpv.lib`: MSVC import library used by Rust linking when `mpv-runtime` is enabled.
- `bin/mpv-2.dll`: libmpv runtime library loaded by the packaged desktop app.
- `bin/avcodec-*.dll`, `bin/avformat-*.dll`, `bin/avutil-*.dll`, and related FFmpeg/runtime DLLs from the same libmpv build.

Keep binaries untracked until licensing is approved. The package script copies `bin/*` into the package `bin/` directory.
```

Create empty placeholder files:

```powershell
New-Item -ItemType Directory -Force third_party/mpv/windows-x64/bin, third_party/mpv/windows-x64/lib
New-Item -ItemType File -Force third_party/mpv/windows-x64/bin/.gitkeep, third_party/mpv/windows-x64/lib/.gitkeep
```

- [ ] **Step 5: Create macOS runtime staging files**

Create `third_party/mpv/macos-universal/README.md`:

```markdown
# macOS Universal libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/libmpv.dylib`: libmpv library used by Rust linking and copied into the package.

Keep binaries untracked until licensing is approved. The package script copies `lib/*` into the package `bin/` directory for this foundation phase. A later app-bundle phase can move these files into a framework or bundle-specific location.
```

Create empty placeholder files:

```powershell
New-Item -ItemType Directory -Force third_party/mpv/macos-universal/lib
New-Item -ItemType File -Force third_party/mpv/macos-universal/lib/.gitkeep
```

- [ ] **Step 6: Create Linux runtime staging files**

Create `third_party/mpv/linux-x64/README.md`:

```markdown
# Linux x64 libmpv Runtime Files

Expected files for runtime-enabled packaging:

- `lib/libmpv.so` or versioned `lib/libmpv.so*`: libmpv library used by Rust linking and copied into the package.

Keep binaries untracked until licensing is approved. Distribution package dependencies may still be needed for developer builds, but release packages should not silently rely on an unknown user-installed libmpv.
```

Create empty placeholder files:

```powershell
New-Item -ItemType Directory -Force third_party/mpv/linux-x64/lib
New-Item -ItemType File -Force third_party/mpv/linux-x64/lib/.gitkeep
```

- [ ] **Step 7: Run the staging check again**

Run the command from Step 1.

Expected: PASS with no output.

- [ ] **Step 8: Confirm binary ignore rules**

Run:

```powershell
New-Item -ItemType File -Force third_party/mpv/windows-x64/bin/mpv-2.dll
New-Item -ItemType File -Force third_party/mpv/windows-x64/lib/mpv.lib
git status --short
Remove-Item -LiteralPath third_party/mpv/windows-x64/bin/mpv-2.dll
Remove-Item -LiteralPath third_party/mpv/windows-x64/lib/mpv.lib
```

Expected: `git status --short` shows README and `.gitkeep` files but does not show `mpv-2.dll` or `mpv.lib`.

- [ ] **Step 9: Commit**

Run:

```powershell
git add .gitignore third_party/mpv
git commit -m "chore: add mpv runtime staging layout"
```

Expected: Commit succeeds.

---

### Task 2: Package Verification Script

**Files:**
- Create: `scripts/verify-package.ps1`

**Interfaces:**
- Consumes: Generated package directory `dist/YoYoVideo-<platform>/`.
- Produces: A non-GUI validation command used by local developers, `scripts/package.ps1`, and GitHub Actions.
- Produces command contract: `pwsh -NoProfile -File scripts/verify-package.ps1 -Platform <platform> [-PackageDir <path>] [-RequireRuntime]`.

- [ ] **Step 1: Run the failing script existence check**

Run:

```powershell
if (-not (Test-Path -LiteralPath scripts/verify-package.ps1)) {
  Write-Error "scripts/verify-package.ps1 is missing"
  exit 1
}
```

Expected: FAIL with `scripts/verify-package.ps1 is missing`.

- [ ] **Step 2: Create `scripts/verify-package.ps1`**

Create the script with this content:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [string]$PackageDir,

    [switch]$RequireRuntime
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Missing $Description at $Path"
    }
}

function Require-Directory([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        Fail "Missing $Description at $Path"
    }
}

function Require-Glob([string]$Pattern, [string]$Description) {
    $matches = @(Get-ChildItem -Path $Pattern -File -ErrorAction SilentlyContinue)
    if ($matches.Count -eq 0) {
        Fail "Missing $Description matching $Pattern"
    }
}

if ([string]::IsNullOrWhiteSpace($PackageDir)) {
    $PackageDir = Join-Path "dist" "YoYoVideo-$Platform"
}

$PackageDir = [System.IO.Path]::GetFullPath($PackageDir)

Require-Directory $PackageDir "package directory"
Require-Directory (Join-Path $PackageDir "bin") "package bin directory"
Require-Directory (Join-Path $PackageDir "docs") "package docs directory"
Require-Directory (Join-Path $PackageDir "LICENSES") "package LICENSES directory"
Require-File (Join-Path $PackageDir "README.md") "package README"
Require-File (Join-Path $PackageDir "docs/runtime-dependencies.md") "runtime dependency docs"
Require-File (Join-Path $PackageDir "docs/manual-smoke-checklist.md") "manual smoke checklist"

$binaryName = if ($Platform -eq "windows-x64") { "yoyovideo-desktop.exe" } else { "yoyovideo-desktop" }
Require-File (Join-Path $PackageDir "bin/$binaryName") "desktop binary"

if ($RequireRuntime) {
    switch ($Platform) {
        "windows-x64" {
            Require-File (Join-Path $PackageDir "bin/mpv-2.dll") "Windows libmpv runtime DLL"
        }
        "macos-universal" {
            Require-File (Join-Path $PackageDir "bin/libmpv.dylib") "macOS libmpv dylib"
        }
        "linux-x64" {
            Require-Glob (Join-Path $PackageDir "bin/libmpv.so*") "Linux libmpv shared library"
        }
    }
}

Write-Host "Verified YoYoVideo package: $PackageDir"
Write-Host "Platform: $Platform"
Write-Host "Runtime required: $($RequireRuntime.IsPresent)"
```

- [ ] **Step 3: Run script against a missing package**

Run:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64
```

Expected: FAIL with `Missing package directory`.

- [ ] **Step 4: Create a minimal dummy package fixture**

Run:

```powershell
$package = "dist/YoYoVideo-windows-x64"
New-Item -ItemType Directory -Force "$package/bin", "$package/docs", "$package/LICENSES" | Out-Null
New-Item -ItemType File -Force "$package/bin/yoyovideo-desktop.exe" | Out-Null
Set-Content -Path "$package/README.md" -Value "# YoYoVideo package"
Set-Content -Path "$package/docs/runtime-dependencies.md" -Value "# Runtime Dependencies"
Set-Content -Path "$package/docs/manual-smoke-checklist.md" -Value "# Manual Smoke Checklist"
```

- [ ] **Step 5: Verify the dummy package without runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64
```

Expected: PASS and prints `Verified YoYoVideo package`.

- [ ] **Step 6: Verify runtime-required failure**

Run:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Expected: FAIL with `Missing Windows libmpv runtime DLL`.

- [ ] **Step 7: Verify runtime-required success**

Run:

```powershell
New-Item -ItemType File -Force dist/YoYoVideo-windows-x64/bin/mpv-2.dll | Out-Null
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
Remove-Item -Recurse -Force dist/YoYoVideo-windows-x64
```

Expected: PASS and prints `Runtime required: True`.

- [ ] **Step 8: Commit**

Run:

```powershell
git add scripts/verify-package.ps1
git commit -m "chore: add package verification script"
```

Expected: Commit succeeds.

---

### Task 3: Package Creation Script

**Files:**
- Create: `scripts/package.ps1`

**Interfaces:**
- Consumes: `third_party/mpv/<platform>/`, `target/<profile>/yoyovideo-desktop[.exe]`, `README.md`, `docs/development/runtime-dependencies.md`, `docs/testing/manual-smoke-checklist.md`, and `scripts/verify-package.ps1`.
- Produces: `dist/YoYoVideo-<platform>/` and `dist/YoYoVideo-<platform>.zip` or `dist/YoYoVideo-<platform>.tar.gz`.
- Produces command contract: `pwsh -NoProfile -File scripts/package.ps1 -Platform <platform> -Configuration <debug|release> [-RequireRuntime] [-SkipBuild]`.

- [ ] **Step 1: Run the failing script existence check**

Run:

```powershell
if (-not (Test-Path -LiteralPath scripts/package.ps1)) {
  Write-Error "scripts/package.ps1 is missing"
  exit 1
}
```

Expected: FAIL with `scripts/package.ps1 is missing`.

- [ ] **Step 2: Create `scripts/package.ps1`**

Create the script with this content:

```powershell
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [ValidateSet("debug", "release")]
    [string]$Configuration = "release",

    [switch]$RequireRuntime,

    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Missing $Description at $Path"
    }
}

function Require-Directory([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        Fail "Missing $Description at $Path"
    }
}

function Require-Glob([string]$Pattern, [string]$Description) {
    $matches = @(Get-ChildItem -Path $Pattern -File -ErrorAction SilentlyContinue)
    if ($matches.Count -eq 0) {
        Fail "Missing $Description matching $Pattern"
    }
    return $matches
}

function Copy-DirectoryFiles([string]$SourceDir, [string]$DestinationDir) {
    if (-not (Test-Path -LiteralPath $SourceDir -PathType Container)) {
        return
    }

    Get-ChildItem -LiteralPath $SourceDir -File | Where-Object { $_.Name -ne ".gitkeep" } | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $DestinationDir $_.Name) -Force
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$distRoot = Join-Path $repoRoot "dist"
$packageName = "YoYoVideo-$Platform"
$packageDir = Join-Path $distRoot $packageName
$runtimeRoot = Join-Path $repoRoot "third_party/mpv/$Platform"
$runtimeBinDir = Join-Path $runtimeRoot "bin"
$runtimeLibDir = Join-Path $runtimeRoot "lib"
$binaryName = if ($Platform -eq "windows-x64") { "yoyovideo-desktop.exe" } else { "yoyovideo-desktop" }
$profileDir = if ($Configuration -eq "release") { "release" } else { "debug" }
$binaryPath = Join-Path $repoRoot "target/$profileDir/$binaryName"

if ($RequireRuntime) {
    Require-Directory $runtimeRoot "runtime staging directory"
    switch ($Platform) {
        "windows-x64" {
            Require-File (Join-Path $runtimeLibDir "mpv.lib") "Windows mpv import library"
            Require-File (Join-Path $runtimeBinDir "mpv-2.dll") "Windows libmpv runtime DLL"
        }
        "macos-universal" {
            Require-File (Join-Path $runtimeLibDir "libmpv.dylib") "macOS libmpv dylib"
        }
        "linux-x64" {
            Require-Glob (Join-Path $runtimeLibDir "libmpv.so*") "Linux libmpv shared library" | Out-Null
        }
    }

    if (Test-Path -LiteralPath $runtimeLibDir -PathType Container) {
        $linkFlag = "-L native=$runtimeLibDir"
        $env:RUSTFLAGS = if ([string]::IsNullOrWhiteSpace($env:RUSTFLAGS)) { $linkFlag } else { "$env:RUSTFLAGS $linkFlag" }
    }
}

if (-not $SkipBuild) {
    $cargoArgs = @("build", "-p", "yoyovideo-desktop")
    if ($Configuration -eq "release") {
        $cargoArgs += "--release"
    }
    if ($RequireRuntime) {
        $cargoArgs += @("--features", "mpv-runtime")
    }

    Write-Host "Running: cargo $($cargoArgs -join ' ')"
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

Require-File $binaryPath "desktop binary. Build first or omit -SkipBuild"

if (Test-Path -LiteralPath $packageDir) {
    Remove-Item -LiteralPath $packageDir -Recurse -Force
}

New-Item -ItemType Directory -Force $packageDir, (Join-Path $packageDir "bin"), (Join-Path $packageDir "docs"), (Join-Path $packageDir "LICENSES") | Out-Null

Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $packageDir "bin/$binaryName") -Force
Copy-Item -LiteralPath (Join-Path $repoRoot "README.md") -Destination (Join-Path $packageDir "README.md") -Force
Copy-Item -LiteralPath (Join-Path $repoRoot "docs/development/runtime-dependencies.md") -Destination (Join-Path $packageDir "docs/runtime-dependencies.md") -Force
Copy-Item -LiteralPath (Join-Path $repoRoot "docs/testing/manual-smoke-checklist.md") -Destination (Join-Path $packageDir "docs/manual-smoke-checklist.md") -Force

Set-Content -Path (Join-Path $packageDir "LICENSES/README.md") -Value @"
# Runtime License Notices

This package directory is prepared for libmpv runtime files.

Before public redistribution, replace this notice or supplement it with the exact license files for libmpv, FFmpeg, and every bundled runtime dependency.
"@

if ($RequireRuntime) {
    Copy-DirectoryFiles $runtimeBinDir (Join-Path $packageDir "bin")
    Copy-DirectoryFiles $runtimeLibDir (Join-Path $packageDir "bin")
}

$verifyArgs = @("-NoProfile", "-File", (Join-Path $repoRoot "scripts/verify-package.ps1"), "-Platform", $Platform, "-PackageDir", $packageDir)
if ($RequireRuntime) {
    $verifyArgs += "-RequireRuntime"
}
& pwsh @verifyArgs
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$zipPath = Join-Path $distRoot "$packageName.zip"
$tarPath = Join-Path $distRoot "$packageName.tar.gz"
Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $tarPath -Force -ErrorAction SilentlyContinue

if ($Platform -eq "windows-x64") {
    Compress-Archive -Path $packageDir -DestinationPath $zipPath -Force
    Write-Host "Created archive: $zipPath"
} else {
    Push-Location $distRoot
    try {
        & tar -czf "$packageName.tar.gz" $packageName
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
    Write-Host "Created archive: $tarPath"
}

Write-Host "Created package directory: $packageDir"
```

- [ ] **Step 3: Build the default debug binary**

Run:

```powershell
cargo build -p yoyovideo-desktop
```

Expected: PASS and creates `target/debug/yoyovideo-desktop.exe` on Windows or `target/debug/yoyovideo-desktop` on Unix.

- [ ] **Step 4: Create a package without runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -SkipBuild
```

Expected: PASS, creates `dist/YoYoVideo-windows-x64/`, and creates `dist/YoYoVideo-windows-x64.zip`.

- [ ] **Step 5: Verify the package without runtime**

Run:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64
```

Expected: PASS and prints `Verified YoYoVideo package`.

- [ ] **Step 6: Verify missing runtime failure**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild
```

Expected: FAIL before copying files with `Missing Windows mpv import library` or `Missing Windows libmpv runtime DLL`.

- [ ] **Step 7: Confirm generated package output is ignored**

Run:

```powershell
git status --short
```

Expected: `dist/` output does not appear.

- [ ] **Step 8: Commit**

Run:

```powershell
git add scripts/package.ps1
git commit -m "chore: add desktop package script"
```

Expected: Commit succeeds.

---

### Task 4: Continuous Integration Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Existing Cargo workspace.
- Produces: GitHub Actions CI with default Rust checks and runtime feature type checks.

- [ ] **Step 1: Run the failing workflow existence check**

Run:

```powershell
if (-not (Test-Path -LiteralPath .github/workflows/ci.yml)) {
  Write-Error ".github/workflows/ci.yml is missing"
  exit 1
}
```

Expected: FAIL with `.github/workflows/ci.yml is missing`.

- [ ] **Step 2: Create `.github/workflows/ci.yml`**

Create the workflow with this content:

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  fmt-test:
    name: Format and default tests
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Check formatting
        run: cargo fmt --check

      - name: Run default tests
        run: cargo test

  runtime-check:
    name: Runtime feature check (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, macos-latest, ubuntu-latest]
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Linux libmpv development package
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libmpv-dev

      - name: Install macOS mpv package
        if: runner.os == 'macOS'
        run: brew install mpv

      - name: Document Windows runtime-link expectation
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          Write-Host "cargo check does not produce a final linked executable."
          Write-Host "Runtime packaging still requires third_party/mpv/windows-x64/lib/mpv.lib and bin/mpv-2.dll."

      - name: Check yoyo-mpv runtime feature
        run: cargo check -p yoyo-mpv --features mpv-runtime

      - name: Check desktop runtime feature
        run: cargo check -p yoyovideo-desktop --features mpv-runtime
```

- [ ] **Step 3: Validate workflow text exists**

Run:

```powershell
Select-String -Path .github/workflows/ci.yml -Pattern "cargo fmt --check","cargo test","cargo check -p yoyo-mpv --features mpv-runtime","cargo check -p yoyovideo-desktop --features mpv-runtime"
```

Expected: PASS and prints all four matched commands.

- [ ] **Step 4: Run local commands matching CI**

Run:

```powershell
cargo fmt --check
cargo test
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS locally. If a runtime feature check fails only because a platform linker cannot find `mpv`, record the exact failure in the final implementation report and keep CI docs explicit about the missing runtime link dependency.

- [ ] **Step 5: Commit**

Run:

```powershell
git add .github/workflows/ci.yml
git commit -m "ci: add rust runtime checks"
```

Expected: Commit succeeds.

---

### Task 5: GitHub Actions Packaging Workflow

**Files:**
- Create: `.github/workflows/package.yml`

**Interfaces:**
- Consumes: `scripts/package.ps1`, `third_party/mpv/<platform>/`, and optional maintainer-provided runtime archive URLs in repository secrets.
- Produces: Uploaded package archives named `YoYoVideo-windows-x64`, `YoYoVideo-macos-universal`, and `YoYoVideo-linux-x64`.

- [ ] **Step 1: Run the failing workflow existence check**

Run:

```powershell
if (-not (Test-Path -LiteralPath .github/workflows/package.yml)) {
  Write-Error ".github/workflows/package.yml is missing"
  exit 1
}
```

Expected: FAIL with `.github/workflows/package.yml is missing`.

- [ ] **Step 2: Create `.github/workflows/package.yml`**

Create the workflow with this content:

```yaml
name: Package

on:
  workflow_dispatch:
  push:
    tags:
      - "v*"

jobs:
  package-windows:
    name: Package Windows x64
    runs-on: windows-latest
    env:
      RUNTIME_ARCHIVE_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Download maintainer runtime archive
        if: env.RUNTIME_ARCHIVE_URL != ''
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force third_party/mpv/windows-x64 | Out-Null
          Invoke-WebRequest -Uri $env:RUNTIME_ARCHIVE_URL -OutFile runtime-windows-x64.zip
          Expand-Archive -Path runtime-windows-x64.zip -DestinationPath third_party/mpv/windows-x64 -Force

      - name: Build package
        shell: pwsh
        run: pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime

      - name: Upload package archive
        uses: actions/upload-artifact@v4
        with:
          name: YoYoVideo-windows-x64
          path: dist/YoYoVideo-windows-x64.zip
          if-no-files-found: error

  package-macos:
    name: Package macOS universal
    runs-on: macos-latest
    env:
      RUNTIME_ARCHIVE_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Download maintainer runtime archive
        if: env.RUNTIME_ARCHIVE_URL != ''
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force third_party/mpv/macos-universal | Out-Null
          Invoke-WebRequest -Uri $env:RUNTIME_ARCHIVE_URL -OutFile runtime-macos-universal.zip
          Expand-Archive -Path runtime-macos-universal.zip -DestinationPath third_party/mpv/macos-universal -Force

      - name: Build package
        shell: pwsh
        run: pwsh -NoProfile -File scripts/package.ps1 -Platform macos-universal -Configuration release -RequireRuntime

      - name: Upload package archive
        uses: actions/upload-artifact@v4
        with:
          name: YoYoVideo-macos-universal
          path: dist/YoYoVideo-macos-universal.tar.gz
          if-no-files-found: error

  package-linux:
    name: Package Linux x64
    runs-on: ubuntu-latest
    env:
      RUNTIME_ARCHIVE_URL: ${{ secrets.YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Download maintainer runtime archive
        if: env.RUNTIME_ARCHIVE_URL != ''
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force third_party/mpv/linux-x64 | Out-Null
          Invoke-WebRequest -Uri $env:RUNTIME_ARCHIVE_URL -OutFile runtime-linux-x64.zip
          Expand-Archive -Path runtime-linux-x64.zip -DestinationPath third_party/mpv/linux-x64 -Force

      - name: Build package
        shell: pwsh
        run: pwsh -NoProfile -File scripts/package.ps1 -Platform linux-x64 -Configuration release -RequireRuntime

      - name: Upload package archive
        uses: actions/upload-artifact@v4
        with:
          name: YoYoVideo-linux-x64
          path: dist/YoYoVideo-linux-x64.tar.gz
          if-no-files-found: error
```

- [ ] **Step 3: Validate workflow upload paths**

Run:

```powershell
Select-String -Path .github/workflows/package.yml -Pattern "dist/YoYoVideo-windows-x64.zip","dist/YoYoVideo-macos-universal.tar.gz","dist/YoYoVideo-linux-x64.tar.gz"
```

Expected: PASS and prints all three artifact paths.

- [ ] **Step 4: Validate missing-runtime behavior locally**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild
```

Expected: FAIL with a clear missing-runtime message. This proves the workflow will not silently upload incomplete runtime packages when maintainer runtime archives are absent.

- [ ] **Step 5: Commit**

Run:

```powershell
git add .github/workflows/package.yml
git commit -m "ci: add package artifact workflow"
```

Expected: Commit succeeds.

---

### Task 6: Packaging Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/development/runtime-dependencies.md`
- Modify: `docs/testing/manual-smoke-checklist.md`

**Interfaces:**
- Consumes: Runtime staging layout, local package script, package verification script, and GitHub Actions workflow behavior.
- Produces: Maintainer-facing instructions for local packaging, runtime staging, CI packaging secrets, and manual smoke checks.

- [ ] **Step 1: Run the failing documentation coverage check**

Run:

```powershell
$checks = @{
  "README.md" = @("scripts/package.ps1", "third_party/mpv")
  "docs/development/runtime-dependencies.md" = @("YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL", "mpv.lib", "libmpv.dylib", "libmpv.so")
  "docs/testing/manual-smoke-checklist.md" = @("dist/YoYoVideo-", "GitHub Actions artifact")
}

$missing = @()
foreach ($file in $checks.Keys) {
  $content = Get-Content -Raw $file
  foreach ($pattern in $checks[$file]) {
    if ($content -notmatch [regex]::Escape($pattern)) {
      $missing += "$file missing $pattern"
    }
  }
}

if ($missing) {
  Write-Error ($missing -join "; ")
  exit 1
}
```

Expected: FAIL because the new packaging docs are not present yet.

- [ ] **Step 2: Update `README.md`**

Add this section after the Development section:

```markdown
## Packaging

Runtime-enabled packages are built from repository-local staging files under `third_party/mpv/<platform>/`.

Local package commands:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
pwsh -NoProfile -File scripts/package.ps1 -Platform macos-universal -Configuration release -RequireRuntime
pwsh -NoProfile -File scripts/package.ps1 -Platform linux-x64 -Configuration release -RequireRuntime
```

Use `scripts/verify-package.ps1` to validate generated package directories without launching the GUI:

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -RequireRuntime
```

Actual libmpv and FFmpeg runtime binaries are intentionally not committed. Stage them locally or provide maintainer runtime archive URLs to GitHub Actions after licensing review.
```

- [ ] **Step 3: Update `docs/development/runtime-dependencies.md`**

Append this section:

```markdown
## Runtime Staging For Packages

Package creation reads runtime files from `third_party/mpv/<platform>/`.

### Windows x64

- Required link file: `third_party/mpv/windows-x64/lib/mpv.lib`
- Required runtime file: `third_party/mpv/windows-x64/bin/mpv-2.dll`
- Expected additional runtime files: FFmpeg and dependency DLLs from the same libmpv build, copied into `third_party/mpv/windows-x64/bin/`

### macOS Universal

- Required runtime/link file: `third_party/mpv/macos-universal/lib/libmpv.dylib`
- Later app-bundle work should move this into the final bundle layout and configure deterministic loader paths.

### Linux x64

- Required runtime/link file: `third_party/mpv/linux-x64/lib/libmpv.so` or versioned `libmpv.so*`
- Release packages should not silently rely on an unknown user-installed libmpv.

## GitHub Actions Runtime Archives

The packaging workflow can download maintainer-provided runtime archives before running `scripts/package.ps1`.

Supported repository secrets:

- `YOYOVIDEO_RUNTIME_ARCHIVE_WINDOWS_X64_URL`: zip archive whose contents expand into `third_party/mpv/windows-x64/`
- `YOYOVIDEO_RUNTIME_ARCHIVE_MACOS_UNIVERSAL_URL`: zip archive whose contents expand into `third_party/mpv/macos-universal/`
- `YOYOVIDEO_RUNTIME_ARCHIVE_LINUX_X64_URL`: zip archive whose contents expand into `third_party/mpv/linux-x64/`

If a secret is absent or the archive lacks required files, packaging fails with a missing-runtime message instead of uploading an incomplete artifact.
```

- [ ] **Step 4: Update `docs/testing/manual-smoke-checklist.md`**

Append this section:

```markdown
## Package Artifacts

- Download each GitHub Actions artifact: `YoYoVideo-windows-x64`, `YoYoVideo-macos-universal`, and `YoYoVideo-linux-x64`.
- Extract the archive and confirm the top-level directory is named `dist/YoYoVideo-<platform>` locally or `YoYoVideo-<platform>` inside the uploaded archive.
- Confirm `README.md`, `LICENSES/`, `docs/runtime-dependencies.md`, and `docs/manual-smoke-checklist.md` are present.
- Confirm `bin/yoyovideo-desktop.exe` exists on Windows and `bin/yoyovideo-desktop` exists on macOS and Linux.
- For runtime-enabled artifacts, confirm Windows includes `bin/mpv-2.dll`, macOS includes `bin/libmpv.dylib`, and Linux includes `bin/libmpv.so*`.
- Launch the app from the extracted `bin/` directory and run the Playback and UX checks above.
```

- [ ] **Step 5: Run the documentation coverage check again**

Run the command from Step 1.

Expected: PASS with no output.

- [ ] **Step 6: Commit**

Run:

```powershell
git add README.md docs/development/runtime-dependencies.md docs/testing/manual-smoke-checklist.md
git commit -m "docs: document runtime packaging workflow"
```

Expected: Commit succeeds.

---

### Task 7: Final Verification

**Files:**
- Read: All files changed by Tasks 1 through 6.

**Interfaces:**
- Consumes: Runtime staging layout, scripts, workflows, docs.
- Produces: A verified branch ready for the next feature phase.

- [ ] **Step 1: Format and run default tests**

Run:

```powershell
cargo fmt --check
cargo test
```

Expected: PASS. These tests must not require libmpv.

- [ ] **Step 2: Run runtime feature checks**

Run:

```powershell
cargo check -p yoyo-mpv --features mpv-runtime
cargo check -p yoyovideo-desktop --features mpv-runtime
```

Expected: PASS for type checking. Do not run linked runtime tests without staged libmpv link/runtime files.

- [ ] **Step 3: Run package script without runtime**

Run:

```powershell
cargo build -p yoyovideo-desktop
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -SkipBuild
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64
```

Expected: PASS and creates `dist/YoYoVideo-windows-x64.zip`.

- [ ] **Step 4: Run missing-runtime validation**

Run:

```powershell
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration debug -RequireRuntime -SkipBuild
```

Expected: FAIL with `Missing Windows mpv import library` or `Missing Windows libmpv runtime DLL`.

- [ ] **Step 5: Inspect git status**

Run:

```powershell
git status --short
```

Expected: No uncommitted source changes. Ignored `dist/` output may exist on disk but must not appear.

- [ ] **Step 6: Record implementation outcome**

Report these items to the user:

```text
Implemented:
- Runtime staging layout under third_party/mpv/
- Package verification script
- Package creation script
- CI workflow
- GitHub Actions packaging workflow
- Packaging docs and smoke checklist updates

Verified:
- cargo fmt --check
- cargo test
- cargo check -p yoyo-mpv --features mpv-runtime
- cargo check -p yoyovideo-desktop --features mpv-runtime
- package script without runtime
- missing-runtime failure path

Remaining outside this phase:
- Real video surface embedding
- Complete keyboard event routing
- Complete settings, playlist, and history UI
- Signed installers and release upload automation
- Public redistribution after runtime binary licensing review
```

Expected: User understands the packaging foundation is complete but the broader full-feature player objective is not complete.

---

## Self-Review

**Spec coverage:** The plan covers runtime staging directories, package scripts, validation script, CI formatting/default tests, runtime feature checks, package workflow artifact upload, maintainer runtime archive inputs, documentation, missing-runtime failures, and simple zip/tar.gz outputs. Excluded feature work is explicitly preserved as out of scope.

**Placeholder scan:** The plan contains no placeholder implementation steps and no deferred error handling instructions. Later feature work is named only as out-of-scope scope control.

**Type and command consistency:** Platform identifiers, package names, script parameters, archive paths, and runtime file names match across tasks and docs.
