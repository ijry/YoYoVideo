#requires -Version 7.0
[CmdletBinding()]
param([string]$VpkPath,[string]$SignerPath,[string]$PlayerPath)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
$root=Join-Path $repo ('.cache/velopack-integration-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$scriptPath=Join-Path $PSScriptRoot 'package-velopack.ps1'
& pwsh -NoProfile -File $scriptPath -Platform windows-x64 -Version 0.0.1 -PackageDir (Join-Path $root 'missing') -OutputDir (Join-Path $root 'invalid') -PrepareOnly
if($LASTEXITCODE -eq 0){throw 'Missing staging directory was accepted'}
if(-not $IsWindows){Write-Host 'Real Windows packaging integration runs on Windows only';exit 0}
if(-not $VpkPath){$VpkPath=Join-Path $repo '.cache/tools/vpk/vpk.exe'}
if(-not $SignerPath){$SignerPath=Join-Path $repo 'target/debug/yoyo-update-sign.exe'}
if(-not $PlayerPath){$PlayerPath=Join-Path $repo 'target/debug/yoyovideo-desktop.exe'}
$input=Join-Path $root 'stage'
foreach($dir in @('bin','docs','LICENSES')){New-Item -ItemType Directory -Force -Path (Join-Path $input $dir) | Out-Null}
Copy-Item -LiteralPath $PlayerPath -Destination (Join-Path $input 'bin/yoyovideo-desktop.exe')
Copy-Item -LiteralPath (Join-Path $repo 'third_party/mpv/windows-x64/bin/mpv-2.dll') -Destination (Join-Path $input 'bin/mpv-2.dll')
foreach($name in @('README.md','RELEASE-NOTES.md','LICENSES/README.md','LICENSES/runtime-provenance.md','docs/runtime-dependencies.md')){'TEST ONLY 0.0.1 - DO NOT PUBLISH' | Set-Content -LiteralPath (Join-Path $input $name)}
$keys=Join-Path $root 'test-keys'
Invoke-VelopackTool (Get-Command cargo).Source @('run','-p','yoyo-update-sign','--example','packaging_test_keys','--',$keys) | Out-Host
$savedKey=$env:YOYOVIDEO_UPDATER_PRIVATE_KEY;$savedPassword=$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD
try {
    $env:YOYOVIDEO_UPDATER_PRIVATE_KEY=[IO.File]::ReadAllText((Join-Path $keys 'TEST-ONLY-private.key'))
    $env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD='fixture-only'
    $public=Join-Path $keys 'TEST-ONLY-public.key';$out=Join-Path $root 'release'
    New-Item -ItemType Directory -Path $out | Out-Null
    & pwsh -NoProfile -File $scriptPath -Platform windows-x64 -Version 0.0.1 -PackageDir $input -OutputDir $out -VpkPath $VpkPath -SignerPath $SignerPath -PublicKeyPath $public
    if($LASTEXITCODE -ne 0){throw 'Real Velopack package pipeline failed'}
    $manifest=Join-Path $out 'yoyovideo-update.windows-x64.json'
    if(-not (Test-Path -LiteralPath $manifest) -or -not (Test-Path -LiteralPath ($manifest+'.sig'))){throw 'Signed update manifest missing'}
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform windows-x64 -Version 0.0.1 -ReleaseDir $out -SignerPath $SignerPath -PublicKeyPath $public
    if($LASTEXITCODE -ne 0){throw 'Independent package verification failed'}
    $signature=[IO.File]::ReadAllText($manifest+'.sig')
    try {
        [IO.File]::WriteAllText($manifest+'.sig','INVALID TEST SIGNATURE')
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform windows-x64 -Version 0.0.1 -ReleaseDir $out -SignerPath $SignerPath -PublicKeyPath $public
        if($LASTEXITCODE -eq 0){throw 'Invalid signature accepted'}
    } finally {[IO.File]::WriteAllText($manifest+'.sig',$signature)}
    $unsigned=Join-Path $root 'unsigned'
    & pwsh -NoProfile -File $scriptPath -Platform windows-x64 -Version 0.0.1 -PackageDir $input -OutputDir $unsigned -VpkPath $VpkPath -PrepareOnly
    if($LASTEXITCODE -ne 0){throw 'PrepareOnly packaging failed'}
    if(Test-Path -LiteralPath (Join-Path $unsigned 'yoyovideo-update.windows-x64.json.sig')){throw 'PrepareOnly unexpectedly signed artifacts'}
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform windows-x64 -Version 0.0.1 -ReleaseDir $unsigned -SignerPath $SignerPath -PublicKeyPath $public
    if($LASTEXITCODE -eq 0){throw 'Unsigned artifacts accepted as publishable'}
    $feed=[IO.File]::ReadAllText((Join-Path $out 'releases.stable-windows-x64.json')) | ConvertFrom-Json
    $package=Join-Path $out $feed.Assets[0].FileName
    $stream=[IO.File]::Open($package,[IO.FileMode]::Open,[IO.FileAccess]::Write);try{$stream.WriteByte(0)}finally{$stream.Dispose()}
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform windows-x64 -Version 0.0.1 -ReleaseDir $out -SignerPath $SignerPath -PublicKeyPath $public
    if($LASTEXITCODE -eq 0){throw 'Tampered update package accepted'}
    Write-Host 'Real Windows packaging and signature pipeline passed; no installer was executed.'
} finally {$env:YOYOVIDEO_UPDATER_PRIVATE_KEY=$savedKey;$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD=$savedPassword}
Write-Host "TEST ONLY artifacts retained under $root"