#requires -Version 7.0
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
function Get-UpdaterQaLayout([string]$Root,[string]$Platform) {
    $target=Get-VelopackTarget $Platform
    $install=Join-Path $Root 'installation'
    $launcher=if($Platform -eq 'windows-x64'){Join-Path $install 'current/yoyovideo-desktop.exe'}elseif($Platform -like 'macos-*'){Join-Path $install 'YoYoVideo QA ONLY.app/Contents/MacOS/yoyovideo-desktop'}else{Join-Path $install 'YoYoVideo.AppImage'}
    return [pscustomobject]@{Install=$install;Launcher=$launcher;Packages=(Join-Path $Root 'sdk-packages');Executable=$target.Exe}
}
function Read-UpdaterQaEvents([string]$Path) {
    if(-not (Test-Path -LiteralPath $Path -PathType Leaf)){return}
    $stream=[IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
    if($stream.Length -gt 67108864){$stream.Dispose();throw 'QA event log exceeds limit'}
    $reader=[IO.StreamReader]::new($stream)
    try {$lines=$reader.ReadToEnd().Split([char]10)}finally{$reader.Dispose()}
    foreach($line in $lines) {
        if([string]::IsNullOrWhiteSpace($line)){continue}
        try {$record=ConvertFrom-Json -InputObject $line -NoEnumerate -ErrorAction Stop}catch{continue}
        if($record -is [pscustomobject] -and $null -ne $record.PSObject.Properties['pid'] -and $null -ne $record.PSObject.Properties['version']) { $record }
    }
}
