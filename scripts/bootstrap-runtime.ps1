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

function Get-CachePath(
    [string]$RepoRoot,
    [string]$Platform,
    [string]$Version,
    [string]$SourceUrl
) {
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

function Assert-Checksum(
    [string]$Path,
    [string]$ExpectedSha256,
    [switch]$AllowUnverifiedOverride
) {
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

function Clear-RuntimeDestination([string]$Destination) {
    if (-not (Test-Path -LiteralPath $Destination -PathType Container)) {
        return
    }

    Get-ChildItem -LiteralPath $Destination -Force | Where-Object {
        $_.Name -ne ".gitkeep"
    } | ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Recurse -Force
    }
}

function Copy-NormalizedRuntime(
    [string]$ExtractDir,
    [string]$Destination,
    [int]$StripComponents,
    [switch]$Force
) {
    if ($Force) {
        Clear-RuntimeDestination $Destination
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
