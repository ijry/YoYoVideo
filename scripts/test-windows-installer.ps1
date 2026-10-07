[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageDir
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'The installer smoke test requires Windows' }
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$PackageDir = (Resolve-Path -LiteralPath $PackageDir).Path
$makensis = Get-Command makensis -ErrorAction Stop

# Never run the ordinary installer against a test /D directory: that still
# overwrites the real user's Start Menu link and HKCU uninstall registration.
# Compile the same production installer with a unique side-by-side identity.
$appId = 'YoYoVideo-InstallerTest-' + [Guid]::NewGuid().ToString('N')
$testParent = Join-Path $repoRoot '.cache/windows-installer-tests'
New-Item -ItemType Directory -Force -Path $testParent | Out-Null
$testParent = (Resolve-Path -LiteralPath $testParent).Path
$testRoot = Join-Path $testParent $appId
$installDir = Join-Path $testRoot 'installed'
$installerPath = Join-Path $testRoot 'artifact-test.exe'
$programs = [Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)
$shortcutDir = Join-Path $programs $appId
$shortcutPath = Join-Path $shortcutDir 'YoYoVideo.lnk'
$registryPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$appId"

function Assert-Within([string]$Target, [string]$Parent) {
    $fullTarget = [IO.Path]::GetFullPath($Target)
    $fullParent = [IO.Path]::GetFullPath($Parent).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    if (-not $fullTarget.StartsWith($fullParent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing filesystem operation outside $Parent : $Target"
    }
}
function Invoke-SilentInstaller([string]$Executable, [string]$Arguments) {
    $process = Start-Process -FilePath $Executable -ArgumentList $Arguments -PassThru -WindowStyle Hidden
    if (-not $process.WaitForExit(60000)) {
        Stop-Process -Id $process.Id -Force
        throw 'Isolated installer operation timed out'
    }
    if ($process.ExitCode -ne 0) { throw "Isolated installer failed: $($process.ExitCode)" }
}

Assert-Within $testRoot $testParent
Assert-Within $installDir $testRoot
Assert-Within $shortcutDir $programs
if ((Test-Path -LiteralPath $testRoot) -or (Test-Path -LiteralPath $shortcutDir) -or
    (Test-Path -LiteralPath $registryPath)) {
    throw 'Refusing to reuse an existing installation identity'
}
New-Item -ItemType Directory -Path $testRoot | Out-Null
try {
    & $makensis.Source /V2 "/DAPP_ID=$appId" "/DPACKAGE_DIR=$PackageDir" `
        "/DOUTPUT_EXE=$installerPath" '/DAPP_VERSION=installer-test' `
        "/DICON_FILE=$(Join-Path $repoRoot 'apps/yoyovideo-desktop/assets/icons/yoyovideo.ico')" `
        (Join-Path $repoRoot 'installer/windows/yoyovideo.nsi')
    if ($LASTEXITCODE -ne 0) { throw 'Could not compile isolated installer' }
    # NSIS requires /D last, with no added quotes, even if the directory has spaces.
    Invoke-SilentInstaller $installerPath "/S /D=$installDir"

    $installedExe = Join-Path $installDir 'bin/yoyovideo-desktop.exe'
    & pwsh -NoProfile -File (Join-Path $repoRoot 'scripts/test-windows-app-artifacts.ps1') `
        -ExecutablePath $installedExe -ShortcutPath $shortcutPath
    if ($LASTEXITCODE -ne 0) { throw 'Installed application/shortcut regression check failed' }
    $registration = Get-ItemProperty -LiteralPath $registryPath
    if ($registration.InstallLocation -ne $installDir -or $registration.DisplayIcon -ne $installedExe) {
        throw 'Installed app registration points outside the installed application'
    }
    Write-Host 'Isolated Windows installer smoke passed: installed GUI executable and portable icon paths'
} finally {
    # All recursive filesystem targets are checked against their explicit roots,
    # including the uninstaller's own recursive $INSTDIR cleanup.
    Assert-Within $installDir $testRoot
    $uninstaller = Join-Path $installDir 'Uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller) {
        Invoke-SilentInstaller $uninstaller "/S _?=$installDir"
    }
    Assert-Within $shortcutDir $programs
    if (Test-Path -LiteralPath $shortcutDir) {
        Remove-Item -LiteralPath $shortcutDir -Recurse -Force
    }
    if (Test-Path -LiteralPath $registryPath) {
        Remove-Item -LiteralPath $registryPath -Force
    }
    Assert-Within $testRoot $testParent
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
