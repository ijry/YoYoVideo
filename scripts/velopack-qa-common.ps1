#requires -Version 7.0
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
function Get-UpdaterQaLayout([string]$Root,[string]$Platform) {
    $target=Get-VelopackTarget $Platform
    $install=Join-Path $Root 'installation'
    $launcher=if($Platform -eq 'windows-x64'){Join-Path $install 'current/yoyovideo-desktop.exe'}elseif($Platform -like 'macos-*'){Join-Path $install 'YoYoVideo QA ONLY.app/Contents/MacOS/yoyovideo-desktop'}else{Join-Path $install 'YoYoVideo.AppImage'}
    return [pscustomobject]@{Install=$install;Launcher=$launcher;Packages=(Join-Path $Root 'sdk-packages');Executable=$target.Exe}
}
