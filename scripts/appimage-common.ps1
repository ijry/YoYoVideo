#requires -Version 7.0
# Pure dependency policy shared by staging, validation and cross-platform tests.
Set-StrictMode -Version Latest
function Test-AppImageHostLibrary([string]$Name) {
    # Keep the host loader/C/C++ ABI and graphics driver/dispatch stack together.
    return $Name -match '^(ld-linux|lib(c|m|pthread|dl|rt|resolv|util|anl|nss_[^.]+)\.so|libstdc\+\+\.so|libgcc_s\.so|lib(GL|EGL|OpenGL|GLES|GLX|GLdispatch|glapi|drm|gbm|va|vdpau|vulkan|OpenCL|cuda|nvidia)[^.]*\.so)'
}
function ConvertFrom-AppImageLdd([string[]]$Lines) {
    foreach($line in $Lines) {
        if($line -match '=>\s+not found'){throw 'Unresolved runtime dependency'}
        if($line -match '^\s*(\S+)\s+=>\s+(/\S+)\s+\('){[pscustomobject]@{Name=$matches[1];Path=$matches[2]};continue}
        if($line -match '^\s*(/\S+)\s+\('){[pscustomobject]@{Name=[IO.Path]::GetFileName($matches[1]);Path=$matches[1]};continue}
        if($line -match '^\s*(linux-vdso\.so\.|statically linked)' -or [string]::IsNullOrWhiteSpace($line)){continue}
        throw 'Unexpected ldd output'
    }
}
function ConvertFrom-AppImageLdconfig([string[]]$Lines) {
    foreach($line in $Lines){if($line -match '^\s*(\S+)\s+\([^)]*x86-64[^)]*\)\s+=>\s+(/\S+)\s*$'){[pscustomobject]@{Name=$matches[1];Path=$matches[2]}}}
}
function Resolve-AppImageLibmpv([string[]]$Lines) {
    $libs=@(ConvertFrom-AppImageLdconfig $Lines)
    foreach($abi in @('libmpv.so.2','libmpv.so.1')) {
        $found=@($libs | Where-Object Name -CEQ $abi)
        if($found.Count -gt 0){return $found[0]}
    }
    throw 'No x86_64 libmpv runtime found; install libmpv-dev on the build host'
}
function Assert-AppImageBaseline([string[]]$Lines) {
    if($Lines -notcontains 'ID=ubuntu' -or $Lines -notcontains 'VERSION_ID="22.04"'){throw 'AppImage releases must be built on Ubuntu 22.04; do not silently raise the ABI baseline'}
}
function Get-AppImageDlopenLibraries {
    return @('libX11.so.6','libXcursor.so.1','libXrandr.so.2','libXi.so.6','libxcb.so.1','libxcb-shape.so.0','libxcb-xfixes.so.0','libxcb-render.so.0','libxkbcommon.so.0','libxkbcommon-x11.so.0','libwayland-client.so.0','libwayland-cursor.so.0','libwayland-egl.so.1')
}
function Get-AppImageLauncher {
    return @('#!/bin/sh','set -eu','APPDIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"','export APPDIR','export LD_LIBRARY_PATH="$APPDIR/usr/bin:${LD_LIBRARY_PATH:-}"','export PATH="$APPDIR/usr/bin:$PATH"','exec "$APPDIR/usr/bin/yoyovideo-desktop" "$@"','') -join [char]10
}

function Assert-AppImageRuntimeProvenance([string]$Bin,[string]$Version) {
    $path=Join-Path $Bin 'LICENSES/appimage-runtime.json'
    if((Get-Item -LiteralPath $path).Length -gt 8388608){throw 'Oversized AppImage provenance'}
    $record=[IO.File]::ReadAllText($path) | ConvertFrom-Json
    if($record.schema -cne 'yoyovideo-appimage-runtime-v1' -or $record.version -cne $Version -or $record.baseline -cne 'ubuntu-22.04-x86_64'){throw 'Invalid AppImage provenance identity'}
    $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach($item in $record.bundled) {
        Assert-VelopackAssetName $item.file;Assert-VelopackAssetName $item.license
        if(-not $seen.Add($item.file) -or (Test-AppImageHostLibrary $item.file)){throw 'Duplicate or forbidden bundled library'}
        if($item.sha256 -cnotmatch '^[a-f0-9]{64}$' -or [string]::IsNullOrWhiteSpace($item.package) -or [string]::IsNullOrWhiteSpace($item.version)){throw 'Incomplete runtime provenance'}
        $file=Join-Path $Bin $item.file
        Assert-VelopackBinary -Path $file -Platform linux-x64 -Library
        if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $item.sha256){throw 'Bundled runtime hash mismatch'}
        if(-not (Test-Path -LiteralPath (Join-Path $Bin ('LICENSES/'+$item.license)))){throw 'Missing bundled-library license'}
    }
    foreach($file in Get-ChildItem -LiteralPath $Bin -File | Where-Object Name -Match '\.so(?:\.|$)') {
        if(-not $seen.Contains($file.Name)){throw 'Unrecorded shared library in AppImage'}
    }
    if(-not ($seen.Contains('libmpv.so.1') -xor $seen.Contains('libmpv.so.2'))){throw 'Expected exactly one bundled libmpv ABI'}
    $source=[IO.File]::ReadAllText((Join-Path $Bin 'LICENSES/runtime-source.json')) | ConvertFrom-Json
    if(-not $seen.Contains($source.library) -or (Get-FileHash -LiteralPath (Join-Path $Bin $source.library) -Algorithm SHA256).Hash -ne $source.sha256){throw 'AppImage libmpv does not match build provenance'}
}
