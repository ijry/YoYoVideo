#requires -Version 7.0
Set-StrictMode -Version Latest
$script:VelopackVersion='1.2.161'
$script:SigningEnvironment=@('YOYOVIDEO_UPDATER_PRIVATE_KEY','YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD','TAURI_SIGNING_PRIVATE_KEY','TAURI_SIGNING_PRIVATE_KEY_PASSWORD','GH_TOKEN','GITHUB_TOKEN')
function Get-VelopackTarget([string]$Platform) {
    $targets=@{
        'windows-x64'=@{Rid='win-x64';Os='win';Arch='x64';Exe='yoyovideo-desktop.exe'}
        'macos-aarch64'=@{Rid='osx-arm64';Os='osx';Arch='arm64';Exe='yoyovideo-desktop'}
        'macos-x86_64'=@{Rid='osx-x64';Os='osx';Arch='x64';Exe='yoyovideo-desktop'}
        'linux-x64'=@{Rid='linux-x64';Os='linux';Arch='x64';Exe='yoyovideo-desktop'}
    }
    if(-not $targets.ContainsKey($Platform)){throw 'Unsupported Velopack platform'}
    $target=$targets[$Platform]; $target.Channel='stable-'+$Platform; return $target
}
function Assert-VelopackVersion([string]$Version) {
    if($Version.Length -gt 64 -or $Version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'){throw 'Expected a stable version without a leading v'}
}
function Assert-VelopackHost([string]$Platform) {
    $target=Get-VelopackTarget $Platform
    if(($target.Os -eq 'win' -and -not $IsWindows) -or ($target.Os -eq 'osx' -and -not $IsMacOS) -or ($target.Os -eq 'linux' -and -not $IsLinux)){throw 'This package must be built and verified on its native OS'}
}
function Get-VelopackPackArguments([string]$Platform,[string]$Version,[string]$PackRoot,[string]$OutputDir,[string]$IconPath,[string]$NotesPath,[switch]$QaFixture) {
    Assert-VelopackVersion $Version; $t=Get-VelopackTarget $Platform
    $title=if($QaFixture){'YoYoVideo QA ONLY'}else{'YoYoVideo'}
    $result=@('--skip-updates','--yes','--legacyConsole','pack','--packId','YoYoVideo','--packTitle',$title,'--packAuthors','YoYoVideo contributors','--packVersion',$Version,'--packDir',$PackRoot,'--mainExe',$t.Exe,'--channel',$t.Channel,'--runtime',$t.Rid,'--outputDir',$OutputDir,'--delta','None','--icon',$IconPath,'--releaseNotes',$NotesPath)
    if($Platform -eq 'windows-x64'){$result+=@('--noPortable','--shortcuts',$(if($QaFixture){'None'}else{'Desktop,StartMenuRoot'}))}
    elseif($Platform -like 'macos-*'){$result+=@('--noInst','--signAppIdentity','-')}
    else {$result+=@('--noInst','--categories','AudioVideo;Player')}
    return $result
}
function Assert-VelopackAssetName([string]$Name) {
    if($Name -cnotmatch '^[A-Za-z0-9][A-Za-z0-9_.-]{0,239}$' -or $Name.Contains('..')){throw 'Unsafe release asset filename'}
    $stem=($Name.Split('.')[0]).ToUpperInvariant()
    if($stem -match '^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$'){throw 'Reserved release asset filename'}
}
function Read-VelopackPrefix([IO.Stream]$Stream,[int]$Limit=65536) {
    $buffer=[byte[]]::new($Limit); $count=0
    while($count -lt $Limit){$n=$Stream.Read($buffer,$count,$Limit-$count);if($n -eq 0){break};$count+=$n}
    $result=[byte[]]::new($count);[Array]::Copy($buffer,$result,$count);return ,$result
}
function Assert-VelopackBinary {
    param([string]$Path,[string]$Platform,[byte[]]$Bytes,[switch]$Library)
    if(-not $PSBoundParameters.ContainsKey('Bytes')) {
        $stream=[IO.File]::OpenRead($Path);try{$Bytes=Read-VelopackPrefix $stream}finally{$stream.Dispose()}
    }
    $null=Get-VelopackTarget $Platform
    if($Bytes.Length -lt 64){throw 'Native binary header is truncated'}
    if($Platform -eq 'windows-x64') {
        if([BitConverter]::ToUInt16($Bytes,0) -ne 0x5a4d){throw 'Expected PE executable'}
        $offset=[BitConverter]::ToInt32($Bytes,60)
        if($offset -lt 64 -or $offset -gt $Bytes.Length-96){throw 'Invalid PE header offset'}
        if([BitConverter]::ToUInt32($Bytes,$offset) -ne 0x4550 -or [BitConverter]::ToUInt16($Bytes,$offset+4) -ne 0x8664 -or [BitConverter]::ToUInt16($Bytes,$offset+24) -ne 0x20b){throw 'Expected x64 PE executable'}
        if(-not $Library -and [BitConverter]::ToUInt16($Bytes,$offset+92) -ne 2){throw 'Player must use the Windows GUI subsystem'}
    } elseif($Platform -eq 'linux-x64') {
        if($Bytes[0] -ne 0x7f -or [Text.Encoding]::ASCII.GetString($Bytes,1,3) -ne 'ELF' -or $Bytes[4] -ne 2 -or $Bytes[5] -ne 1 -or [BitConverter]::ToUInt16($Bytes,18) -ne 62){throw 'Expected x86_64 ELF executable'}
    } else {
        $cpu=if($Platform -eq 'macos-aarch64'){16777228}else{16777223}
        if([BitConverter]::ToUInt32($Bytes,0) -ne 4277009103 -or [BitConverter]::ToUInt32($Bytes,4) -ne $cpu){throw 'Expected thin Mach-O of the selected architecture'}
    }
}
function New-VelopackMacIcon {
    param([string]$PngPath,[string]$OutputPath)
    $png=[IO.File]::ReadAllBytes($PngPath)
    if($png.Length -lt 24 -or [Convert]::ToHexString($png[0..7]) -ne '89504E470D0A1A0A' -or [Convert]::ToHexString($png[16..23]) -ne '0000020000000200'){throw 'Expected a 512x512 PNG icon'}
    $bytes=[byte[]]::new($png.Length+16);[Text.Encoding]::ASCII.GetBytes('icns').CopyTo($bytes,0)
    $size=[BitConverter]::GetBytes([uint32]$bytes.Length);[Array]::Reverse($size);$size.CopyTo($bytes,4)
    [Text.Encoding]::ASCII.GetBytes('ic09').CopyTo($bytes,8)
    $size=[BitConverter]::GetBytes([uint32]($png.Length+8));[Array]::Reverse($size);$size.CopyTo($bytes,12);$png.CopyTo($bytes,16)
    $file=[IO.File]::Open($OutputPath,[IO.FileMode]::CreateNew);try{$file.Write($bytes,0,$bytes.Length)}finally{$file.Dispose()}
}
function Invoke-VelopackTool {
    param([string]$FilePath,[string[]]$Arguments,[switch]$Signing)
    $saved=@{}
    try {
        foreach($name in $script:SigningEnvironment) {
            if($Signing -and $name -in @('YOYOVIDEO_UPDATER_PRIVATE_KEY','YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD')){continue}
            $saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process');[Environment]::SetEnvironmentVariable($name,$null,'Process')
        }
        $global:LASTEXITCODE=0
        $output=@(& $FilePath @Arguments 2>&1)
        if($LASTEXITCODE -ne 0){throw "External command failed: $([IO.Path]::GetFileName($FilePath)) (exit $LASTEXITCODE)"}
        return $output
    } finally { foreach($name in $saved.Keys){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')} }
}
function Assert-VelopackCli([string]$VpkPath) {
    $help=(Invoke-VelopackTool $VpkPath @('--legacyConsole','--skip-updates','--help')) -join [Environment]::NewLine
    if($help -notmatch 'Velopack CLI 1\.2\.161(?:,|\s)'){throw 'vpk 1.2.161 is required; do not use an unpinned tool'}
}
function Get-VelopackBuildInfo {
    param([string]$Executable,[string]$Version,[switch]$QaFixture)
    Assert-VelopackVersion $Version
    # Do not launch an older player that would treat the probe flag as a media argument.
    $file=[IO.File]::OpenRead($Executable);$found=$false;$tail='';$buffer=[byte[]]::new(65536)
    try {while(($count=$file.Read($buffer,0,$buffer.Length)) -gt 0){$text=$tail+[Text.Encoding]::ASCII.GetString($buffer,0,$count);if($text.Contains('yoyovideo-build-info-v1')){$found=$true;break};$tail=$text.Substring([Math]::Max(0,$text.Length-32))}}finally{$file.Dispose()}
    if(-not $found){throw 'Player lacks build-info support; rebuild before packaging'}
    $start=[Diagnostics.ProcessStartInfo]::new($Executable);$start.ArgumentList.Add('--build-info')
    $start.UseShellExecute=$false;$start.CreateNoWindow=$true;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
    foreach($name in $script:SigningEnvironment){[void]$start.Environment.Remove($name)}
    $process=[Diagnostics.Process]::Start($start)
    try {
        $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
        if(-not $process.WaitForExit(30000)){$process.Kill();$process.WaitForExit();throw 'Build-info probe timed out'}
        if($process.ExitCode -ne 0){throw "Build-info probe failed (exit $($process.ExitCode))"}
        $text=$stdout.GetAwaiter().GetResult();$null=$stderr.GetAwaiter().GetResult()
        if($text.Length -gt 65536){throw 'Oversized build-info response'}
        $info=$text | ConvertFrom-Json
        if($info.schema -cne 'yoyovideo-build-info-v1' -or $info.version -cne $Version -or $info.mpv_runtime -ne $true -or $info.updater -ne $true){throw 'Player version/runtime/updater does not match requested package'}
        $isQa=($info.PSObject.Properties.Name -contains 'updater_qa') -and $info.updater_qa -eq $true
        if($isQa -ne [bool]$QaFixture){throw 'QA and production player builds must not be mixed'}
        return $info
    } finally {$process.Dispose()}
}

function Resolve-VelopackTool([string]$Name,[string]$Explicit,[string]$DefaultPath) {
    if($Explicit){if(-not (Test-Path -LiteralPath $Explicit -PathType Leaf)){throw "Missing tool: $Name"};return [IO.Path]::GetFullPath($Explicit)}
    $command=Get-Command $Name -ErrorAction SilentlyContinue
    if($command){return $command.Source}
    if(Test-Path -LiteralPath $DefaultPath -PathType Leaf){return [IO.Path]::GetFullPath($DefaultPath)}
    throw "Missing $Name. Install the pinned tool or pass its explicit path."
}
function Copy-VelopackPayload([string]$Source,[string]$Destination) {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    foreach($item in Get-ChildItem -LiteralPath $Source -Force) {
        if($item.Name -eq '.gitkeep' -or $item.Name -like '*.pdb' -or $item.Name -like '*.dSYM'){continue}
        if($item.PSIsContainer){if($item.LinkType){throw 'Directory links are not allowed in staging'};Copy-VelopackPayload $item.FullName (Join-Path $Destination $item.Name)}
        else {$sourceFile=if($item.LinkType){$item.ResolveLinkTarget($true).FullName}else{$item.FullName};Copy-Item -LiteralPath $sourceFile -Destination (Join-Path $Destination $item.Name)}
    }
}
function Assert-VelopackStaging([string]$PackageDir,[string]$Platform) {
    $target=Get-VelopackTarget $Platform
    foreach($name in @('README.md','RELEASE-NOTES.md','LICENSES/README.md','LICENSES/runtime-provenance.md','docs/runtime-dependencies.md')){if(-not (Test-Path -LiteralPath (Join-Path $PackageDir $name) -PathType Leaf)){throw "Missing staging file: $name"}}
    Assert-VelopackBinary -Path (Join-Path $PackageDir ('bin/'+$target.Exe)) -Platform $Platform
    if($Platform -eq 'windows-x64'){Assert-VelopackBinary -Path (Join-Path $PackageDir 'bin/mpv-2.dll') -Platform $Platform -Library}
    elseif($Platform -like 'macos-*'){Assert-VelopackBinary -Path (Join-Path $PackageDir 'bin/libmpv.dylib') -Platform $Platform -Library}
    elseif(-not (Test-Path -LiteralPath (Join-Path $PackageDir 'LICENSES/runtime-source.json'))){throw 'Missing Linux dependency provenance'}
}
function Initialize-VelopackDotnet([string]$RepoRoot) {
    $env:DOTNET_CLI_TELEMETRY_OPTOUT='1';$env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE='1';$env:DOTNET_GENERATE_ASPNET_CERTIFICATE='false'
    if(-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
        $local=Join-Path $RepoRoot '.cache/tools/dotnet-8.0.425'
        if(Test-Path -LiteralPath (Join-Path $local 'dotnet.exe')){$env:DOTNET_ROOT=$local;$env:PATH=$local+[IO.Path]::PathSeparator+$env:PATH}
    }
    if(-not $env:DOTNET_CLI_HOME){$env:DOTNET_CLI_HOME=Join-Path $RepoRoot '.cache/tools/dotnet-home'}
}
function Get-VelopackPrimaryAsset([string]$ReleaseDir,[string]$Platform) {
    $t=Get-VelopackTarget $Platform
    $index=[IO.File]::ReadAllText((Join-Path $ReleaseDir ('assets.'+$t.Channel+'.json'))) | ConvertFrom-Json
    $primary=@($index | Where-Object {if($Platform -eq 'windows-x64'){$_.Type -eq 'Installer' -and $_.RelativeFileName -like '*.exe'}elseif($Platform -like 'macos-*'){$_.Type -eq 'Portable' -and $_.RelativeFileName -like '*.zip'}else{$_.Type -eq 'Portable' -and $_.RelativeFileName -like '*.AppImage'}})
    if($primary.Count -ne 1){throw 'Expected exactly one installable artifact for this platform'}
    foreach($item in $index){Assert-VelopackAssetName $item.RelativeFileName;if(-not (Test-Path -LiteralPath (Join-Path $ReleaseDir $item.RelativeFileName) -PathType Leaf)){throw 'Asset index references a missing file'}}
    return $primary[0].RelativeFileName
}