[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-aarch64", "macos-x86_64", "linux-x64")]
    [string]$Platform,

    [ValidateSet("debug", "release")]
    [string]$Configuration = "release",

    [switch]$RequireRuntime,

    [switch]$BootstrapRuntime,

    [string]$ReleaseVersion = "dev",

    [switch]$ReleaseMode,

    [switch]$AllowMissingRuntimeLicenseFiles,

    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        if ($Description -like "*runtime*" -or $Description -like "*mpv*") {
            Fail "Missing $Description at $Path. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform $Platform"
        }
        Fail "Missing $Description at $Path"
    }
}

function Require-Directory([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        if ($Description -like "*runtime*") {
            Fail "Missing $Description at $Path. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform $Platform"
        }
        Fail "Missing $Description at $Path"
    }
}

function Require-Glob([string]$Pattern, [string]$Description) {
    $matches = @(Get-ChildItem -Path $Pattern -File -ErrorAction SilentlyContinue)
    if ($matches.Count -eq 0) {
        if ($Description -like "*runtime*" -or $Description -like "*mpv*") {
            Fail "Missing $Description matching $Pattern. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform $Platform"
        }
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

function Read-ResolvedRuntimeSource([string]$Platform) {
    # The system-library platforms have no upstream archive to pin a SHA-256
    # against, so fetch-runtime.ps1 records what it actually resolved. Surface it
    # here rather than leaving the package claiming a version it cannot name.
    $path = Join-Path $repoRoot "third_party/mpv/$Platform/runtime-source.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return $null
    }
    return Get-Content -Raw -LiteralPath $path
}

function Write-ReleaseMetadata([string]$PackageDir, [string]$Platform, [string]$ReleaseVersion) {
    $runtime = Read-RuntimeEntrySummary $Platform
    $resolved = Read-ResolvedRuntimeSource $Platform
    $buildDate = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")

    # Computed before the here-string below: anything assigned inside @"..."@ is
    # literal text, not an interpolated expression.
    $resolvedBlock = if ($resolved) {
@"
Resolved runtime source, recorded at build time (see runtime-source.json):

$resolved
"@
    } else {
        "Resolved runtime source: pinned upstream archive; see the URL and SHA-256 above."
    }

    $templatePath = Join-Path $repoRoot "docs/release/RELEASE-NOTES.template.md"
    $releaseNotes = Get-Content -Raw -LiteralPath $templatePath
    $releaseNotes = $releaseNotes.Replace("{{VERSION}}", $ReleaseVersion)
    $releaseNotes = $releaseNotes.Replace("{{PLATFORM}}", $Platform)
    $releaseNotes = $releaseNotes.Replace("{{BUILD_DATE_UTC}}", $buildDate)
    $releaseNotes = $releaseNotes.Replace("{{RUNTIME_VERSION}}", $runtime.Version)
    Set-Content -LiteralPath (Join-Path $PackageDir "RELEASE-NOTES.md") -Value $releaseNotes

    Copy-Item -LiteralPath (Join-Path $repoRoot "docs/release/LICENSES-README.md") -Destination (Join-Path $PackageDir "LICENSES/README.md") -Force
    if ($resolved) {
        Set-Content -LiteralPath (Join-Path $PackageDir "LICENSES/runtime-source.json") -Value $resolved -Encoding utf8
    }
    Set-Content -LiteralPath (Join-Path $PackageDir "LICENSES/runtime-provenance.md") -Value @"
# Runtime Provenance

- Platform: $Platform
- Runtime version: $($runtime.Version)
- Runtime source: $($runtime.Source)
- Runtime SHA-256: $($runtime.Sha256)
- Package build date UTC: $buildDate

$($runtime.Notes)

$resolvedBlock

This runtime is distributed under GPL-2.0-or-later (mpv and FFmpeg). Redistributing this
package requires providing the corresponding source: the YoYoVideo git tag it was built
from, plus the upstream mpv, FFmpeg and runtime build projects named above.
See LICENSES.md in the YoYoVideo source repository for the full details.
"@
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

if ($RequireRuntime -and $BootstrapRuntime) {
    $bootstrapScript = Join-Path $repoRoot "scripts/bootstrap-runtime.ps1"
    & pwsh -NoProfile -File $bootstrapScript -Platform $Platform
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

if ($RequireRuntime) {
    Require-Directory $runtimeRoot "runtime staging directory"
    switch ($Platform) {
        "windows-x64" {
            Require-File (Join-Path $runtimeLibDir "mpv.lib") "Windows mpv import library"
            Require-File (Join-Path $runtimeBinDir "mpv-2.dll") "Windows libmpv runtime DLL"
        }
        { $_ -in @("macos-aarch64", "macos-x86_64") } {
            Require-File (Join-Path $runtimeLibDir "libmpv.dylib") "macOS libmpv dylib"
        }
        "linux-x64" {
            Require-Glob (Join-Path $runtimeLibDir "libmpv.so*") "Linux libmpv shared library" | Out-Null
        }
    }

    $linkFlags = @("-L native=$runtimeLibDir")

    # The packaged executable has to find the bundled libraries beside itself. The
    # loader records the SONAME / install name at link time, so a copy elsewhere on
    # the machine is not enough: the search path has to be baked into the binary.
    # Windows needs none of this -- mpv-2.dll sits next to the .exe and the default
    # search order already covers that.
    if ($Platform -eq "linux-x64") {
        $linkFlags += '-C link-arg=-Wl,-rpath,$ORIGIN'
    }
    if ($Platform -in @("macos-aarch64", "macos-x86_64")) {
        $linkFlags += '-C link-arg=-Wl,-rpath,@loader_path'
    }

    if (Test-Path -LiteralPath $runtimeLibDir -PathType Container) {
        $linkFlagsJoined = $linkFlags -join " "
        $env:RUSTFLAGS = if ([string]::IsNullOrWhiteSpace($env:RUSTFLAGS)) { $linkFlagsJoined } else { "$env:RUSTFLAGS $linkFlagsJoined" }
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

Write-ReleaseMetadata $packageDir $Platform $ReleaseVersion

if ($RequireRuntime) {
    Copy-DirectoryFiles $runtimeBinDir (Join-Path $packageDir "bin")
    # On macOS and Linux the lib/ directory holds the shared library itself, which
    # the package must carry. On Windows lib/ holds only the MSVC import library
    # used at link time; shipping it would put a build artifact in a release and
    # tell users nothing about running the binary.
    if ($Platform -ne "windows-x64") {
        Copy-DirectoryFiles $runtimeLibDir (Join-Path $packageDir "bin")
    }
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
