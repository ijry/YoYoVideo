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
$iconPath = Join-Path $repoRoot "apps/yoyovideo-desktop/assets/icons/yoyovideo.ico"

if (-not (Test-Path -LiteralPath $packageFullPath -PathType Container)) {
    Fail "Package directory not found: $packageFullPath"
}
if (-not (Test-Path -LiteralPath (Join-Path $packageFullPath "bin/yoyovideo-desktop.exe") -PathType Leaf)) {
    Fail "Windows package must contain bin/yoyovideo-desktop.exe before installer generation"
}
if (-not (Test-Path -LiteralPath $scriptPath -PathType Leaf)) {
    Fail "NSIS script not found: $scriptPath"
}
if (-not (Test-Path -LiteralPath $iconPath -PathType Leaf)) {
    Fail "Icon not found: $iconPath. Run: node scripts/generate-icons.mjs"
}

$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if ($null -eq $makensis) {
    Fail "NSIS makensis command not found. Install NSIS, then retry: pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir $PackageDir -OutputPath $OutputPath -Version $Version"
}

$outputDir = Split-Path -Parent $outputFullPath
New-Item -ItemType Directory -Force $outputDir | Out-Null

& $makensis.Source "/DPACKAGE_DIR=$packageFullPath" "/DOUTPUT_EXE=$outputFullPath" "/DAPP_VERSION=$Version" "/DICON_FILE=$iconPath" $scriptPath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $outputFullPath -PathType Leaf)) {
    Fail "Installer was not created at $outputFullPath"
}

Write-Host "Created installer: $outputFullPath"
