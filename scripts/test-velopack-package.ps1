#requires -Version 7.0
[CmdletBinding()]
param([switch]$SkipNativeProbe)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
$root=Join-Path $repo ('.cache/velopack-contract-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$failures=[Collections.Generic.List[string]]::new()
function Check([bool]$Condition,[string]$Message) { if(-not $Condition){throw $Message} }
function Reject([scriptblock]$Action) { $rejected=$false; try { & $Action | Out-Null } catch {$rejected=$true}; Check $rejected 'Expected the operation to fail' }
function Case([string]$Name,[scriptblock]$Action) { try { & $Action; Write-Host "PASS $Name" } catch { $failures.Add("$Name : $($_.Exception.Message)"); Write-Host "FAIL $Name" } }
function Header([string]$Platform) {
    $b=[byte[]]::new(512)
    if($Platform -eq 'windows-x64') {
        $b[0]=0x4d;$b[1]=0x5a;[BitConverter]::GetBytes([int]128).CopyTo($b,60)
        $b[128]=0x50;$b[129]=0x45;[BitConverter]::GetBytes([UInt16]0x8664).CopyTo($b,132)
        [BitConverter]::GetBytes([UInt16]0x20b).CopyTo($b,152)
        [BitConverter]::GetBytes([UInt16]2).CopyTo($b,220)
    } elseif($Platform -eq 'linux-x64') {
        $b[0]=0x7f;$b[1]=0x45;$b[2]=0x4c;$b[3]=0x46;$b[4]=2;$b[5]=1
        [BitConverter]::GetBytes([UInt16]62).CopyTo($b,18)
    } else {
        [BitConverter]::GetBytes([uint32]4277009103).CopyTo($b,0)
        $cpu=if($Platform -eq 'macos-aarch64'){[uint32]16777228}else{[uint32]16777223}
        [BitConverter]::GetBytes($cpu).CopyTo($b,4)
    }
    return ,$b
}
Case 'four targets use distinct channels and correct RIDs' {
    foreach($item in @(@('windows-x64','win-x64'),@('macos-aarch64','osx-arm64'),@('macos-x86_64','osx-x64'),@('linux-x64','linux-x64'))) {
        $t=Get-VelopackTarget $item[0]; Check ($t.Rid -ceq $item[1]) 'Wrong RID'
        Check ($t.Channel -ceq ('stable-'+$item[0])) 'Wrong channel'
    }
    Reject { Get-VelopackTarget '../other' }
}
Case 'arguments disable delta and preserve platform signing choices' {
    foreach($platform in @('windows-x64','macos-aarch64','macos-x86_64','linux-x64')) {
        $a=@(Get-VelopackPackArguments $platform '0.0.1' 'payload' 'out' 'icon' 'notes')
        Check ($a -contains '--skip-updates') 'CLI update checks not disabled'
        $i=[Array]::IndexOf($a,'--delta'); Check ($i -ge 0 -and $a[$i+1] -eq 'None') 'Delta enabled'
        if($platform -like 'macos-*') { $i=[Array]::IndexOf($a,'--signAppIdentity'); Check ($i -ge 0 -and $a[$i+1] -eq '-') 'Missing ad-hoc signature'; Check (-not ($a -contains '--notaryProfile')) 'Unexpected notarization' }
        if($platform -eq 'linux-x64'){Check (-not ($a -contains '--noPortable')) 'AppImage disabled'; Check (-not ($a -contains '--noInst')) 'Linux vpk does not accept noInst'}
        Check (-not ($a -contains '--skipVeloAppCheck')) 'App integration check bypassed'
    }
    Reject { Get-VelopackPackArguments 'windows-x64' 'v0.0.1' 'in' 'out' 'icon' 'notes' }
}
Case 'native headers must match the target' {
    foreach($platform in @('windows-x64','macos-aarch64','macos-x86_64','linux-x64')) {
        $file=Join-Path $root $platform; [IO.File]::WriteAllBytes($file,(Header $platform))
        Assert-VelopackBinary -Path $file -Platform $platform
        $other=if($platform -eq 'windows-x64'){'linux-x64'}else{'windows-x64'}
        Reject { Assert-VelopackBinary -Path $file -Platform $other }
    }
    $bad=Join-Path $root 'bad'; Set-Content -LiteralPath $bad -Value 'not an executable'
    Reject { Assert-VelopackBinary -Path $bad -Platform 'windows-x64' }
}
Case 'artifact names cannot escape the release directory' {
    Assert-VelopackAssetName 'YoYoVideo-0.0.1-stable-windows-x64-full.nupkg'
    foreach($name in @('../outside','/absolute','C:drive.exe','a\b','file name.exe','x..y')) { Reject { Assert-VelopackAssetName $name } }
}
Case 'ICNS is generated from the existing PNG, without Apple tools' {
    $icon=Join-Path $root 'app.icns'
    New-VelopackMacIcon -PngPath (Join-Path $repo 'apps/yoyovideo-desktop/assets/icons/yoyovideo-512.png') -OutputPath $icon
    $b=[IO.File]::ReadAllBytes($icon)
    Check ([Text.Encoding]::ASCII.GetString($b,0,4) -eq 'icns') 'Invalid ICNS header'
    Check ([Text.Encoding]::ASCII.GetString($b,8,4) -eq 'ic09') 'Missing 512px entry'
}
Case 'subprocesses do not inherit signing keys and environment is restored' {
    $fixture=Join-Path $root 'env-probe.ps1'
    'if($env:YOYOVIDEO_UPDATER_PRIVATE_KEY -or $env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD){exit 13}; "OK"; exit 0' | Set-Content -LiteralPath $fixture -Encoding utf8
    $saved=$env:YOYOVIDEO_UPDATER_PRIVATE_KEY
    try { $env:YOYOVIDEO_UPDATER_PRIVATE_KEY='test-only-marker'; $output=Invoke-VelopackTool -FilePath (Get-Command pwsh).Source -Arguments @('-NoProfile','-File',$fixture); Check ($output -contains 'OK') 'Probe failed'; Check ($env:YOYOVIDEO_UPDATER_PRIVATE_KEY -eq 'test-only-marker') 'Environment was not restored' } finally { $env:YOYOVIDEO_UPDATER_PRIVATE_KEY=$saved }
}

. (Join-Path $PSScriptRoot 'velopack-archive.ps1')
function Package-Entries {
    $xml='<package><metadata><id>YoYoVideo</id><title>YoYoVideo</title><version>0.0.1</version><channel>stable-windows-x64</channel><mainExe>yoyovideo-desktop.exe</mainExe><os>win</os><rid>win-x64</rid><machineArchitecture>x64</machineArchitecture></metadata></package>'
    $entries=@{'YoYoVideo.nuspec'=$xml;'lib/app/sq.version'=$xml;'lib/app/yoyovideo-build-info.json'='{"schema":"yoyovideo-build-info-v1","version":"0.0.1","mpv_runtime":true,"updater":true}'}
    foreach($name in @('yoyovideo-desktop.exe','mpv-2.dll','Squirrel.exe','YoYoVideo_ExecutionStub.exe')){$entries['lib/app/'+$name]=Header 'windows-x64'}
    foreach($name in @('README.md','LICENSE','LICENSES/README.md','LICENSES/Velopack-LICENSE.txt','LICENSES/runtime-provenance.md')){$entries['lib/app/'+$name]='fixture notice'}
    return $entries
}
function Zip-Fixture($Entries) {
    $path=Join-Path $root ([Guid]::NewGuid().ToString('N')+'.nupkg')
    $zip=[IO.Compression.ZipFile]::Open($path,[IO.Compression.ZipArchiveMode]::Create)
    try {foreach($name in $Entries.Keys){$entry=$zip.CreateEntry($name);$stream=$entry.Open();try{$bytes=if($Entries[$name] -is [byte[]]){$Entries[$name]}else{[Text.Encoding]::UTF8.GetBytes($Entries[$name])};$stream.Write($bytes,0,$bytes.Length)}finally{$stream.Dispose()}}}finally{$zip.Dispose()}
    return $path
}
Case 'package metadata and bundled runtime are checked inside the actual archive' {
    $entries=Package-Entries; $zip=Zip-Fixture $entries
    $metadata=Assert-VelopackArchive -Path $zip -Platform 'windows-x64' -Version '0.0.1'
    Check ($metadata.Version -eq '0.0.1') 'Version not read from archive'
    $entries.Remove('lib/app/mpv-2.dll'); Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
}
Case 'wrong version and architecture cannot pass archive validation' {
    $entries=Package-Entries; $entries['YoYoVideo.nuspec']=$entries['YoYoVideo.nuspec'].Replace('<version>0.0.1</version>','<version>0.0.2</version>')
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
    $entries=Package-Entries; $entries['lib/app/yoyovideo-desktop.exe']=Header 'linux-x64'
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
}
Case 'archive paths and XML entities are rejected without extraction' {
    $entries=Package-Entries; $entries['../outside']='must not extract'
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
    $entries=Package-Entries; $entries['YoYoVideo.nuspec']='<!DOCTYPE package [<!ENTITY x SYSTEM "file:///never-read">]><package><metadata>&x;</metadata></package>'
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
    Check (-not (Test-Path -LiteralPath (Join-Path (Split-Path $root) 'outside'))) 'Archive traversal wrote outside fixture'
}

if($IsWindows -and -not $SkipNativeProbe) { Case 'build probe reports real version and runtime without exposing signing credentials' {
    $bin=Join-Path $root 'real-player';New-Item -ItemType Directory -Path $bin | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo 'target/debug/yoyovideo-desktop.exe') -Destination (Join-Path $bin 'yoyovideo-desktop.exe')
    Copy-Item -LiteralPath (Join-Path $repo 'third_party/mpv/windows-x64/bin/mpv-2.dll') -Destination (Join-Path $bin 'mpv-2.dll')
    $info=Get-VelopackBuildInfo -Executable (Join-Path $bin 'yoyovideo-desktop.exe') -Version '0.0.1'
    Check ($info.version -eq '0.0.1' -and $info.mpv_runtime -eq $true -and $info.updater -eq $true) 'Incorrect build metadata'
    Reject {Get-VelopackBuildInfo -Executable (Join-Path $bin 'yoyovideo-desktop.exe') -Version '0.0.2'}
    Reject {Get-VelopackBuildInfo -Executable (Join-Path $root 'windows-x64') -Version '0.0.1'}
}

}

. (Join-Path $PSScriptRoot 'appimage-common.ps1')
Case 'AppImage provenance rejects missing, altered and host ABI libraries' {
    $bin=Join-Path $root 'linux-provenance';New-Item -ItemType Directory -Path (Join-Path $bin 'LICENSES') | Out-Null
    $lib=Join-Path $bin 'libmpv.so.1';[IO.File]::WriteAllBytes($lib,(Header 'linux-x64'))
    'test license' | Set-Content -LiteralPath (Join-Path $bin 'LICENSES/mpv-copyright.txt')
    $hash=(Get-FileHash -LiteralPath $lib -Algorithm SHA256).Hash.ToLowerInvariant()
    @{library='libmpv.so.1';sha256=$hash} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $bin 'LICENSES/runtime-source.json')
    $record=@{schema='yoyovideo-appimage-runtime-v1';version='0.0.1';baseline='ubuntu-22.04-x86_64';bundled=@(@{file='libmpv.so.1';sha256=$hash;package='libmpv1';version='0.34.1';license='mpv-copyright.txt'})}
    $record | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $bin 'LICENSES/appimage-runtime.json')
    Assert-AppImageRuntimeProvenance $bin '0.0.1'
    Reject {Assert-AppImageRuntimeProvenance $bin '0.0.2'}
    [IO.File]::WriteAllBytes($lib,[byte[]]::new(128));Reject {Assert-AppImageRuntimeProvenance $bin '0.0.1'}
    [IO.File]::WriteAllBytes($lib,(Header 'linux-x64'))
    [IO.File]::WriteAllBytes((Join-Path $bin 'libc.so.6'),(Header 'linux-x64'));Reject {Assert-AppImageRuntimeProvenance $bin '0.0.1'}
}
Case 'macOS resources stay outside MacOS binaries and symlinks are constrained' {
    $windows=Package-Entries;$entries=@{}
    foreach($key in $windows.Keys){if($key.StartsWith('lib/app/') -and $key -notmatch '\.exe$|\.dll$'){$entries[$key.Replace('lib/app/','lib/app/Contents/Resources/')]=$windows[$key]}}
    $xml=$windows['YoYoVideo.nuspec'].Replace('stable-windows-x64','stable-macos-aarch64').Replace('yoyovideo-desktop.exe','Contents/MacOS/yoyovideo-desktop').Replace('<os>win</os>','<os>osx</os>').Replace('<rid>win-x64</rid>','<rid>osx-arm64</rid>').Replace('<machineArchitecture>x64','<machineArchitecture>arm64')
    $entries['YoYoVideo.nuspec']=$xml
    $entries['lib/app/Contents/Resources/sq.version']=$xml
    foreach($name in @('yoyovideo-desktop','libmpv.dylib','UpdateMac')){$entries['lib/app/Contents/MacOS/'+$name]=Header 'macos-aarch64'}
    $entries['lib/app/Contents/Info.plist']='<plist/>'
    $entries['lib/app/Contents/MacOS/sq.version']='../Resources/sq.version'
    $zipPath=Zip-Fixture $entries
    $zip=[IO.Compression.ZipFile]::Open($zipPath,[IO.Compression.ZipArchiveMode]::Update)
    try {$zip.GetEntry('lib/app/Contents/MacOS/sq.version').ExternalAttributes=(0xa1ff -shl 16)}finally{$zip.Dispose()}
    $null=Assert-VelopackArchive -Path $zipPath -Platform 'macos-aarch64' -Version '0.0.1'
    $entries['lib/app/Contents/MacOS/sq.version']='../../../../outside'
    $zipPath=Zip-Fixture $entries;$zip=[IO.Compression.ZipFile]::Open($zipPath,[IO.Compression.ZipArchiveMode]::Update)
    try {$zip.GetEntry('lib/app/Contents/MacOS/sq.version').ExternalAttributes=(0xa1ff -shl 16)}finally{$zip.Dispose()}
    Reject {Assert-VelopackArchive -Path $zipPath -Platform 'macos-aarch64' -Version '0.0.1'}
}

Case 'installed sq.version must match the full package identity' {
    $entries=Package-Entries
    $entries['lib/app/sq.version']=$entries['lib/app/sq.version'].Replace('<version>0.0.1</version>','<version>0.0.2</version>')
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
}
Case 'test-only packages cannot pass the production release validator' {
    $entries=Package-Entries
    $entries['YoYoVideo.nuspec']=$entries['YoYoVideo.nuspec'].Replace('<title>YoYoVideo</title>','<title>YoYoVideo QA ONLY</title>')
    $entries['lib/app/sq.version']=$entries['YoYoVideo.nuspec']
    $entries['lib/app/yoyovideo-build-info.json']='{"schema":"yoyovideo-build-info-v1","version":"0.0.1","mpv_runtime":true,"updater":true,"updater_qa":true}'
    $entries['lib/app/YoYoVideo QA ONLY_ExecutionStub.exe']=$entries['lib/app/YoYoVideo_ExecutionStub.exe']
    $entries.Remove('lib/app/YoYoVideo_ExecutionStub.exe')
    $null=Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1' -QaFixture
    $args=@(Get-VelopackPackArguments 'windows-x64' '0.0.1' 'in' 'out' 'icon' 'notes' -QaFixture)
    Check ($args -contains 'YoYoVideo QA ONLY' -and $args[[Array]::IndexOf($args,'--shortcuts')+1] -eq 'None') 'QA packaging must have a distinct title and no shortcuts'
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
}
Case 'privacy QA packages cannot pass production validation' {
    $entries=Package-Entries
    $entries['lib/app/yoyovideo-build-info.json']='{"schema":"yoyovideo-build-info-v1","version":"0.0.1","mpv_runtime":true,"updater":true,"privacy_qa":true}'
    Reject {Assert-VelopackArchive -Path (Zip-Fixture $entries) -Platform 'windows-x64' -Version '0.0.1'}
}
Case 'native Linux nuspec names the AppDir executable' {
    $entries=Package-Entries
    $xml=$entries['YoYoVideo.nuspec'].Replace('stable-windows-x64','stable-linux-x64').Replace('win-x64','linux-x64').Replace('<os>win</os>','<os>linux</os>').Replace('yoyovideo-desktop.exe','usr/bin/yoyovideo-desktop')
    $linux=@{'YoYoVideo.nuspec'=$xml;'lib/app/YoYoVideo.AppImage'=(Header 'linux-x64')}
    $null=Assert-VelopackArchive -Path (Zip-Fixture $linux) -Platform linux-x64 -Version 0.0.1
}
Case 'Velopack encoded symlinks cannot escape the bundle' {
    $entries=@{'lib/app/Contents/MacOS/sq.version.__symlink'='../../../../outside'}
    $path=Zip-Fixture $entries;$zip=[IO.Compression.ZipFile]::OpenRead($path)
    try {Reject {Assert-VelopackZipPaths $zip 'macos-aarch64'}}finally{$zip.Dispose()}
    $entries['lib/app/Contents/MacOS/sq.version.__symlink']='../Resources/sq.version'
    $path=Zip-Fixture $entries;$zip=[IO.Compression.ZipFile]::OpenRead($path)
    try {Assert-VelopackZipPaths $zip 'macos-aarch64'}finally{$zip.Dispose()}
}
Case 'AppImage runtime output cannot replace or duplicate build metadata' {
    $json='{"schema":"yoyovideo-build-info-v1","version":"0.0.1","mpv_runtime":true,"updater":true}'
    $info=ConvertFrom-VelopackBuildInfoText ("Environment runtime notice`n"+$json+"`n") -AppImage
    Check ($info.version -eq '0.0.1') 'Did not read the app report'
    Reject {ConvertFrom-VelopackBuildInfoText "Error: unable to mount AppImage" -AppImage}
    Reject {ConvertFrom-VelopackBuildInfoText ($json+"`n"+$json) -AppImage}
    Reject {ConvertFrom-VelopackBuildInfoText ("notice`n"+$json)}
}
function Reject-WithMessage([scriptblock]$Action,[string]$Expected) {
    $message=$null
    try { & $Action | Out-Null } catch { $message=$_.Exception.Message }
    Check ($null -ne $message -and $message.Contains($Expected)) "Expected rejection containing '$Expected', got '$message'"
}
Case 'CLI version verification rejects unpinned tools at the subprocess boundary' {
    # Replace only the external CLI call; keep the real version validation and arguments.
    function Invoke-VelopackTool([string]$FilePath,[string[]]$Arguments) {
        Check ($FilePath -ceq 'fixture-vpk') 'Unexpected CLI executable'
        Check (($Arguments -join '|') -ceq '--legacyConsole|--skip-updates|--help') 'Unexpected CLI probe arguments'
        return $script:FixtureCliHelp
    }
    foreach($help in @('Velopack CLI 1.2.160,','Velopack CLI 1.2.1610,','unrecognized help','')) {
        $script:FixtureCliHelp=$help
        Reject-WithMessage {Assert-VelopackCli 'fixture-vpk'} 'vpk 1.2.161 is required'
    }
    $script:FixtureCliHelp='Velopack CLI 1.2.161, current build'
    Assert-VelopackCli 'fixture-vpk'
}
Case 'actual package script plans every target and rejects missing runtime or public key' {
    $scriptPath=Join-Path $PSScriptRoot 'package-velopack.ps1'
    foreach($item in @(@('windows-x64','win-x64','yoyovideo-desktop.exe','mpv-2.dll'),@('macos-aarch64','osx-arm64','yoyovideo-desktop','libmpv.dylib'),@('macos-x86_64','osx-x64','yoyovideo-desktop','libmpv.dylib'),@('linux-x64','linux-x64','yoyovideo-desktop',''))) {
        $platform=$item[0];$stage=Join-Path $root ('plan-'+$platform)
        foreach($dir in @('bin','docs','LICENSES')){New-Item -ItemType Directory -Path (Join-Path $stage $dir) -Force | Out-Null}
        foreach($name in @('README.md','RELEASE-NOTES.md','LICENSES/README.md','LICENSES/runtime-provenance.md','docs/runtime-dependencies.md','LICENSES/runtime-source.json')){[IO.File]::WriteAllText((Join-Path $stage $name),'fixture only')}
        [IO.File]::WriteAllBytes((Join-Path $stage ('bin/'+$item[2])),(Header $platform))
        if($item[3]){[IO.File]::WriteAllBytes((Join-Path $stage ('bin/'+$item[3])),(Header $platform))}
        $params=@{Platform=$platform;Version='0.0.1';PackageDir=$stage;OutputDir=(Join-Path $root ('output-'+$platform))}
        $plan=(& $scriptPath @params -PrepareOnly -PlanOnly) | ConvertFrom-Json
        Check ($plan.Platform -ceq $platform -and $plan.Runtime -ceq $item[1] -and $plan.Channel -ceq ('stable-'+$platform)) 'Wrong planned target'
        foreach($pair in @(@('--packId','YoYoVideo'),@('--packVersion','0.0.1'),@('--mainExe',$item[2]),@('--channel',('stable-'+$platform)),@('--runtime',$item[1]),@('--delta','None'),@('--outputDir',$params.OutputDir))) {
            $index=[Array]::IndexOf($plan.Arguments,$pair[0]);Check ($index -ge 0 -and $plan.Arguments[$index+1] -ceq $pair[1]) "Wrong planned $($pair[0])"
        }
        Reject-WithMessage {& $scriptPath @params -PrepareOnly -PlanOnly -PublicKeyPath (Join-Path $root 'missing.pub')} 'Pinned public key is required'
        if($item[3]) {
            Remove-Item -LiteralPath (Join-Path $stage ('bin/'+$item[3]))
            Reject-WithMessage {& $scriptPath @params -PrepareOnly -PlanOnly} $item[3]
            [IO.File]::WriteAllBytes((Join-Path $stage ('bin/'+$item[3])),(Header $platform))
        }
        Check (-not (Test-Path -LiteralPath $params.OutputDir)) 'Planning must not create release artifacts'
    }
}
Case 'actual signed packaging refuses absent credentials before probing any executable' {
    $platform=if($IsWindows){'windows-x64'}elseif($IsMacOS){'macos-aarch64'}else{'linux-x64'}
    $stage=Join-Path $root ('plan-'+$platform)
    $savedKey=$env:YOYOVIDEO_UPDATER_PRIVATE_KEY;$savedPassword=$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD
    try {
        $env:YOYOVIDEO_UPDATER_PRIVATE_KEY=$null;$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD=$null
        Reject-WithMessage {& (Join-Path $PSScriptRoot 'package-velopack.ps1') -Platform $platform -Version 0.0.1 -PackageDir $stage -OutputDir (Join-Path $root 'no-signing')} 'Signing credentials are required'
    } finally {$env:YOYOVIDEO_UPDATER_PRIVATE_KEY=$savedKey;$env:YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD=$savedPassword}
}
Write-Host "Fixture diagnostics retained under $root"
if($failures.Count){ throw ($failures -join [Environment]::NewLine) }
Write-Host 'Velopack packaging contracts passed.'
