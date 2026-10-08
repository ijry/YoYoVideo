#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$ReleaseDir,[Parameter(Mandatory)][string]$Platform,[Parameter(Mandatory)][string]$Version,[string]$SignerPath,[string]$PublicKeyPath,[switch]$BeforeSigning,[switch]$Native,[switch]$QaFixture)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'velopack-archive.ps1')
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$ReleaseDir=(Resolve-Path -LiteralPath $ReleaseDir).Path
$t=Get-VelopackTarget $Platform;Assert-VelopackVersion $Version
$feedPath=Join-Path $ReleaseDir ('releases.'+$t.Channel+'.json')
if((Get-Item -LiteralPath $feedPath).Length -gt 8388608){throw 'Oversized release feed'}
$feed=[IO.File]::ReadAllText($feedPath) | ConvertFrom-Json
$assets=@($feed.Assets)
if($assets.Count -ne 1){throw 'Exactly one full update package is required'}
$a=$assets[0];Assert-VelopackAssetName $a.FileName
if($a.Type -cne 'Full' -or $a.PackageId -cne 'YoYoVideo' -or $a.Version -cne $Version -or $a.SHA256 -notmatch '^[a-fA-F0-9]{64}$' -or $a.SHA1 -notmatch '^[a-fA-F0-9]{40}$'){throw 'Invalid full-package metadata'}
$full=Join-Path $ReleaseDir $a.FileName
if($a.Size -le 0 -or $a.Size -gt 2147483648 -or (Get-Item -LiteralPath $full).Length -ne $a.Size){throw 'Package size mismatch'}
if((Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash -ne $a.SHA256){throw 'Package SHA-256 mismatch'}
$metadata=Assert-VelopackArchive -Path $full -Platform $Platform -Version $Version -QaFixture:$QaFixture
$primaryName=Get-VelopackPrimaryAsset $ReleaseDir $Platform
$primary=Join-Path $ReleaseDir $primaryName
if($Platform -eq 'windows-x64'){Assert-VelopackBinary -Path $primary -Platform $Platform}
elseif($Platform -eq 'linux-x64') {
    Assert-VelopackBinary -Path $primary -Platform $Platform
    $zip=[IO.Compression.ZipFile]::OpenRead($full)
    try {$stream=$zip.GetEntry('lib/app/YoYoVideo.AppImage').Open();$hash=[Security.Cryptography.SHA256]::Create();try{$embedded=[Convert]::ToHexString($hash.ComputeHash($stream))}finally{$stream.Dispose();$hash.Dispose()}}finally{$zip.Dispose()}
    if($embedded -ne (Get-FileHash -LiteralPath $primary -Algorithm SHA256).Hash){throw 'AppImage differs from the authenticated full package'}
} else {
    $bundleName=if($QaFixture){'YoYoVideo QA ONLY.app'}else{'YoYoVideo.app'}
    $zip=[IO.Compression.ZipFile]::OpenRead($primary);$package=[IO.Compression.ZipFile]::OpenRead($full)
    try {
        Assert-VelopackZipPaths $zip $Platform
        $main=$zip.GetEntry(($bundleName+'/Contents/MacOS/yoyovideo-desktop'));if($null -eq $main){throw 'Missing .app in portable archive'}
        $stream=$main.Open();try{Assert-VelopackBinary -Bytes (Read-VelopackPrefix $stream) -Platform $Platform}finally{$stream.Dispose()}
        foreach($entry in $zip.Entries) {
            if($entry.FullName.EndsWith('/') -or $entry.FullName.StartsWith('__MACOSX/')){continue}
            if(-not $entry.FullName.StartsWith($bundleName+'/')){throw 'Unexpected file outside the app bundle'}
            $relative=$entry.FullName.Substring(($bundleName+'/').Length);$other=$package.GetEntry('lib/app/'+$relative)
            if($null -eq $other -or $other.Length -ne $entry.Length){throw 'Portable app differs from full package'}
            $sha=[Security.Cryptography.SHA256]::Create();$left=$entry.Open();$right=$other.Open()
            try {if([Convert]::ToHexString($sha.ComputeHash($left)) -ne [Convert]::ToHexString($sha.ComputeHash($right))){throw 'Portable app content differs from full package'}}finally{$left.Dispose();$right.Dispose();$sha.Dispose()}
        }
    } finally {$zip.Dispose();$package.Dispose()}
}
if(-not $BeforeSigning) {
    if(-not $PublicKeyPath){$PublicKeyPath=Join-Path $repo 'apps/yoyovideo-desktop/assets/updater.pub'}
    $signName=if($IsWindows){'yoyo-update-sign.exe'}else{'yoyo-update-sign'}
    $SignerPath=Resolve-VelopackTool 'yoyo-update-sign' $SignerPath (Join-Path $repo ('target/release/'+$signName))
    Invoke-VelopackTool $SignerPath @('verify','--manifest',(Join-Path $ReleaseDir ('yoyovideo-update.'+$Platform+'.json')),'--platform',$Platform,'--assets-dir',$ReleaseDir,'--public-key',$PublicKeyPath) | Out-Host
}
if($Native) {
    Assert-VelopackHost $Platform
    if($Platform -like 'macos-*') {
        $work=Join-Path $repo ('.cache/verify-macos-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Path $work | Out-Null
        Invoke-VelopackTool '/usr/bin/ditto' @('-x','-k',$primary,$work) | Out-Null
        $app=Join-Path $work $bundleName;$bin=Join-Path $app 'Contents/MacOS'
        Invoke-VelopackTool '/usr/bin/codesign' @('--verify','--deep','--strict',$app) | Out-Null
        foreach($file in Get-ChildItem -LiteralPath $bin -File | Where-Object {$_.Name -eq 'yoyovideo-desktop' -or $_.Name -eq 'UpdateMac' -or $_.Name -like '*.dylib'}) {
            Assert-VelopackBinary -Path $file.FullName -Platform $Platform -Library
            Invoke-VelopackTool '/usr/bin/codesign' @('--verify','--strict',$file.FullName) | Out-Null
            $dependencies=@(Invoke-VelopackTool '/usr/bin/otool' @('-L',$file.FullName)) | Select-Object -Skip 1
            foreach($line in $dependencies) {
                if($line -match '^\s*(\S+)\s+\('){$dependency=$matches[1];if($dependency -like '/System/Library/*' -or $dependency -like '/usr/lib/*'){continue};if($dependency -match '^@(rpath|loader_path|executable_path)/(.+)$' -and (Test-Path -LiteralPath (Join-Path $bin $matches[2]))){continue};throw 'Non-relocatable or missing dylib dependency'}
            }
            foreach($line in @(Invoke-VelopackTool '/usr/bin/otool' @('-l',$file.FullName))){if($line -match '^\s*path\s+/(opt|usr/local|Users)/'){throw 'Absolute build-host rpath in app bundle'}}
        }
    } elseif($Platform -eq 'linux-x64') {
        $work=Join-Path $repo ('.cache/verify-appimage-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Path $work | Out-Null
        Invoke-VelopackTool 'chmod' @('+x',$primary) | Out-Null
        Push-Location $work
        try {Invoke-VelopackTool $primary @('--appimage-extract') | Out-Null}finally{Pop-Location}
        $appdir=Join-Path $work 'squashfs-root';$bin=Join-Path $appdir 'usr/bin'
        foreach($name in @('yoyovideo-desktop','UpdateNix')){Assert-VelopackBinary -Path (Join-Path $bin $name) -Platform $Platform -Library}
        . (Join-Path $PSScriptRoot 'appimage-common.ps1')
        Assert-AppImageRuntimeProvenance $bin $Version
        $old=$env:LD_LIBRARY_PATH
        try {
            $env:LD_LIBRARY_PATH=$bin
            foreach($file in Get-ChildItem -LiteralPath $bin -File | Where-Object {$_.Name -in @('yoyovideo-desktop','UpdateNix') -or $_.Name -match '\.so(?:\.|$)'}) {
                $dynamic=Invoke-VelopackTool 'readelf' @('-d',$file.FullName)
                if(-not ($dynamic -match '\(NEEDED\)')){continue} # Static native helpers have no ldd closure.
                $lines=Invoke-VelopackTool 'ldd' @($file.FullName)
                if($lines -match 'not found'){throw 'Unresolved AppImage dependencies'}
            }
            $null=Get-VelopackBuildInfo -Executable (Join-Path $bin 'yoyovideo-desktop') -Version $Version -QaFixture:$QaFixture
        }finally{$env:LD_LIBRARY_PATH=$old}
    }
}
Write-Host "Verified $Platform $Version (signed: $(-not $BeforeSigning))"