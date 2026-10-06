<#
.SYNOPSIS
    Type-checks the desktop app for the macOS targets, from a non-macOS machine.

.DESCRIPTION
    macOS-only code is easy to accumulate unverified on a Windows or Linux box: the
    normal `cargo check` never looks at it, and the first feedback is a CI round
    trip. `cargo check` does not link, so the only thing it actually needs for a
    foreign target is that target's standard library.

    This script builds a private sysroot (a copy of the installed one plus the
    macOS standard libraries, downloaded from static.rust-lang.org and verified
    against the published hashes) and checks both macOS architectures against it.

    It is not a substitute for building on macOS, and the CI macOS job remains the
    authority. It just moves a whole class of errors from minutes to seconds.

.PARAMETER Target
    Which architecture to check. Defaults to both.

.PARAMETER Refresh
    Re-download the standard libraries even if they are already cached.

.EXAMPLE
    pwsh -NoProfile -File scripts/check-macos.ps1
#>
[CmdletBinding()]
param(
    [ValidateSet("all", "aarch64-apple-darwin", "x86_64-apple-darwin")]
    [string]$Target = "all",

    [switch]$Refresh
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$cacheRoot = Join-Path $repoRoot ".cache/macos-cross"
$sysroot = Join-Path $repoRoot ".cache/macos-cross/sysroot"
$targetDir = Join-Path $repoRoot ".cache/macos-cross/target"

function Get-RustRelease([string]$Version) {
    # The channel manifest carries the release date, which is part of the download
    # URL. Guessing it from the commit date is wrong by a day often enough to matter.
    $manifest = Join-Path $cacheRoot "channel.toml"
    if ($Refresh -or -not (Test-Path -LiteralPath $manifest)) {
        Invoke-WebRequest -Uri "https://static.rust-lang.org/dist/channel-rust-$Version.toml" -OutFile $manifest
    }
    $line = Select-String -Path $manifest -Pattern '^date = "([^"]+)"' | Select-Object -First 1
    if (-not $line) {
        Fail "Could not read the release date out of the $Version channel manifest."
    }
    return $line.Matches[0].Groups[1].Value
}

function Get-ExpectedHash([string]$Manifest, [string]$Target) {
    # [pkg.rust-std.target.<target>] then the xz_hash a couple of lines down.
    $pattern = '(?ms)^\[pkg\.rust-std\.target\.' + [regex]::Escape($Target) + '\]\s*$(.*?)(?=^\[)'
    $match = [regex]::Match((Get-Content -Raw -LiteralPath $Manifest), $pattern)
    if (-not $match.Success) {
        Fail "No rust-std entry for $Target in the channel manifest."
    }
    $hash = [regex]::Match($match.Groups[1].Value, 'xz_hash = "([0-9a-f]+)"')
    if (-not $hash.Success) {
        Fail "No xz_hash for $Target in the channel manifest."
    }
    return $hash.Groups[1].Value
}

$rustcVersion = (& rustc --version)
if ($rustcVersion -notmatch '^rustc (\S+)') {
    Fail "Could not parse the rustc version from '$rustcVersion'."
}
$version = $matches[1]
$hostTriple = (& rustc -vV | Select-String '^host: (.+)$').Matches[0].Groups[1].Value
Write-Host "rustc $version, host $hostTriple"

New-Item -ItemType Directory -Force $cacheRoot, $sysroot | Out-Null
$manifest = Join-Path $cacheRoot "channel.toml"
if ($Refresh -or -not (Test-Path -LiteralPath $manifest)) {
    Invoke-WebRequest -Uri "https://static.rust-lang.org/dist/channel-rust-$version.toml" -OutFile $manifest
}
$release = Get-RustRelease $version
Write-Host "release date $release"

# A sysroot has to carry the host libraries too: build scripts and proc macros are
# compiled for the host even when the crate under test is not.
$sysrootRustlib = Join-Path $sysroot "lib/rustlib"
$hostLibs = Join-Path $sysrootRustlib $hostTriple
if (-not (Test-Path -LiteralPath $hostLibs)) {
    $installed = Join-Path (& rustc --print sysroot) "lib/rustlib"
    $installedHost = Join-Path $installed $hostTriple
    if (-not (Test-Path -LiteralPath $installedHost)) {
        Fail "Could not find the host standard library at $installedHost"
    }
    New-Item -ItemType Directory -Force $sysrootRustlib | Out-Null
    Copy-Item -LiteralPath $installedHost -Destination $sysrootRustlib -Recurse -Force
    $etc = Join-Path $installed "etc"
    if (Test-Path -LiteralPath $etc) {
        Copy-Item -LiteralPath $etc -Destination $sysrootRustlib -Recurse -Force
    }
    Write-Host "seeded the sysroot with the host libraries"
}

$targets = if ($Target -eq "all") {
    @("aarch64-apple-darwin", "x86_64-apple-darwin")
} else {
    @($Target)
}

foreach ($t in $targets) {
    $destination = Join-Path $sysrootRustlib $t
    if ($Refresh -or -not (Test-Path -LiteralPath $destination)) {
        $url = "https://static.rust-lang.org/dist/$release/rust-std-$version-$t.tar.xz"
        $archive = Join-Path $cacheRoot "$t.tar.xz"
        Write-Host "downloading $url"
        Invoke-WebRequest -Uri $url -OutFile $archive

        $expected = Get-ExpectedHash $manifest $t
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
        if ($actual -ne $expected) {
            Fail "Checksum mismatch for $t. Expected $expected but got $actual"
        }

        $extract = Join-Path $cacheRoot "extract/$t"
        New-Item -ItemType Directory -Force $extract | Out-Null
        & tar -xJf $archive -C $extract
        if ($LASTEXITCODE -ne 0) {
            Fail "Could not extract $archive"
        }
        $source = Join-Path $extract "rust-std-$version-$t/rust-std-$t/lib/rustlib/$t"
        if (-not (Test-Path -LiteralPath $source)) {
            Fail "Unexpected archive layout: $source does not exist"
        }
        New-Item -ItemType Directory -Force $destination | Out-Null
        Copy-Item -Path (Join-Path $source "*") -Destination $destination -Recurse -Force
        Write-Host "installed the $t standard library"
    }
}

$failed = @()
foreach ($t in $targets) {
    Write-Host ""
    Write-Host "=== cargo check --target $t ==="
    $env:RUSTFLAGS = "--sysroot=$sysroot"
    $env:CARGO_TARGET_DIR = $targetDir
    & cargo check -p yoyovideo-desktop --target $t --features mpv-runtime
    if ($LASTEXITCODE -ne 0) {
        $failed += $t
    }
}

Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue

if ($failed.Count -gt 0) {
    Fail "cargo check failed for: $($failed -join ', ')"
}
Write-Host ""
Write-Host "macOS cross-checks passed for: $($targets -join ', ')"