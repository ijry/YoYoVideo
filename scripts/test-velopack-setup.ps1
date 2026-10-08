#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$ReleaseDir,[Parameter(Mandatory)][string]$Version)
$ErrorActionPreference='Stop'
# Setup mutates HKCU and shell shortcuts. Never run it in a developer profile.
if(-not $IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'Setup acceptance is restricted to a disposable GitHub-hosted Windows runner'
}
. (Join-Path $PSScriptRoot 'velopack-archive.ps1')
Assert-VelopackVersion $Version
$ReleaseDir=(Resolve-Path -LiteralPath $ReleaseDir).Path
& (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -ReleaseDir $ReleaseDir -Platform windows-x64 -Version $Version -BeforeSigning
$setup=Join-Path $ReleaseDir (Get-VelopackPrimaryAsset $ReleaseDir 'windows-x64')
$key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo'
$shortcuts=@((Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'YoYoVideo.lnk'),(Join-Path ([Environment]::GetFolderPath('Programs')) 'YoYoVideo.lnk'))
if(Test-Path -LiteralPath $key){throw 'Existing YoYoVideo registration; refusing to overwrite'}
foreach($shortcut in $shortcuts){if(Test-Path -LiteralPath $shortcut){throw 'Existing YoYoVideo shortcut; refusing to overwrite'}}
$case=Join-Path $env:RUNNER_TEMP ('yoyo-setup-'+[guid]::NewGuid().ToString('N'))
$install=Join-Path $case 'application';New-Item -ItemType Directory -Path $case | Out-Null
$sentinelDir=Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'xyito/YoYoVideo/data'
$sentinel=Join-Path $sentinelDir ('setup-preserve-'+[guid]::NewGuid().ToString('N')+'.txt')
New-Item -ItemType Directory -Force -Path $sentinelDir | Out-Null
'preserve-user-data' | Set-Content -LiteralPath $sentinel
function Run-SetupTool([string]$File,[string[]]$Arguments) {
    $process=Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if(-not $process.WaitForExit(120000)){throw 'Installer/uninstaller did not exit'}
    if($process.ExitCode -ne 0){throw "Installer/uninstaller failed: $($process.ExitCode)"}
}
try {
    Run-SetupTool $setup @('--silent','--installto',('"'+$install+'"'),'--log',('"'+(Join-Path $case 'setup.log')+'"'))
    $registry=Get-ItemProperty -LiteralPath $key
    if($registry.DisplayName -cne 'YoYoVideo' -or $registry.DisplayVersion -cne $Version -or [IO.Path]::GetFullPath($registry.InstallLocation).TrimEnd('\') -ine $install){throw 'Installed registration does not match the expected application'}
    if(-not $registry.UninstallString.Contains((Join-Path $install 'Update.exe'))){throw 'Uninstall entry does not target this installation'}
    if(Test-Path -LiteralPath (Join-Path $install '.portable')){throw 'Setup produced a portable layout'}
    $main=Join-Path $install 'current/yoyovideo-desktop.exe'
    $null=Get-VelopackBuildInfo -Executable $main -Version $Version
    $shell=New-Object -ComObject WScript.Shell
    $links=@()
    foreach($shortcut in $shortcuts) {
        if(-not (Test-Path -LiteralPath $shortcut)){throw "Missing shortcut: $shortcut"}
        $link=$shell.CreateShortcut($shortcut)
        if(-not (Test-Path -LiteralPath $link.TargetPath) -or (Split-Path $link.TargetPath) -ine $install){throw 'Shortcut does not point to a stable executable in the install root'}
        $links+=@{path=$shortcut;target=$link.TargetPath}
    }
    Run-SetupTool (Join-Path $install 'Update.exe') @('--uninstall','--silent','--log',('"'+(Join-Path $case 'uninstall.log')+'"'))
    if(Test-Path -LiteralPath $key){throw 'Uninstall registration was not removed'}
    foreach($shortcut in $shortcuts){if(Test-Path -LiteralPath $shortcut){throw 'Uninstall left a shortcut'}}
    if(Test-Path -LiteralPath $main){throw 'Uninstall left the player executable'}
    if([IO.File]::ReadAllText($sentinel).Trim() -cne 'preserve-user-data'){throw 'Uninstall removed user data'}
    @{version=$Version;installed=$true;shortcut_targets=$links;uninstalled=$true;user_data_preserved=$true} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $case 'SUCCESS.json')
    Write-Host "PASS clean Windows Setup installation/shortcuts/uninstall: $case"
} finally {
    if(Test-Path -LiteralPath $key) {
        $owned=Get-ItemProperty -LiteralPath $key
        if([IO.Path]::GetFullPath($owned.InstallLocation).TrimEnd('\') -ieq $install -and (Test-Path -LiteralPath (Join-Path $install 'Update.exe'))) {
            Run-SetupTool (Join-Path $install 'Update.exe') @('--uninstall','--silent')
        }
    }
}
