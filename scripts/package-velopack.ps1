#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('windows-x64','macos-aarch64','macos-x86_64','linux-x64')][string]$Platform,
    [Parameter(Mandatory)][string]$Version,[Parameter(Mandatory)][string]$PackageDir,
    [Parameter(Mandatory)][string]$OutputDir,[string]$VpkPath,[string]$SignerPath,[string]$PublicKeyPath,
    [switch]$PrepareOnly,[switch]$PlanOnly,[switch]$QaFixture
)
$ErrorActionPreference='Stop'
if($QaFixture -and -not $PrepareOnly){throw 'QA packaging requires PrepareOnly; never use production signing credentials'}
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$target=Get-VelopackTarget $Platform; Assert-VelopackVersion $Version
$PackageDir=[IO.Path]::GetFullPath($PackageDir);$OutputDir=[IO.Path]::GetFullPath($OutputDir)
Assert-VelopackStaging $PackageDir $Platform
if(-not $PublicKeyPath){$PublicKeyPath=Join-Path $repo 'apps/yoyovideo-desktop/assets/updater.pub'}
if(-not (Test-Path -LiteralPath $PublicKeyPath -PathType Leaf)){throw 'Pinned public key is required'}
$prefix=$PackageDir.TrimEnd('\','/')+[IO.Path]::DirectorySeparatorChar
if($OutputDir -eq $PackageDir -or $OutputDir.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)){throw 'Output must not be inside source staging'}
if((Test-Path -LiteralPath $OutputDir) -and @(Get-ChildItem -LiteralPath $OutputDir -Force).Count){throw 'Output directory must be empty; release artifacts are not overwritten'}
$work=Join-Path $repo ('.cache/velopack-build-'+[Guid]::NewGuid().ToString('N'))
$payload=Join-Path $work 'payload'
if($Platform -like 'macos-*'){$payload=Join-Path $work 'YoYoVideo.app'}
if($Platform -eq 'linux-x64'){$payload=Join-Path $work 'YoYoVideo.AppDir'}
$icon=if($Platform -eq 'windows-x64'){Join-Path $repo 'apps/yoyovideo-desktop/assets/icons/yoyovideo.ico'}elseif($Platform -like 'macos-*'){Join-Path $work 'yoyovideo.icns'}else{Join-Path $repo 'apps/yoyovideo-desktop/assets/icons/yoyovideo-512.png'}
$notes=Join-Path $PackageDir 'RELEASE-NOTES.md'
$arguments=@(Get-VelopackPackArguments $Platform $Version $payload $OutputDir $icon $notes -QaFixture:$QaFixture)
if($PlanOnly){[pscustomobject]@{Platform=$Platform;Runtime=$target.Rid;Channel=$target.Channel;Arguments=$arguments;PrepareOnly=[bool]$PrepareOnly} | ConvertTo-Json -Depth 5;return}
Assert-VelopackHost $Platform
if(-not $PrepareOnly -and ([string]::IsNullOrWhiteSpace($env:YOYOVIDEO_UPDATER_PRIVATE_KEY) -or $null -eq $env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD)){throw 'Signing credentials are required; use PrepareOnly only for unsigned CI staging'}
Initialize-VelopackDotnet $repo
$toolName=if($IsWindows){'vpk.exe'}else{'vpk'}
$VpkPath=Resolve-VelopackTool 'vpk' $VpkPath (Join-Path $repo ('.cache/tools/vpk/'+$toolName));Assert-VelopackCli $VpkPath
$build=Get-VelopackBuildInfo -Executable (Join-Path $PackageDir ('bin/'+$target.Exe)) -Version $Version -QaFixture:$QaFixture
New-Item -ItemType Directory -Force -Path $work,$OutputDir | Out-Null
$bin=$payload;$resources=$payload
if($Platform -like 'macos-*'){$bin=Join-Path $payload 'Contents/MacOS';$resources=Join-Path $payload 'Contents/Resources'}
elseif($Platform -eq 'linux-x64'){$bin=Join-Path $payload 'usr/bin';$resources=$bin}
Copy-VelopackPayload (Join-Path $PackageDir 'bin') $bin
Copy-VelopackPayload (Join-Path $PackageDir 'docs') (Join-Path $resources 'docs')
Copy-VelopackPayload (Join-Path $PackageDir 'LICENSES') (Join-Path $resources 'LICENSES')
foreach($file in @('README.md','RELEASE-NOTES.md')){Copy-Item -LiteralPath (Join-Path $PackageDir $file) -Destination (Join-Path $resources $file)}
Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination (Join-Path $resources 'LICENSE')
Copy-Item -LiteralPath (Join-Path $repo 'LICENSES.md') -Destination (Join-Path $resources 'LICENSES/project-notices.md')
Copy-Item -LiteralPath (Join-Path $repo 'third_party/velopack/LICENSE.txt') -Destination (Join-Path $resources 'LICENSES/Velopack-LICENSE.txt')
$build | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $resources 'yoyovideo-build-info.json') -Encoding utf8NoBOM
if($Platform -like 'macos-*') {
    New-VelopackMacIcon -PngPath (Join-Path $repo 'apps/yoyovideo-desktop/assets/icons/yoyovideo-512.png') -OutputPath $icon
    Copy-Item -LiteralPath $icon -Destination (Join-Path $resources 'yoyovideo.icns')
    $plist='<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.xyito.yoyovideo</string><key>CFBundleName</key><string>YoYoVideo</string><key>CFBundleDisplayName</key><string>YoYoVideo</string><key>CFBundleExecutable</key><string>yoyovideo-desktop</string><key>CFBundleIconFile</key><string>yoyovideo.icns</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>'+ $Version +'</string><key>CFBundleVersion</key><string>'+ $Version +'</string><key>NSHighResolutionCapable</key><true/></dict></plist>'
    [IO.File]::WriteAllText((Join-Path $payload 'Contents/Info.plist'),$plist)
}
if($Platform -eq 'linux-x64') {
    & (Join-Path $PSScriptRoot 'stage-appimage-runtime.ps1') -AppDir $payload -Version $Version
    if($LASTEXITCODE -ne 0){throw 'AppImage runtime staging failed'}
}
Invoke-VelopackTool -FilePath $VpkPath -Arguments $arguments | Out-Host
& (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform $Platform -Version $Version -ReleaseDir $OutputDir -BeforeSigning -Native -QaFixture:$QaFixture
if($LASTEXITCODE -ne 0){throw 'Native package verification failed'}
if(-not $PrepareOnly) {
    $signName=if($IsWindows){'yoyo-update-sign.exe'}else{'yoyo-update-sign'}
    $SignerPath=Resolve-VelopackTool 'yoyo-update-sign' $SignerPath (Join-Path $repo ('target/release/'+$signName))
    $manifest=Join-Path $OutputDir ('yoyovideo-update.'+$Platform+'.json')
    Invoke-VelopackTool -FilePath $SignerPath -Arguments @('sign','--feed',(Join-Path $OutputDir ('releases.'+$target.Channel+'.json')),'--platform',$Platform,'--version',$Version,'--assets-dir',$OutputDir,'--output',$manifest,'--public-key',$PublicKeyPath) -Signing | Out-Host
    & (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -Platform $Platform -Version $Version -ReleaseDir $OutputDir -SignerPath $SignerPath -PublicKeyPath $PublicKeyPath
    if($LASTEXITCODE -ne 0){throw 'Signed package verification failed'}
}
Write-Host "Velopack package prepared: $OutputDir (signed: $(-not $PrepareOnly))"
Write-Host "Build staging retained for diagnostics: $work"