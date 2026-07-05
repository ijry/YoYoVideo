[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-ExpectSuccess([scriptblock]$Command, [string]$Message) {
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw $Message
    }
}

function Invoke-ExpectFailure([scriptblock]$Command, [string]$ExpectedText) {
    $output = & $Command 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "Expected command to fail: $ExpectedText"
    }
    if (($output | Out-String) -notmatch [regex]::Escape($ExpectedText)) {
        throw "Expected failure containing '$ExpectedText'. Actual output: $output"
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-bootstrap-test-" + [Guid]::NewGuid())
$fixtureRoot = Join-Path $tempRoot "fixture"
$archivePath = Join-Path $tempRoot "runtime.zip"
$manifestPath = Join-Path $tempRoot "manifest.toml"
$destinationRoot = Join-Path $tempRoot "stage"

New-Item -ItemType Directory -Force (Join-Path $fixtureRoot "lib"), (Join-Path $fixtureRoot "bin"), (Join-Path $fixtureRoot "licenses") | Out-Null
Set-Content -LiteralPath (Join-Path $fixtureRoot "lib/mpv.lib") -Value "fixture import library"
Set-Content -LiteralPath (Join-Path $fixtureRoot "bin/mpv-2.dll") -Value "fixture runtime dll"
Set-Content -LiteralPath (Join-Path $fixtureRoot "licenses/LICENSE.txt") -Value "fixture license"
Compress-Archive -Path (Join-Path $fixtureRoot "*") -DestinationPath $archivePath -Force

$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
Set-Content -LiteralPath $manifestPath -Value @"
[[runtime]]
platform = "windows-x64"
version = "fixture"
source_url = "file:///$($archivePath.Replace('\', '/'))"
sha256 = "$hash"
archive_format = "zip"
strip_components = 0
destination = "windows-x64"
required_files = ["lib/mpv.lib", "bin/mpv-2.dll"]
license_files = ["licenses/LICENSE.txt"]
notes = "Fixture runtime archive."
"@

Invoke-ExpectSuccess {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/bootstrap-runtime.ps1") -Platform windows-x64 -Manifest $manifestPath -DestinationRoot $destinationRoot -Force
} "bootstrap fixture should succeed"

Assert-True (Test-Path -LiteralPath (Join-Path $destinationRoot "windows-x64/lib/mpv.lib") -PathType Leaf) "mpv.lib was not staged"
Assert-True (Test-Path -LiteralPath (Join-Path $destinationRoot "windows-x64/bin/mpv-2.dll") -PathType Leaf) "mpv-2.dll was not staged"

Set-Content -LiteralPath $manifestPath -Value ((Get-Content -Raw -LiteralPath $manifestPath) -replace $hash, ("0" * 64))
Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/bootstrap-runtime.ps1") -Platform windows-x64 -Manifest $manifestPath -DestinationRoot $destinationRoot -Force
} "Checksum mismatch"

Remove-Item -LiteralPath $tempRoot -Recurse -Force
Write-Host "bootstrap fixture tests passed"
