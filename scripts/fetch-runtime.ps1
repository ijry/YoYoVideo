<#
.SYNOPSIS
    Stages the bundled libmpv playback core for one platform.

.DESCRIPTION
    Reads runtime/manifest.toml, downloads the pinned public archive, verifies its
    SHA-256, and normalizes it into the third_party/mpv/<platform>/ layout that
    scripts/package.ps1 expects:

        <destination>/bin/...   runtime payload shipped next to the binary
        <destination>/lib/...   import library / link-time library

    On Windows the upstream mpv-dev archive contains a MinGW import library
    (libmpv.dll.a) and a DLL named libmpv-2.dll. MSVC link.exe cannot consume the
    former, and an executable linked against libmpv.dll.a records "libmpv-2.dll" as
    its import name, so a DLL shipped under any other name fails to load at
    startup. This script therefore renames the DLL to mpv-2.dll and regenerates an
    MSVC import library from that DLL's own export table, which keeps the
    executable, the import library, and the shipped DLL in agreement.

.PARAMETER Platform
    windows-x64, macos-universal, or linux-x64.

.PARAMETER DestinationRoot
    Overrides the manifest `destination` field. Mainly for tests.

.PARAMETER Force
    Re-download and re-stage even when the destination already looks complete.

.EXAMPLE
    pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [string]$Manifest = "runtime/manifest.toml",

    [string]$DestinationRoot,

    [switch]$DryRun,

    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

# Mirrors scripts/bootstrap-runtime.ps1 so both scripts read the same manifest
# dialect, but tolerates a key whose value is a bare `true`/`false`.
function Parse-ManifestValue([string]$Value) {
    $trimmed = $Value.Trim()
    if ($trimmed.StartsWith("[") -and $trimmed.EndsWith("]")) {
        $inner = $trimmed.Substring(1, $trimmed.Length - 2).Trim()
        if ([string]::IsNullOrWhiteSpace($inner)) {
            return @()
        }
        return @($inner -split "," | ForEach-Object { $_.Trim().Trim('"') })
    }
    if ($trimmed.StartsWith('"') -and $trimmed.EndsWith('"')) {
        return $trimmed.Trim('"')
    }
    if ($trimmed -match '^\d+$') {
        return [int]$trimmed
    }
    if ($trimmed -eq "true") { return $true }
    if ($trimmed -eq "false") { return $false }
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

function Get-RuntimeEntry([object[]]$Entries, [string]$Name) {
    $entry = $Entries | Where-Object { $_.platform -eq $Name } | Select-Object -First 1
    if ($null -eq $entry) {
        Fail "No runtime manifest entry for $Name"
    }
    if ($entry.available -ne $true) {
        Fail "Runtime manifest entry for $Name is not marked available = true. $($entry.notes)"
    }
    return $entry
}

function Get-SevenZipCommand {
    $command = Get-Command 7z.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }
    foreach ($candidate in @(
            "C:\Program Files\7-Zip\7z.exe",
            "C:\Program Files (x86)\7-Zip\7z.exe",
            "C:\ProgramData\chocolatey\bin\7z.exe")) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }
    Fail "7z is required to expand a .7z runtime archive but was not found on PATH"
}

function Expand-RuntimeArchive([string]$ArchivePath, [string]$ArchiveFormat, [string]$ExtractDir) {
    New-Item -ItemType Directory -Force $ExtractDir | Out-Null
    switch ($ArchiveFormat) {
        "zip" {
            Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDir -Force
        }
        "7z" {
            & (Get-SevenZipCommand) x $ArchivePath "-o$ExtractDir" -y | Out-Null
            if ($LASTEXITCODE -ne 0) {
                Fail "7z failed to expand $ArchivePath (exit $LASTEXITCODE)"
            }
        }
        "tar.gz" {
            & tar -xzf $ArchivePath -C $ExtractDir
            if ($LASTEXITCODE -ne 0) { Fail "tar failed to expand $ArchivePath" }
        }
        "tar.xz" {
            & tar -xJf $ArchivePath -C $ExtractDir
            if ($LASTEXITCODE -ne 0) { Fail "tar failed to expand $ArchivePath" }
        }
        default {
            Fail "Unsupported archive_format: $ArchiveFormat"
        }
    }
}

# Locates the MSVC x64 tool binaries that ship with Visual Studio / the Build Tools.
# vswhere is the supported way to find the installation; the hardcoded path is only
# a fallback for images where vswhere is not registered.
function Get-MsvcToolPath([string]$ToolName) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    $roots = @()
    if (Test-Path -LiteralPath $vswhere) {
        $installation = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($installation) {
            $roots += $installation
        }
    }
    $roots += @(
        "C:\Program Files\Microsoft Visual Studio\2022\Enterprise",
        "C:\Program Files\Microsoft Visual Studio\2022\Professional",
        "C:\Program Files\Microsoft Visual Studio\2022\Community",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Enterprise",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Professional",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community",
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
    )

    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root -PathType Container)) {
            continue
        }
        $toolsRoot = Join-Path $root "VC\Tools\MSVC"
        if (-not (Test-Path -LiteralPath $toolsRoot -PathType Container)) {
            continue
        }
        $candidates = Get-ChildItem -LiteralPath $toolsRoot -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending
        foreach ($candidate in $candidates) {
            $path = Join-Path $candidate.FullName "bin\Hostx64\x64\$ToolName"
            if (Test-Path -LiteralPath $path -PathType Leaf) {
                return $path
            }
        }
    }
    Fail "$ToolName was not found. Install the Visual Studio Build Tools with the C++ workload."
}

function New-MsvcImportLibrary([string]$DllPath, [string]$LibraryName, [string]$OutputPath) {
    # lib.exe cannot read a DLL's exports directly, so dumpbin produces the export
    # table and the rows are rewritten into a module-definition file.
    $dumpbin = Get-MsvcToolPath "dumpbin.exe"
    $lib = Get-MsvcToolPath "lib.exe"

    $work = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-def-" + [Guid]::NewGuid())
    New-Item -ItemType Directory -Force $work | Out-Null
    try {
        $dump = Join-Path $work "exports.txt"
        & $dumpbin /nologo /exports $DllPath > $dump
        if ($LASTEXITCODE -ne 0) {
            Fail "dumpbin could not read exports from $DllPath"
        }

        $lines = Get-Content -LiteralPath $dump
        $headerIndex = ($lines | Select-String -Pattern '^\s+ordinal hint RVA\s+name' | Select-Object -First 1).LineNumber
        if (-not $headerIndex) {
            Fail "Could not locate the export table header in the dumpbin output for $DllPath"
        }

        $exports = @($lines[$headerIndex..($lines.Count - 1)] |
                Where-Object { $_ -match '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+\S+' } |
                ForEach-Object { ($_.Trim() -split '\s+')[3] })
        if ($exports.Count -eq 0) {
            Fail "No exports found in $DllPath; refusing to emit an empty import library"
        }

        # The LIBRARY name is what the executable records as its import DLL name,
        # so it has to match the file name the package actually ships.
        $definitionPath = Join-Path $work "$LibraryName.def"
        @("LIBRARY $LibraryName", "EXPORTS") + $exports |
            Set-Content -LiteralPath $definitionPath -Encoding ascii

        & $lib /nologo "/def:$definitionPath" "/out:$OutputPath" /machine:X64 | Out-Null
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $OutputPath -PathType Leaf)) {
            Fail "lib.exe could not build an import library for $DllPath"
        }
    } finally {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Assert-Checksum([string]$Path, [string]$ExpectedSha256) {
    if ([string]::IsNullOrWhiteSpace($ExpectedSha256) -or $ExpectedSha256 -like "<requires *") {
        Fail "A pinned sha256 is required to verify $Path"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
        Fail "Checksum mismatch for $Path. Expected $ExpectedSha256 but got $actual"
    }
}

function Assert-RequiredFiles([string]$Destination, [object[]]$RequiredFiles) {
    foreach ($file in @($RequiredFiles)) {
        $matches = @(Get-ChildItem -Path (Join-Path $Destination $file) -File -ErrorAction SilentlyContinue)
        if ($matches.Count -eq 0) {
            Fail "Required runtime file missing after staging: $file under $Destination"
        }
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$manifestPath = if ([System.IO.Path]::IsPathRooted($Manifest)) {
    $Manifest
} else {
    Join-Path $repoRoot $Manifest
}

$entry = Get-RuntimeEntry (Read-RuntimeManifest $manifestPath) $Platform
$sourceUrl = Resolve-ManifestToken $entry.source_url -RequiredForDryRun:$DryRun
$sha256 = Resolve-ManifestToken $entry.sha256 -RequiredForDryRun:$DryRun

$destination = if (-not [string]::IsNullOrWhiteSpace($DestinationRoot)) {
    if ([System.IO.Path]::IsPathRooted($DestinationRoot)) {
        Join-Path $DestinationRoot $Platform
    } else {
        Join-Path $repoRoot (Join-Path $DestinationRoot $Platform)
    }
} elseif ([System.IO.Path]::IsPathRooted($entry.destination)) {
    $entry.destination
} else {
    Join-Path $repoRoot $entry.destination
}

if ($DryRun) {
    Write-Host "Runtime fetch dry run"
    Write-Host "Platform:    $($entry.platform)"
    Write-Host "Version:     $($entry.version)"
    Write-Host "Source:      $sourceUrl"
    Write-Host "SHA256:      $sha256"
    Write-Host "Format:      $($entry.archive_format)"
    Write-Host "Destination: $destination"
    Write-Host "Required files:"
    foreach ($file in @($entry.required_files)) {
        Write-Host "  - $file"
    }
    Write-Host $entry.notes
    exit 0
}

$extension = switch ($entry.archive_format) {
    "7z" { ".7z" }
    "tar.gz" { ".tar.gz" }
    "tar.xz" { ".tar.xz" }
    default { ".zip" }
}

$cacheDir = Join-Path $repoRoot ".cache/runtime"
New-Item -ItemType Directory -Force $cacheDir | Out-Null
$archivePath = Join-Path $cacheDir "$Platform-$($entry.version)$extension"

Write-Host "Fetching runtime for $Platform"
Write-Host "Source: $sourceUrl"

if ($Force -or -not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
    $tempArchive = "$archivePath.part"
    Invoke-WebRequest -Uri $sourceUrl -OutFile $tempArchive
    Move-Item -LiteralPath $tempArchive -Destination $archivePath -Force
}

Assert-Checksum $archivePath $sha256

$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-fetch-" + $Platform + "-" + [Guid]::NewGuid())
try {
    Expand-RuntimeArchive $archivePath $entry.archive_format $extractDir

    $binDir = Join-Path $destination "bin"
    $libDir = Join-Path $destination "lib"
    New-Item -ItemType Directory -Force $binDir, $libDir | Out-Null

    # Clear stale payloads without touching the tracked placeholders. .gitkeep is
    # what keeps an otherwise-empty staging directory in git, and the per-platform
    # README documents the layout — neither is generated, so neither is deleted.
    # Only files are removed: the bin/ and lib/ directories themselves are the
    # contract that package.ps1 and verify-package.ps1 rely on.
    foreach ($stagedDir in @($binDir, $libDir)) {
        Get-ChildItem -LiteralPath $stagedDir -Force -File |
            Where-Object { $_.Name -ne ".gitkeep" } |
            ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }
    }
    Get-ChildItem -LiteralPath $destination -Force -File |
        Where-Object { $_.Name -ne ".gitkeep" -and $_.Extension -ne ".md" } |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }

    if ($Platform -eq "windows-x64") {
        $upstreamDll = Get-ChildItem -LiteralPath $extractDir -Filter "libmpv-2.dll" -File -Recurse |
            Select-Object -First 1
        if ($null -eq $upstreamDll) {
            Fail "The runtime archive does not contain libmpv-2.dll"
        }

        # Ship under the name the import library records, and import library name
        # is what the executable records: see New-MsvcImportLibrary.
        $stagedDll = Join-Path $binDir "mpv-2.dll"
        Copy-Item -LiteralPath $upstreamDll.FullName -Destination $stagedDll -Force
        New-MsvcImportLibrary $stagedDll "mpv-2" (Join-Path $libDir "mpv.lib")
    } else {
        # Untouched normalization for platforms whose archive already has the
        # expected layout: everything lands under lib/.
        Get-ChildItem -LiteralPath $extractDir -Force | ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $libDir -Recurse -Force
        }
    }

    Assert-RequiredFiles $destination @($entry.required_files)
} finally {
    Remove-Item -LiteralPath $extractDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Runtime staged"
Write-Host "Platform:    $Platform"
Write-Host "Version:     $($entry.version)"
Write-Host "Destination: $destination"
