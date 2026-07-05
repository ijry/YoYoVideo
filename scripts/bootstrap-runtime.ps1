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
