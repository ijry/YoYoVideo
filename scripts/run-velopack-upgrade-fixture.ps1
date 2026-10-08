#requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Root,[Parameter(Mandatory)][string]$BuildRoot,[string]$FromVersion='0.0.1',[string]$ToVersion='0.0.2',[int]$TimeoutSeconds=180)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
if(-not $IsWindows){throw 'This native runner currently implements Windows portable-layout upgrades only'}
$Root=(Resolve-Path -LiteralPath $Root).Path
if([IO.File]::ReadAllText((Join-Path $Root 'TEST-ONLY')) -cne "YoYoVideo updater QA fixture v1`n"){throw 'Missing QA root marker'}
$install=Join-Path $Root 'installation';$main=Join-Path $install 'current/yoyovideo-desktop.exe'
if(-not (Test-Path -LiteralPath (Join-Path $install '.portable'))){throw 'Refusing a profile-affecting installation'}
if($env:YOYOVIDEO_UPDATER_QA_ROOT -cne $Root){throw 'QA process root is not configured'}
$null=Get-VelopackBuildInfo -Executable $main -Version $FromVersion -QaFixture
if (-not ('QaProcessSignal' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class QaProcessSignal {
 [DllImport("kernel32.dll")] public static extern uint WaitForSingleObject(IntPtr process, uint ms);
 [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
 [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)] static extern bool QueryFullProcessImageName(IntPtr p, uint flags, StringBuilder path, ref uint size);
 [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr p, uint code);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr p);
 public static string Cleanup(int pid, string expectedPath) {
  IntPtr p=OpenProcess(0x00101001,false,pid);
  if(p==IntPtr.Zero) return "not accessible: "+Marshal.GetLastWin32Error();
  try {
   var path=new StringBuilder(32768);uint size=32768;
   if(!QueryFullProcessImageName(p,0,path,ref size)) return "identity unavailable; skipped";
   if(!String.Equals(expectedPath,path.ToString(),StringComparison.OrdinalIgnoreCase)) return "identity changed; skipped";
   if(WaitForSingleObject(p,0)==0) return "already terminated";
   if(!TerminateProcess(p,1)) return "termination refused: "+Marshal.GetLastWin32Error();
   return "termination wait: "+WaitForSingleObject(p,5000);
  } finally {CloseHandle(p);}
 }
}
'@
}
$script:QaSeq=0
$events=Join-Path $Root 'events.jsonl'
function Normalize-QaPath([string]$Value) {
    if($Value.StartsWith('\\?\')){$Value=$Value.Substring(4)}
    return [IO.Path]::GetFullPath($Value)
}
function Read-QaState([int]$TargetPid=0) {
    if(-not (Test-Path -LiteralPath $events)){return $null}
    $stream=[IO.File]::Open($events,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
    $reader=[IO.StreamReader]::new($stream)
    try {$lines=$reader.ReadToEnd().Split([char]10)}finally{$reader.Dispose()}
    for($i=$lines.Length-1;$i -ge 0;$i--) {
        try {$state=$lines[$i] | ConvertFrom-Json -ErrorAction Stop}catch{continue}
        if($null -ne $state -and ($TargetPid -eq 0 -or $state.pid -eq $TargetPid)) {
            if($state.qa_error){throw "QA control error: $($state.qa_error)"}
            return $state
        }
    }
    return $null
}
function Wait-Qa([scriptblock]$Predicate,[string]$Description,[int]$TargetPid=0,[int]$Seconds=$TimeoutSeconds) {
    $until=[DateTime]::UtcNow.AddSeconds($Seconds);$last=$null
    do {
        $last=Read-QaState $TargetPid
        if($last -and (& $Predicate $last)){return $last}
        Start-Sleep -Milliseconds 200
    } while([DateTime]::UtcNow -lt $until)
    Get-ChildItem -LiteralPath $Root -Filter '*.log' -File | ForEach-Object { Write-Host $_.Name; Get-Content -LiteralPath $_.FullName -Tail 30 | Out-Host }
    throw "Timeout: $Description. Last state: $($last | ConvertTo-Json -Depth 6 -Compress)"
}
function Send-Qa([int]$TargetPid,[string]$Command,[string]$MediaPath) {
    $script:QaSeq++
    $request=@{seq=$script:QaSeq;pid=$TargetPid;command=$Command}
    if($MediaPath){$request.path=$MediaPath}
    $temp=Join-Path $Root 'request.tmp'
    $request | ConvertTo-Json | Set-Content -LiteralPath $temp -Encoding utf8NoBOM
    Move-Item -LiteralPath $temp -Destination (Join-Path $Root 'request.json') -Force
    return $script:QaSeq
}
function Start-Qa {
    $tag=[guid]::NewGuid().ToString('N')
    $process=Start-Process -FilePath $main -WorkingDirectory $Root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $Root "$tag.stdout.log") -RedirectStandardError (Join-Path $Root "$tag.stderr.log")
    $null=$process.Handle # Retain the real kernel process handle before termination.
    $null=Wait-Qa {param($s) $s.playback.ready} 'player initialization' $process.Id
    return $process
}
function Wait-QaExit($Process,[int]$Milliseconds=30000) {
    $watch=[Diagnostics.Stopwatch]::StartNew()
    do {
        $status=[QaProcessSignal]::WaitForSingleObject($Process.Handle,0)
        if($status -eq 0){break}
        if($status -ne 258){throw 'Could not inspect the kernel process signal'}
        Start-Sleep -Milliseconds 100
    } while($watch.ElapsedMilliseconds -lt $Milliseconds)
    if($status -ne 0){throw 'Player kernel process did not terminate within the deadline (exit code alone is insufficient)'}
    $Process.Refresh()
    if($Process.ExitCode -ne 0){throw "Player exited with $($Process.ExitCode)"}
    Write-Host "QA process $($Process.Id) fully terminated in $($watch.ElapsedMilliseconds) ms (exit=$($Process.ExitCode))"
}
function Close-Qa($Process) {
    $null=Send-Qa $Process.Id 'close'
    Wait-QaExit $Process
}
function Ready-Qa($Process) {
    $seq=Send-Qa $Process.Id 'check'
    $null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 4} 'authenticated update available' $Process.Id
    $seq=Send-Qa $Process.Id 'download'
    $null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 6} 'verified update download' $Process.Id
}
# A real, long-enough WAV decoded by libmpv (ao=null in this test feature).
$media=Join-Path $Root 'media.wav';$pcm=[byte[]]::new(8000*2*600)
$stream=[IO.File]::Create($media);$writer=[IO.BinaryWriter]::new($stream)
try {
    $writer.Write([Text.Encoding]::ASCII.GetBytes('RIFF'));$writer.Write([int](36+$pcm.Length));$writer.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt '));$writer.Write([int]16);$writer.Write([short]1);$writer.Write([short]1);$writer.Write([int]8000);$writer.Write([int]16000);$writer.Write([short]2);$writer.Write([short]16);$writer.Write([Text.Encoding]::ASCII.GetBytes('data'));$writer.Write([int]$pcm.Length);$writer.Write($pcm)
}finally{$writer.Dispose()}
$old=Start-Qa
$seq=Send-Qa $old.Id 'disable_auto'
$null=Wait-Qa {param($s) $s.seq -eq $seq -and -not $s.automatic_check} 'disable automatic checks' $old.Id
$seq=Send-Qa $old.Id 'open' 'media.wav'
$played=Wait-Qa {param($s) $s.seq -eq $seq -and $s.playback.position -ge 2 -and $s.playback.duration -gt 100} 'real playback advances' $old.Id
if($played.version -cne $FromVersion){throw 'Initial process is not the old compiled version'}
$signature=Join-Path $Root 'source/yoyovideo-update.windows-x64.json.sig'
$goodSignature=[IO.File]::ReadAllText($signature)
[IO.File]::WriteAllText($signature,'BAD TEST SIGNATURE')
$seq=Send-Qa $old.Id 'check'
$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 8} 'reject invalid signature' $old.Id
[IO.File]::WriteAllText($signature,$goodSignature)
$packageName="YoYoVideo-$ToVersion-stable-windows-x64-full.nupkg"
$sourcePackage=Join-Path $Root "source/$packageName"
$pristine=Join-Path $BuildRoot "$ToVersion/$packageName"
$stream=[IO.File]::OpenWrite($sourcePackage);try{$stream.WriteByte(0)}finally{$stream.Dispose()}
$seq=Send-Qa $old.Id 'check';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 4} 'valid manifest for corrupted package' $old.Id
$seq=Send-Qa $old.Id 'download';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 8} 'reject corrupted download' $old.Id
Copy-Item -LiteralPath $pristine -Destination $sourcePackage -Force
Ready-Qa $old
$seq=Send-Qa $old.Id 'later';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 6} 'postpone without installing' $old.Id
$null=Wait-Qa {param($s) $s.playback.position -ge 15} 'establish an unambiguous resume target' $old.Id
Close-Qa $old
$restarted=Start-Qa
$pending=Wait-Qa {param($s) $s.version -ceq $FromVersion -and $s.phase -eq 6} 'cached update after restart' $restarted.Id
Start-Sleep -Seconds 12
$pending=Read-QaState $restarted.Id
if($restarted.HasExited -or $pending.version -cne $FromVersion -or $pending.playback.media -or $pending.automatic_check){throw 'Restart applied an unconfirmed update or lost preferences'}
if(@($pending.playback.history).Count -ne 1 -or $pending.playback.history[0].last_position_seconds -lt 2){throw 'Old restart lost real playback history'}
$cachePackage=Join-Path $install "packages/$packageName"
if(-not (Test-Path -LiteralPath $cachePackage -PathType Leaf)){throw 'Native updater did not cache the expected package'}
$stream=[IO.File]::OpenWrite($cachePackage);try{$stream.WriteByte(0)}finally{$stream.Dispose()}
$seq=Send-Qa $restarted.Id 'install';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 8} 'reject cache tampering before install' $restarted.Id
Copy-Item -LiteralPath $pristine -Destination $cachePackage -Force
Ready-Qa $restarted
$other=Start-Qa
$seq=Send-Qa $restarted.Id 'install';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 8} 'refuse another instance in the same installation' $restarted.Id
if($other.HasExited){throw 'Other instance was killed instead of refusing installation'}
Close-Qa $other
Ready-Qa $restarted
$seq=Send-Qa $restarted.Id 'install'
$newState=Wait-Qa {param($s) $s.pid -ne $restarted.Id -and $s.version -ceq $ToVersion -and $s.playback.ready} 'new compiled player after actual helper replacement/restart'
Wait-QaExit $restarted
if($newState.playback.media -or $newState.automatic_check){throw 'New version autoplayed or lost updater preferences'}
if(@($newState.playback.history).Count -ne 1 -or $newState.playback.history[0].last_position_seconds -lt 2){throw 'New version lost playback history'}
$new=Get-Process -Id $newState.pid
if(-not $new.Path.StartsWith($install+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Unexpected restarted process path'}
$null=Get-VelopackBuildInfo -Executable $main -Version $ToVersion -QaFixture
$resumeTarget=[double]$newState.playback.history[0].last_position_seconds
if($resumeTarget -lt 15){throw 'Saved resume target is too short to distinguish seeking from fresh playback'}
$seq=Send-Qa $new.Id 'resume_history'
$resumed=Wait-Qa {param($s) $s.seq -eq $seq -and $s.playback.position -ge ($resumeTarget-1) -and (Normalize-QaPath $s.playback.media) -eq $media} 'restore history through real UI callback' $new.Id -Seconds 8
$seq=Send-Qa $new.Id 'check';$null=Wait-Qa {param($s) $s.seq -eq $seq -and $s.phase -eq 3} 'same-version check must be up-to-date' $new.Id
Close-Qa $new
@{from_pid=$old.Id;pending_restart_pid=$restarted.Id;to_pid=$newState.pid;from_version=$played.version;to_version=$newState.version;restored_position=$resumed.playback.position;layout='isolated Windows Velopack portable';registry_and_shortcuts_tested=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Root 'SUCCESS.json')
Write-Host "PASS actual player $FromVersion -> $ToVersion, real restart/history/preferences and negative security cases."
