[CmdletBinding()]
param(
    # Media to open. Several files open as a grid (batch playback).
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Media
)

$ErrorActionPreference = "Stop"

# Builds and runs the desktop app with real playback enabled.
#
# Uses a target directory of its own, because a plain `cargo test` / `cargo build`
# rebuilds the same binary WITHOUT `mpv-runtime` and silently replaces it. The app then
# starts with "Playback runtime is disabled in this build" and creates no video surface,
# which looks exactly like a regression. `mpv-runtime` cannot simply be made a default
# feature: docs/development/runtime-dependencies.md requires default `cargo test` to work
# without libmpv present.

$repoRoot = Split-Path -Parent $PSScriptRoot
$targetDir = Join-Path $repoRoot "target-mpv"
$runtimeBin = Join-Path $repoRoot "third_party/mpv/windows-x64/bin/mpv-2.dll"

if (-not (Test-Path -LiteralPath $runtimeBin)) {
    throw "Missing $runtimeBin. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64"
}

Push-Location $repoRoot
try {
    & cargo build -p yoyovideo-desktop --features mpv-runtime --target-dir $targetDir
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

$exe = Join-Path $targetDir "debug/yoyovideo-desktop.exe"
$exeDir = Split-Path -Parent $exe
$stagedRuntime = Join-Path $exeDir "mpv-2.dll"

# libmpv is loaded at runtime, so it has to sit next to the binary. Skip the copy when it
# is already staged: a running instance holds the DLL open, which would fail the copy.
$needsCopy = -not (Test-Path -LiteralPath $stagedRuntime) -or
    ((Get-Item -LiteralPath $stagedRuntime).Length -ne (Get-Item -LiteralPath $runtimeBin).Length)
if ($needsCopy) {
    Copy-Item -LiteralPath $runtimeBin -Destination $exeDir -Force
}

Write-Host "Running $exe"
if ($Media) {
    & $exe @Media
} else {
    & $exe
}
