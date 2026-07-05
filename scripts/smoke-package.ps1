[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform,

    [Parameter(Mandatory = $true)]
    [string]$PackageDir,

    [switch]$RequireRuntime,

    [int]$TimeoutSeconds = 5,

    [switch]$SkipLaunch,

    [switch]$SkipRuntimePlayback
)

$ErrorActionPreference = "Stop"

function Write-SmokeLog([string]$Message) {
    if ([string]::IsNullOrWhiteSpace($script:SmokeLog)) {
        return
    }
    $parent = Split-Path -Parent $script:SmokeLog
    New-Item -ItemType Directory -Force $parent | Out-Null
    Add-Content -LiteralPath $script:SmokeLog -Value "$([DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")) $Message"
}

function Fail([string]$Message) {
    Write-SmokeLog "ERROR $Message"
    Write-Error $Message
    exit 1
}

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Fail "Missing $Description at $Path"
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$PackageDir = [System.IO.Path]::GetFullPath($PackageDir)
$script:SmokeLog = Join-Path $PackageDir "smoke/package-smoke.log"
$binaryName = if ($Platform -eq "windows-x64") { "yoyovideo-desktop.exe" } else { "yoyovideo-desktop" }
$binaryPath = Join-Path $PackageDir "bin/$binaryName"

Write-SmokeLog "package_smoke=start platform=$Platform package=$PackageDir"

$verifyArgs = @("-NoProfile", "-File", (Join-Path $repoRoot "scripts/verify-package.ps1"), "-Platform", $Platform, "-PackageDir", $PackageDir)
if ($RequireRuntime) {
    $verifyArgs += "-RequireRuntime"
}
& pwsh @verifyArgs
if ($LASTEXITCODE -ne 0) {
    Fail "Package layout verification failed"
}

Require-File $binaryPath "desktop binary"

if (-not $SkipLaunch) {
    Write-SmokeLog "launch=start binary=$binaryPath"
    if ($IsWindows) {
        $process = Start-Process -FilePath $binaryPath -PassThru -WindowStyle Hidden
    } else {
        $process = Start-Process -FilePath $binaryPath -PassThru
    }
    Start-Sleep -Seconds ([Math]::Min($TimeoutSeconds, 3))
    if ($process.HasExited -and $process.ExitCode -ne 0) {
        Fail "Desktop binary exited during launch smoke with code $($process.ExitCode)"
    }
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    Write-SmokeLog "launch=ok"
}

if ($RequireRuntime -and -not $SkipRuntimePlayback) {
    $runtimeBin = Join-Path $PackageDir "bin"
    $smokeArgs = @(
        "-NoProfile",
        "-File",
        (Join-Path $repoRoot "scripts/smoke-runtime.ps1"),
        "-Platform",
        $Platform,
        "-TimeoutSeconds",
        $TimeoutSeconds,
        "-RuntimeBin",
        $runtimeBin,
        "-RuntimeLib",
        $runtimeBin
    )
    $oldPath = $env:PATH
    $oldDyld = $env:DYLD_LIBRARY_PATH
    $oldLd = $env:LD_LIBRARY_PATH
    try {
        if ($Platform -eq "windows-x64") {
            $env:PATH = "$runtimeBin;$env:PATH"
        }
        if ($Platform -eq "macos-universal") {
            $env:DYLD_LIBRARY_PATH = "$runtimeBin;$env:DYLD_LIBRARY_PATH"
        }
        if ($Platform -eq "linux-x64") {
            $env:LD_LIBRARY_PATH = "$runtimeBin;$env:LD_LIBRARY_PATH"
        }
        Write-SmokeLog "runtime_playback=start"
        & pwsh @smokeArgs
        if ($LASTEXITCODE -ne 0) {
            Fail "Runtime playback smoke failed"
        }
        Write-SmokeLog "runtime_playback=ok"
    } finally {
        $env:PATH = $oldPath
        $env:DYLD_LIBRARY_PATH = $oldDyld
        $env:LD_LIBRARY_PATH = $oldLd
    }
}

Write-SmokeLog "package_smoke=ok"
Write-Host "Package smoke passed: $PackageDir"
Write-Host "Smoke log: $script:SmokeLog"
