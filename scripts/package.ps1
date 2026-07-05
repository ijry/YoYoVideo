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
