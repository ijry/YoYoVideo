#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$AppDir,[Parameter(Mandatory)][string]$Version)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
. (Join-Path $PSScriptRoot 'appimage-common.ps1')
Assert-VelopackHost 'linux-x64'
Assert-VelopackVersion $Version
Assert-AppImageBaseline (Get-Content -LiteralPath '/etc/os-release')
$AppDir=(Resolve-Path -LiteralPath $AppDir).Path
if(-not $AppDir.EndsWith('.AppDir') -or (Get-Item -LiteralPath $AppDir).LinkType){throw 'Expected an isolated, non-linked .AppDir'}
$bin=Join-Path $AppDir 'usr/bin'
$main=Join-Path $bin 'yoyovideo-desktop'
Assert-VelopackBinary -Path $main -Platform linux-x64
$licenses=Join-Path $bin 'LICENSES'
New-Item -ItemType Directory -Force -Path $licenses | Out-Null
$cache=@(ConvertFrom-AppImageLdconfig @(Invoke-VelopackTool 'ldconfig' @('-p')))
$queue=[Collections.Generic.Queue[object]]::new()
$seen=@{};$records=[Collections.Generic.List[object]]::new();$hostLibraries=[Collections.Generic.HashSet[string]]::new()
function Get-DirectDependencies([string]$File) {
    $resolved=@(ConvertFrom-AppImageLdd @(Invoke-VelopackTool 'ldd' @($File)))
    foreach($line in @(Invoke-VelopackTool 'readelf' @('-d',$File))) {
        if($line -match '\(NEEDED\).*\[([^\]]+)\]') {
            $name=$matches[1];$found=@($resolved | Where-Object Name -CEQ $name)
            if($found.Count -ne 1){throw "Unresolved or ambiguous direct dependency: $name"}
            $found[0]
        }
    }
}
function Get-DistroOwner([string]$File) {
    # dpkg records may use either /lib or /usr/lib, depending on usr-merge.
    $paths=@($File)
    if($File.StartsWith('/usr/lib/')){$paths+=$File.Substring(4)}
    elseif($File.StartsWith('/lib/')){$paths+='/usr'+$File}
    foreach($candidate in $paths) {
        try {$output=@(Invoke-VelopackTool 'dpkg-query' @('-S',$candidate))}catch{continue}
        foreach($line in $output){if($line -match '^([a-z0-9][a-z0-9+.-]*)(?::amd64)?: /') {return $matches[1]}}
    }
    throw 'Bundled dependency is not owned by a distribution package'
}
foreach($dep in @(Get-DirectDependencies $main)){$queue.Enqueue($dep)}
foreach($name in Get-AppImageDlopenLibraries) {
    $found=@($cache | Where-Object Name -CEQ $name)
    if($found.Count -eq 0){throw "Missing dlopen runtime: $name"}
    $queue.Enqueue($found[0])
}
while($queue.Count -gt 0) {
    $dep=$queue.Dequeue();$name=$dep.Name
    if(Test-AppImageHostLibrary $name){[void]$hostLibraries.Add($name);continue}
    Assert-VelopackAssetName $name
    $real=(@(Invoke-VelopackTool 'readlink' @('-f',$dep.Path)) -join '').Trim()
    if(-not ($real.StartsWith('/usr/lib/') -or $real.StartsWith('/lib/'))){throw 'Only distribution libraries may be bundled'}
    $hash=(Get-FileHash -LiteralPath $real -Algorithm SHA256).Hash.ToLowerInvariant()
    if($seen.ContainsKey($name)){if($seen[$name] -ne $hash){throw 'Conflicting runtime SONAMEs'};continue}
    $seen[$name]=$hash
    Assert-VelopackBinary -Path $real -Platform linux-x64 -Library
    $destination=Join-Path $bin $name
    if(Test-Path -LiteralPath $destination){throw 'AppDir contains an unexpected pre-existing runtime'}
    Copy-Item -LiteralPath $real -Destination $destination
    $package=Get-DistroOwner $real
    $packageVersion=(@(Invoke-VelopackTool 'dpkg-query' @('-W','-f=${Version}',$package)) -join '').Trim()
    $copyright=Join-Path '/usr/share/doc' ($package+'/copyright')
    if(-not (Test-Path -LiteralPath $copyright)){throw "Missing copyright notice for $package"}
    $licenseName='distro-'+$package+'-copyright.txt'
    Copy-Item -LiteralPath $copyright -Destination (Join-Path $licenses $licenseName) -Force
    $records.Add([ordered]@{file=$name;sha256=$hash;source=$real;package=$package;version=$packageVersion;license=$licenseName})
    foreach($child in @(Get-DirectDependencies $real)){$queue.Enqueue($child)}
}
if(-not ($seen.ContainsKey('libmpv.so.1') -or $seen.ContainsKey('libmpv.so.2'))){throw 'The player was not linked to libmpv'}
# Distro copyright notices refer to these license texts by their system path.
Copy-VelopackPayload '/usr/share/common-licenses' (Join-Path $licenses 'common-licenses')
[ordered]@{schema='yoyovideo-appimage-runtime-v1';version=$Version;baseline='ubuntu-22.04-x86_64';bundled=@($records | Sort-Object file);host_libraries=@($hostLibraries | Sort-Object);host_requirements=@('glibc >= 2.35 and compatible C++ ABI','desktop X11/Wayland session and system fonts/fontconfig configuration','host OpenGL/EGL/GPU drivers; optional VA-API/VDPAU/Vulkan drivers','audio services and optional ALSA/PipeWire plugins')} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $licenses 'appimage-runtime.json') -Encoding utf8NoBOM
[IO.File]::WriteAllText((Join-Path $AppDir 'AppRun'),(Get-AppImageLauncher))
$desktop=@('[Desktop Entry]','Type=Application','Name=YoYoVideo','Exec=yoyovideo-desktop %F','Icon=yoyovideo','Categories=AudioVideo;Player;','Terminal=false','') -join [char]10
[IO.File]::WriteAllText((Join-Path $AppDir 'YoYoVideo.desktop'),$desktop)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../apps/yoyovideo-desktop/assets/icons/yoyovideo-512.png') -Destination (Join-Path $AppDir 'yoyovideo.png')
Invoke-VelopackTool 'chmod' @('+x',(Join-Path $AppDir 'AppRun'),$main) | Out-Null
Write-Host "Staged $($records.Count) distribution libraries; host ABI/graphics libraries excluded."
