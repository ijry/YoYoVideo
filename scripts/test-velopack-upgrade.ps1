#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('windows-x64','macos-aarch64','macos-x86_64','linux-x64')][string]$Platform='windows-x64',
    [string]$FromVersion='0.0.1',[string]$ToVersion='0.0.2',
    [string]$BuildRoot,[switch]$BuildOnly,[int]$TimeoutSeconds=180,
    [ValidateSet('extract','fuse')][string]$LinuxMode='extract'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'velopack-archive.ps1')
. (Join-Path $PSScriptRoot 'velopack-qa-common.ps1')
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Assert-VelopackHost $Platform
$exe=(Get-VelopackTarget $Platform).Exe
$signerName=if($IsWindows){'yoyo-update-sign.exe'}else{'yoyo-update-sign'}
Assert-VelopackVersion $FromVersion;Assert-VelopackVersion $ToVersion
if([version]$ToVersion -le [version]$FromVersion){throw 'QA requires an increasing version pair'}
$savedEnvironment=@{}
foreach($name in @($script:SigningEnvironment)+@('YOYOVIDEO_UPDATER_QA_ROOT','RUSTFLAGS','TMPDIR','APPIMAGE_EXTRACT_AND_RUN')) {
    $savedEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process')
    [Environment]::SetEnvironmentVariable($name,$null,'Process')
}
$runRoot=$null
if($IsMacOS){$env:RUSTFLAGS='-C link-arg=-Wl,-rpath,@loader_path'}
elseif($IsLinux){$env:RUSTFLAGS='-C link-arg=-Wl,-rpath,$ORIGIN'}
$qaBuildStarted=$false
try {
    if(-not $BuildRoot) {
        $BuildRoot=Join-Path $repo ('.cache/updater-upgrade-build-'+[guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $BuildRoot | Out-Null
        'YoYoVideo updater QA build v1' | Set-Content -LiteralPath (Join-Path $BuildRoot 'TEST-ONLY-BUILD')
        $archive=Join-Path $BuildRoot 'source.zip'
        Invoke-VelopackTool 'git' @('archive','HEAD','--format=zip','--output',$archive) | Out-Host
        $source=Join-Path $BuildRoot 'source'
        Expand-Archive -LiteralPath $archive -DestinationPath $source
        # Explicit overlay of this feature only, never the unrelated installer work.
        $overlay=@('apps/yoyovideo-desktop/Cargo.toml','apps/yoyovideo-desktop/src/app.rs','apps/yoyovideo-desktop/src/main.rs','crates/yoyo-mpv/build.rs','apps/yoyovideo-desktop/src/lib.rs','apps/yoyovideo-desktop/src/startup_report.rs','apps/yoyovideo-desktop/src/update_runtime.rs','apps/yoyovideo-desktop/src/update_qa.rs','apps/yoyovideo-desktop/src/grid_runtime.rs','apps/yoyovideo-desktop/src/video_host_winit.rs','crates/yoyo-updater/Cargo.toml','crates/yoyo-updater/src/lib.rs','crates/yoyo-updater/src/source.rs','crates/yoyo-updater/src/service/native.rs','crates/yoyo-updater/src/qa_fixture.rs','crates/yoyo-updater/src/process_guard.rs')
        foreach($name in $overlay){Copy-Item -LiteralPath (Join-Path $repo $name) -Destination (Join-Path $source $name) -Force}
        Copy-VelopackPayload (Join-Path $repo "third_party/mpv/$Platform") (Join-Path $source "third_party/mpv/$Platform")
        $cargoTemplate=[IO.File]::ReadAllText((Join-Path $source 'Cargo.toml'))
        # Reuse dependency compilation, but copy each version out before the next build.
        $target=Join-Path $repo 'target'
        $keys=Join-Path $BuildRoot 'keys'
        Invoke-VelopackTool 'cargo' @('run','-p','yoyo-update-sign','--example','packaging_test_keys','--',$keys) | Out-Host
        Invoke-VelopackTool 'cargo' @('build','-p','yoyo-update-sign','-j','2') | Out-Host
        foreach($version in @($FromVersion,$ToVersion)) {
            $patched=[regex]::Replace($cargoTemplate,'(?m)^version = "[^"]+"',('version = "'+$version+'"'))
            [IO.File]::WriteAllText((Join-Path $source 'Cargo.toml'),$patched)
            $qaBuildStarted=$true
            Invoke-VelopackTool 'cargo' @('build','--manifest-path',(Join-Path $source 'Cargo.toml'),'--target-dir',$target,'-p','yoyovideo-desktop','--features','updater-qa','-j','2') | Out-Host
            $bin=Join-Path $source 'target/debug';New-Item -ItemType Directory -Force -Path $bin | Out-Null
            Copy-Item -LiteralPath (Join-Path $target ('debug/'+$exe)) -Destination $bin -Force
            Invoke-VelopackTool 'pwsh' @('-NoProfile','-File',(Join-Path $source 'scripts/package.ps1'),'-Platform',$Platform,'-Configuration','debug','-RequireRuntime','-ReleaseVersion',$version,'-StageOnly','-SkipBuild') | Out-Host
            $out=Join-Path $BuildRoot $version
            & (Join-Path $PSScriptRoot 'package-velopack.ps1') -Platform $Platform -Version $version -PackageDir (Join-Path $source "dist/YoYoVideo-$Platform") -OutputDir $out -PrepareOnly -QaFixture
            if($LASTEXITCODE){throw 'QA package failed'}
            $env:YOYOVIDEO_UPDATER_PRIVATE_KEY=[IO.File]::ReadAllText((Join-Path $keys 'TEST-ONLY-private.key'))
            $env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD='fixture-only'
            try {
                Invoke-VelopackTool (Join-Path $target ('debug/'+$signerName)) @('sign','--feed',(Join-Path $out "releases.stable-$Platform.json"),'--platform',$Platform,'--version',$version,'--assets-dir',$out,'--output',(Join-Path $out "yoyovideo-update.$Platform.json"),'--public-key',(Join-Path $keys 'TEST-ONLY-public.key')) -Signing | Out-Host
            } finally {$env:YOYOVIDEO_UPDATER_PRIVATE_KEY=$null;$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD=$null}
            & (Join-Path $PSScriptRoot 'verify-velopack-package.ps1') -ReleaseDir $out -Platform $Platform -Version $version -SignerPath (Join-Path $target ('debug/'+$signerName)) -PublicKeyPath (Join-Path $keys 'TEST-ONLY-public.key') -QaFixture
            if($LASTEXITCODE){throw 'QA signature verification failed'}
        }
        @{platform=$Platform;from=$FromVersion;to=$ToVersion} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $BuildRoot 'fixture.json')
        # Restore a non-QA player in the developer target directory and verify it.
        Invoke-VelopackTool 'cargo' @('build','-p','yoyovideo-desktop','--features','mpv-runtime','-j','2') | Out-Host
        if($IsMacOS){Copy-VelopackPayload (Join-Path $repo "third_party/mpv/$Platform/lib") (Join-Path $target 'debug')}
        $env:PATH=(Join-Path $repo "third_party/mpv/$Platform/bin")+[IO.Path]::PathSeparator+$env:PATH
        $null=Get-VelopackBuildInfo -Executable (Join-Path $target ('debug/'+$exe)) -Version (([regex]::Match([IO.File]::ReadAllText((Join-Path $repo 'Cargo.toml')),'(?m)^version = "([^"]+)"')).Groups[1].Value)
    }
    if($qaBuildStarted){
        $normal=[IO.File]::ReadAllBytes((Join-Path $repo ('target/debug/'+$exe)))
        if([Text.Encoding]::ASCII.GetString($normal).Contains('YOYOVIDEO_UPDATER_QA_ROOT')){throw 'QA environment hook leaked into normal build'}
    }
    $qaBuildStarted=$false
    $BuildRoot=(Resolve-Path -LiteralPath $BuildRoot).Path
    if(-not (Test-Path -LiteralPath (Join-Path $BuildRoot 'TEST-ONLY-BUILD'))){throw 'Not a marked QA build root'}
    $meta=[IO.File]::ReadAllText((Join-Path $BuildRoot 'fixture.json')) | ConvertFrom-Json
    if($meta.platform -cne $Platform -or $meta.from -cne $FromVersion -or $meta.to -cne $ToVersion){throw 'Fixture build identity mismatch'}
    Write-Host "QA artifacts: $BuildRoot"
    if($BuildOnly){return}
    $runRoot=Join-Path $BuildRoot ('run-'+[guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runRoot | Out-Null
    [IO.File]::WriteAllText((Join-Path $runRoot 'TEST-ONLY'),"YoYoVideo updater QA fixture v1`n")
    Copy-Item -LiteralPath (Join-Path $BuildRoot 'keys/TEST-ONLY-public.key') -Destination (Join-Path $runRoot 'public.key')
    Copy-VelopackPayload (Join-Path $BuildRoot $ToVersion) (Join-Path $runRoot 'source')
    $layout=Get-UpdaterQaLayout $runRoot $Platform
    $install=$layout.Install;$packages=$layout.Packages
    New-Item -ItemType Directory -Path $install,$packages | Out-Null
    $fromPackage=Join-Path $BuildRoot "$FromVersion/YoYoVideo-$FromVersion-stable-$Platform-full.nupkg"
    $null=Assert-VelopackArchive -Path $fromPackage -Platform $Platform -Version $FromVersion -QaFixture
    Copy-Item -LiteralPath $fromPackage -Destination $packages
    if($IsWindows) {
        $extracted=Join-Path $runRoot 'initial-package';[IO.Compression.ZipFile]::ExtractToDirectory($fromPackage,$extracted)
        Copy-VelopackPayload (Join-Path $extracted 'lib/app') (Join-Path $install 'current')
        Copy-Item -LiteralPath (Join-Path $install 'current/Squirrel.exe') -Destination (Join-Path $install 'Update.exe')
        [IO.File]::WriteAllText((Join-Path $install '.portable'),'QA ONLY')
    } else {
        $release=Join-Path $BuildRoot $FromVersion
        $primary=Join-Path $release (Get-VelopackPrimaryAsset $release $Platform)
        if($IsMacOS){Invoke-VelopackTool '/usr/bin/ditto' @('-x','-k',$primary,$install) | Out-Null}
        else {Copy-Item -LiteralPath $primary -Destination $layout.Launcher;Invoke-VelopackTool 'chmod' @('+x',$layout.Launcher) | Out-Null}
        $env:TMPDIR=Join-Path $runRoot 'tmp';New-Item -ItemType Directory -Path $env:TMPDIR | Out-Null
        if($IsLinux){$env:APPIMAGE_EXTRACT_AND_RUN=if($LinuxMode -eq 'extract'){'1'}else{$null}}
    }
    $env:YOYOVIDEO_UPDATER_QA_ROOT=$runRoot

    & (Join-Path $PSScriptRoot 'run-velopack-upgrade-fixture.ps1') -Root $runRoot -BuildRoot $BuildRoot -Platform $Platform -LinuxMode $LinuxMode -FromVersion $FromVersion -ToVersion $ToVersion -TimeoutSeconds $TimeoutSeconds
} finally {
    if($runRoot) {
        $allowed=[IO.Path]::GetFullPath((Join-Path $runRoot 'installation'))+[IO.Path]::DirectorySeparatorChar
        # Only our verified fixture images may be stopped; never a user's player.
        foreach($process in Get-Process -Name 'yoyovideo-desktop','Update','UpdateMac','UpdateNix' -ErrorAction SilentlyContinue) {
            try {
                if($process.Path -and $process.Path.StartsWith($allowed,[StringComparison]::OrdinalIgnoreCase)) {
                    if('QaProcessSignal' -as [type]) { Write-Host "QA cleanup $($process.Id): $([QaProcessSignal]::Cleanup($process.Id,$process.Path))" }
                    else { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
                }
            } catch { Write-Warning 'Could not clean up one isolated QA process; diagnostics retained' }
        }
        Write-Host "QA events and helper diagnostics retained at $runRoot"
    }
    if($qaBuildStarted) {
        Write-Host 'Restoring a non-QA developer binary after the interrupted fixture build'
        Invoke-VelopackTool 'cargo' @('build','-p','yoyovideo-desktop','--features','mpv-runtime','-j','2') | Out-Host
    }
    foreach($name in $savedEnvironment.Keys){[Environment]::SetEnvironmentVariable($name,$savedEnvironment[$name],'Process')}
}
