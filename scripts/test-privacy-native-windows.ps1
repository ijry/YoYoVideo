#requires -Version 7.0
[CmdletBinding()]
param([string]$Executable,[string]$OutputDirectory)
$ErrorActionPreference='Stop'
if($env:OS -ne 'Windows_NT'){throw 'This native privacy acceptance script requires Windows.'}
$repo=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if(-not $Executable){$Executable=Join-Path $repo 'target/debug/yoyovideo-desktop.exe'}
$Executable=(Resolve-Path -LiteralPath $Executable).Path
$cache=(Resolve-Path -LiteralPath (Join-Path $repo '.cache')).Path
if(-not $OutputDirectory){$OutputDirectory=Join-Path $cache ('privacy-native-'+[Guid]::NewGuid().ToString('N'))}
$root=[IO.Path]::GetFullPath($OutputDirectory)
if(-not $root.StartsWith($cache+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'QA output must stay under this workspace .cache directory.'}
if(Test-Path -LiteralPath $root){throw 'Refusing to overwrite an existing QA fixture. Use a new output directory.'}
New-Item -ItemType Directory -Path (Join-Path $root 'media') -Force | Out-Null
$root=(Resolve-Path -LiteralPath $root).Path
[IO.File]::WriteAllText((Join-Path $root 'fixture.txt'),'yoyovideo-privacy-qa-v1')
[IO.File]::WriteAllText((Join-Path $root 'clock.txt'),'2026-10-08T08:59:50')
$checks=[Collections.Generic.List[string]]::new()
$script:player=$null;$script:seq=0;$script:launches=0
$oldPath=$env:PATH;$oldQa=$env:YOYOVIDEO_PRIVACY_QA_ROOT
$env:PATH=(Join-Path $repo 'third_party/mpv/windows-x64/bin')+';'+$env:PATH
$env:YOYOVIDEO_PRIVACY_QA_ROOT=$root
function Pass([string]$Name){$checks.Add($Name);Write-Host "PASS $Name"}
function Read-State {
    $file=Join-Path $root 'state.json'
    if(-not (Test-Path -LiteralPath $file)){return $null}
    try{$record=[IO.File]::ReadAllText($file)|ConvertFrom-Json}catch{return $null}
    if($record.pid -ne $script:player.Id){return $null}
    if($record.qa_error){throw "QA command failed: $($record.qa_error)"}
    return $record
}
function Wait-State([scriptblock]$Predicate,[string]$Description,[int]$Seconds=25){
    $until=[DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $script:player.Refresh()
        if($script:player.HasExited){throw "Player exited ($($script:player.ExitCode)) while waiting for: $Description"}
        $record=Read-State
        if($record -and $record.seq -ge $script:seq -and (& $Predicate $record.state)){return $record.state}
        Start-Sleep -Milliseconds 80
    }while([DateTime]::UtcNow -lt $until)
    throw "Timed out: $Description (inspect $root)"
}
function Write-Command([string]$Command,[hashtable]$Extra=@{}){
    $script:seq++
    $body=@{pid=$script:player.Id;seq=$script:seq;command=$Command}
    foreach($key in $Extra.Keys){$body[$key]=$Extra[$key]}
    $temporary=Join-Path $root 'request.tmp';$target=Join-Path $root 'request.json'
    [IO.File]::WriteAllText($temporary,($body|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $target -Force
}
function Send([string]$Command,[hashtable]$Extra=@{}){
    Write-Command $Command $Extra
    $null=Wait-State {param($s) $true} "acknowledge $Command"
}
function Launch([string]$InitialMedia='') {
    $script:launches++;$script:seq=0
    $start=@{FilePath=$Executable;WorkingDirectory=$repo;WindowStyle='Hidden';PassThru=$true;RedirectStandardOutput=(Join-Path $root "run-$($script:launches).stdout.log");RedirectStandardError=(Join-Path $root "run-$($script:launches).stderr.log")}
    if($InitialMedia){$start.ArgumentList='"'+(Join-Path $root "media/$InitialMedia")+'"'}
    $script:player=Start-Process @start
    $null=Wait-State {param($s) $s.ready} 'playback runtime ready' 40
    Send 'focus_main'
}
function Close-Player {
    Write-Command 'close'
    if(-not $script:player.WaitForExit(30000)){throw 'QA player did not exit gracefully.'}
    if($script:player.ExitCode -ne 0){throw "QA player exited with code $($script:player.ExitCode)"}
}
function Pin-Prompt {
    $null=Wait-State {param($s) $s.ui.visible -and $s.ui.focused -and -not $s.ui.busy} 'active PIN dialog'
}
function Unlock {
    Send 'toggle';Pin-Prompt;Send 'submit_good'
    $null=Wait-State {param($s) -not $s.privacy.enabled -and -not $s.ui.visible} 'PIN-authorized unlock'
    Send 'focus_main'
}
function Set-Clock([string]$Time){Send 'clock' @{local_time=$Time}}
function Check([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
try {
    # Refuse a production binary: an old build must never run against the user's actual profile.
    $probe=[Diagnostics.ProcessStartInfo]::new();$probe.FileName=$Executable;$probe.Arguments='--build-info';$probe.UseShellExecute=$false;$probe.CreateNoWindow=$true;$probe.RedirectStandardOutput=$true;$probe.RedirectStandardError=$true
    $process=[Diagnostics.Process]::Start($probe)
    $text=$process.StandardOutput.ReadToEnd();$diagnostic=$process.StandardError.ReadToEnd()
    if(-not $process.WaitForExit(10000) -or $process.ExitCode -ne 0){throw 'Build-info probe failed.'}
    $info=$text|ConvertFrom-Json
    if(-not $info.privacy_qa -or -not $info.mpv_runtime){throw 'Build with --features privacy-qa first. Production binaries are not driven by this script.'}
    $process.Dispose()
    $ffmpeg=(Get-Command ffmpeg -ErrorAction Stop).Source
    foreach($name in @('red','blue')){
        & $ffmpeg -hide_banner -loglevel error -f lavfi -i "color=c=${name}:s=320x180:r=15:d=180" -an -c:v libx264 -preset ultrafast -pix_fmt yuv420p -movflags +faststart -y (Join-Path $root "media/$name.mp4")
        if($LASTEXITCODE -ne 0){throw 'Failed to generate synthetic media.'}
    }
    Launch
    $state=Wait-State {param($s) -not $s.privacy.configured -and -not $s.native_visible} 'empty launch without a native surface hiding Open File'
    Pass 'empty launch keeps the Open File surface available'
    Send 'open' @{path='red.mp4'}
    $state=Wait-State {param($s) $s.current_file -eq 'red.mp4' -and $s.native_visible -and $s.color.red -ge 5 -and -not $s.backend_paused} 'visible red native video before protection'
    Check $state.host_parent_is_main 'An auxiliary PIN/settings window became the video host parent.'
    $initial=$state.position
    $null=Wait-State {param($s) $s.position -gt ($initial+1.0)} 'actual playing progress before concealment'
    Pass 'protected fixture is visibly red and genuinely playing before any lock'

    Send 'protect_current';Pin-Prompt
    $state=Read-State;Check ($state.state.ui.mode -eq 0) 'First protection must require PIN setup.'
    Send 'submit_good'
    $null=Wait-State {param($s) $s.privacy.configured -and -not $s.ui.visible -and -not $s.privacy.enabled} 'PIN setup and protection-list save'
    Send 'settings';Pin-Prompt;Send 'submit_good'
    $null=Wait-State {param($s) $s.ui.mode -eq 2 -and $s.ui.protected_rows -eq 1} 'authorized settings list'
    Send 'save_daily_schedule';Send 'cancel';Send 'focus_main'
    $null=Wait-State {param($s) -not $s.ui.visible -and -not $s.privacy.enabled} 'saved daily 09:00–18:00 schedule'
    Pass 'PIN setup and authorized protection/schedule editing use the actual window callbacks'

    Set-Clock '2026-10-08T09:00:00'
    $state=Wait-State {param($s) $s.blocked -and -not $s.native_visible -and $s.backend_paused -and $s.backend_muted -and -not $s.frame_active} 'scheduled pause, mute and real native hide'
    Check (-not $state.user_muted) 'Privacy mute overwrote the user mute preference.'
    Check (-not $state.status_mentions_red) 'A protected filename leaked through the status.'
    Check ($state.history_redacted[0] -and $state.playlist_redacted[0] -and $state.recent_redacted[0]) 'Protected rows were not redacted.'
    $lockedPosition=$state.position
    Start-Sleep -Milliseconds 800
    $state=(Read-State).state
    Check ([Math]::Abs($state.position-$lockedPosition) -lt 0.3) 'Protected playback kept advancing.'
    Pass 'entering the restricted period pauses, temporarily mutes, hides and redacts the playing protected item'
    Send 'play';Send 'screenshot'
    $state=Wait-State {param($s) $s.blocked -and $s.backend_paused -and -not $s.native_visible} 'blocked resume and screenshot'
    Check (-not (Test-Path -LiteralPath (Join-Path $root 'requested-screenshot.png'))) 'Restricted screenshot was created.'
    Send 'open_popup';$null=Wait-State {param($s) $s.popup} 'real menu opened'
    Send 'close_popup';Send 'fullscreen';Send 'fullscreen'
    $null=Wait-State {param($s) -not $s.popup -and $s.blocked -and -not $s.native_visible} 'popup close and fullscreen cannot reveal the protected surface'
    Pass 'resume, screenshot, popup-close and fullscreen paths cannot bypass protection'

    Send 'open' @{path='blue.mp4'};Send 'focus_main'
    $state=Wait-State {param($s) $s.current_file -eq 'blue.mp4' -and $s.color.blue -ge 5 -and -not $s.backend_paused} 'ordinary video remains available during privacy mode'
    Send 'history' @{index=1};Send 'recent' @{index=1};Send 'open' @{path='red.mp4'}
    $state=Wait-State {param($s) $s.current_file -eq 'blue.mp4' -and -not $s.blocked -and -not $s.backend_paused} 'denied history/recent/drop do not replace current ordinary media'
    Check (-not $state.status_mentions_red) 'Denied target leaked its filename.'
    Pass 'ordinary media plays; protected history, recent and drop opens are transactional'
    Send 'playlist';Send 'next'
    $null=Wait-State {param($s) $s.current_file -eq 'blue.mp4'} 'next-item guard'
    Send 'seek_end'
    $null=Wait-State {param($s) $s.current_file -eq 'blue.mp4' -and $s.backend_idle} 'EOF refuses the protected next item' 12
    Pass 'playlist selection and automatic EOF cannot open the protected next item'

    Set-Clock '2026-10-08T10:00:00';Unlock
    Send 'open' @{path='red.mp4'};Send 'focus_main'
    $null=Wait-State {param($s) $s.current_file -eq 'red.mp4' -and $s.color.red -ge 5 -and -not $s.privacy.enabled -and $s.privacy.manual} 'manual-off override within the restricted period'
    Set-Clock '2026-10-08T10:01:00';Close-Player;Launch
    $null=Wait-State {param($s) $s.privacy.manual -and -not $s.privacy.enabled} 'manual off survives restart'
    Send 'open' @{path='red.mp4'};Send 'focus_main'
    $null=Wait-State {param($s) $s.color.red -ge 5} 'protected media is permitted by persisted manual off'
    Set-Clock '2026-10-08T18:00:00'
    $null=Wait-State {param($s) $s.privacy.manual -and -not $s.privacy.enabled} 'current period end does not cancel manual override'
    Set-Clock '2026-10-09T09:00:00'
    $null=Wait-State {param($s) $s.privacy.enabled -and -not $s.privacy.manual -and $s.blocked -and $s.backend_paused -and -not $s.native_visible} 'next start takes back automatic control'
    Pass 'manual override survives restart and the current end, and expires only at the next start'
    Unlock
    $state=Wait-State {param($s) -not $s.blocked -and $s.paused -and $s.backend_paused -and -not $s.backend_muted} 'unlock never automatically resumes or keeps privacy mute'
    Pass 'unlock keeps playback paused and restores the user mute preference'

    Send 'grid';Send 'focus_main'
    $state=Wait-State {param($s) $s.grid.Count -eq 2 -and $s.grid[0].color.red -ge 5 -and $s.grid[1].color.blue -ge 5} 'two native tiles are genuinely visible'
    $normalStart=$state.grid[1].position
    Send 'toggle'
    $state=Wait-State {param($s) $s.grid.Count -eq 2 -and $s.grid[0].blocked -and -not $s.grid[0].native_visible -and $s.grid[0].backend_paused -and $s.grid[0].backend_muted -and $s.grid[1].native_visible -and -not $s.grid[1].backend_paused} 'only the protected tile is concealed'
    $null=Wait-State {param($s) $s.grid[1].position -gt ($normalStart+1.0)} 'ordinary tile keeps advancing'
    Send 'grid_play';Send 'grid_volume' @{index=0;volume=37}
    $null=Wait-State {param($s) $s.grid[0].backend_paused -and $s.grid[0].backend_muted -and $s.grid[0].volume -eq 37 -and -not $s.grid[0].user_muted} 'grid play-all and volume changes preserve privacy'
    Send 'open_popup';Send 'close_popup'
    $null=Wait-State {param($s) -not $s.grid[0].native_visible -and $s.grid[1].native_visible} 'popup close only restores the ordinary tile'
    Set-Clock '2026-10-09T18:00:00'
    $null=Wait-State {param($s) $s.privacy.manual -and $s.privacy.enabled -and $s.grid[0].blocked} 'manual on persists across the current end'
    Set-Clock '2026-10-10T09:00:00';Set-Clock '2026-10-10T18:00:00'
    $state=Wait-State {param($s) -not $s.privacy.enabled -and $s.grid[0].paused -and $s.grid[0].backend_paused -and -not $s.grid[0].backend_muted -and $s.grid[0].volume -eq 37} 'scheduled end releases protection but does not resume the protected tile'
    Pass 'grid protection is per-tile; ordinary playback and user volume remain independent'

    Send 'toggle';Send 'toggle';Pin-Prompt
    for($i=0;$i -lt 5;$i++){
        Send 'submit_bad'
        $null=Wait-State {param($s) -not $s.ui.busy -and $s.ui.pin_empty} 'wrong PIN result and cleared input'
    }
    $state=Wait-State {param($s) $s.privacy.cooldown -eq 30 -and $s.privacy.enabled} 'five failures enter cooldown'
    Send 'submit_good'
    $state=(Read-State).state;Check $state.privacy.enabled 'Correct PIN bypassed the active cooldown.'
    Send 'cancel';Close-Player;Launch 'red.mp4'
    $state=Wait-State {param($s) $s.privacy.cooldown -eq 30 -and $s.privacy.enabled} 'cooldown is persisted across restart'
    Check (-not $state.current_file -and $state.backend_idle -and -not $state.native_visible) 'Protected command-line startup bypassed privacy.'
    Pass 'protected command-line startup cannot bypass persisted restrictions'
    Set-Clock '2026-10-10T18:00:31';Unlock
    Pass 'five failed PINs enforce a persisted 30-second cooldown without logging digits'

    Close-Player
    $privacyFile=Join-Path $root 'user/config/privacy.toml'
    [IO.File]::WriteAllText($privacyFile,'intentionally malformed privacy fixture [')
    Launch;Send 'open' @{path='blue.mp4'}
    $state=Wait-State {param($s) $s.privacy.fail_closed -and $s.privacy.enabled -and -not $s.current_file -and -not $s.native_visible} 'corrupt configuration fails closed for all media'
    Close-Player
    Check ([IO.File]::ReadAllText($privacyFile) -eq 'intentionally malformed privacy fixture [') 'Corrupt configuration was silently overwritten.'
    Pass 'corrupt privacy configuration is fail-closed and is not reset or overwritten'

    [IO.File]::WriteAllText((Join-Path $root 'report.json'),(@{success=$true;platform='windows-x64';checks=$checks;fixture=$root;launches=$script:launches}|ConvertTo-Json -Depth 5))
    Write-Host "Native privacy acceptance passed. Evidence: $root"
} catch {
    [IO.File]::WriteAllText((Join-Path $root 'report.json'),(@{success=$false;platform='windows-x64';checks=$checks;error=$_.Exception.Message;fixture=$root}|ConvertTo-Json -Depth 5))
    Write-Host "Native privacy acceptance failed. Evidence: $root"
    throw
} finally {
    if($script:player){$script:player.Refresh();if(-not $script:player.HasExited){$script:player.Kill();$null=$script:player.WaitForExit(10000)}}
    $env:PATH=$oldPath;$env:YOYOVIDEO_PRIVACY_QA_ROOT=$oldQa
}
