#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'velopack-qa-common.ps1')
$root=Join-Path ([IO.Path]::GetTempPath()) 'yoyo-qa-layout-contract'
foreach($platform in @('windows-x64','macos-aarch64','macos-x86_64','linux-x64')) {
    $layout=Get-UpdaterQaLayout $root $platform
    if($layout.Packages -ne (Join-Path $root 'sdk-packages')){throw 'SDK cache escaped fixture'}
    if(-not $layout.Launcher.StartsWith($root)){throw 'Launch path escaped fixture'}
    if($platform -like 'macos-*' -and -not $layout.Launcher.EndsWith('yoyovideo-desktop')){throw 'Wrong app entry'}
    if($platform -eq 'linux-x64' -and -not $layout.Launcher.EndsWith('.AppImage')){throw 'Not an AppImage'}
}
$bad=$false;try{Get-UpdaterQaLayout $root 'unknown'}catch{$bad=$true};if(-not $bad){throw 'Unknown target accepted'}
Write-Host 'PASS native QA layout contracts for all four targets'
