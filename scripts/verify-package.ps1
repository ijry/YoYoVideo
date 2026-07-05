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
Require-File (Join-Path $PackageDir "RELEASE-NOTES.md") "package release notes"
Require-File (Join-Path $PackageDir "LICENSES/README.md") "package license notice README"
Require-File (Join-Path $PackageDir "LICENSES/runtime-provenance.md") "runtime provenance notice"
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
