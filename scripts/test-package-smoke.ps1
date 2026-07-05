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
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-package-smoke-test-" + [Guid]::NewGuid())
$packageDir = Join-Path $tempRoot "YoYoVideo-windows-x64"

$fixtureDirs = @(
    (Join-Path $packageDir "bin"),
    (Join-Path $packageDir "docs"),
    (Join-Path $packageDir "LICENSES")
)
New-Item -ItemType Directory -Force -Path $fixtureDirs | Out-Null

Set-Content -LiteralPath (Join-Path $packageDir "README.md") -Value "readme"
Set-Content -LiteralPath (Join-Path $packageDir "RELEASE-NOTES.md") -Value "release"
Set-Content -LiteralPath (Join-Path $packageDir "LICENSES/README.md") -Value "licenses"
Set-Content -LiteralPath (Join-Path $packageDir "LICENSES/runtime-provenance.md") -Value "runtime"
Set-Content -LiteralPath (Join-Path $packageDir "docs/runtime-dependencies.md") -Value "runtime docs"
Set-Content -LiteralPath (Join-Path $packageDir "docs/manual-smoke-checklist.md") -Value "smoke"

Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "Missing desktop binary"

Set-Content -LiteralPath (Join-Path $packageDir "bin/yoyovideo-desktop.exe") -Value "fixture exe"
Invoke-ExpectFailure {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "Missing Windows libmpv runtime DLL"

Set-Content -LiteralPath (Join-Path $packageDir "bin/mpv-2.dll") -Value "fixture dll"
Invoke-ExpectSuccess {
    pwsh -NoProfile -File (Join-Path $repoRoot "scripts/smoke-package.ps1") -Platform windows-x64 -PackageDir $packageDir -RequireRuntime -SkipLaunch -SkipRuntimePlayback
} "package smoke fixture should succeed when required files exist"

$logPath = Join-Path $packageDir "smoke/package-smoke.log"
Assert-True (Test-Path -LiteralPath $logPath -PathType Leaf) "smoke log was not created"
Assert-True ((Get-Content -Raw -LiteralPath $logPath) -match "package_smoke=ok") "smoke log did not record success"

Remove-Item -LiteralPath $tempRoot -Recurse -Force
Write-Host "package smoke fixture tests passed"
