#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('macos-aarch64','macos-x86_64','linux-x64')]
    [string]$Platform,
    [string]$Executable,
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
if (-not $IsMacOS -and -not $IsLinux) { throw 'This native privacy acceptance script requires macOS or Linux.' }
if (-not $Platform) {
    if ($IsMacOS) {
        $Platform = if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'macos-aarch64' } else { 'macos-x86_64' }
    } else { $Platform = 'linux-x64' }
}
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $Executable) { $Executable = Join-Path $repo 'target/debug/yoyovideo-desktop' }
$Executable = (Resolve-Path -LiteralPath $Executable).Path
New-Item -ItemType Directory -Path (Join-Path $repo '.cache') -Force | Out-Null
$cache = (Resolve-Path -LiteralPath (Join-Path $repo '.cache')).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $cache ('privacy-native-unix-' + [guid]::NewGuid().ToString('N')) }
$root = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $root.StartsWith($cache + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'QA output must stay under this workspace .cache directory.' }
if (Test-Path -LiteralPath $root) { throw 'Refusing to overwrite an existing QA fixture.' }
New-Item -ItemType Directory -Path (Join-Path $root 'media') -Force | Out-Null
$root = (Resolve-Path -LiteralPath $root).Path
[IO.File]::WriteAllText((Join-Path $root 'fixture.txt'), 'yoyovideo-privacy-qa-v1')
[IO.File]::WriteAllText((Join-Path $root 'clock.txt'), '2026-10-08T08:59:50')
$checks = [Collections.Generic.List[string]]::new()
$script:player = $null
$script:seq = 0
$script:launches = 0
$oldPath = $env:PATH
$oldQa = $env:YOYOVIDEO_PRIVACY_QA_ROOT
$oldQaFocus = $env:YOYOVIDEO_PRIVACY_QA_FOCUS
$oldLoader = if ($IsMacOS) { $env:DYLD_LIBRARY_PATH } else { $env:LD_LIBRARY_PATH }
$runtimeRoot = Join-Path $repo ("third_party/mpv/$Platform")
$runtimeBin = Join-Path $runtimeRoot 'bin'
$runtimeLib = Join-Path $runtimeRoot 'lib'
if (Test-Path -LiteralPath $runtimeBin) { $env:PATH = $runtimeBin + [IO.Path]::PathSeparator + $env:PATH }
$loaderName = if ($IsMacOS) { 'DYLD_LIBRARY_PATH' } else { 'LD_LIBRARY_PATH' }
[Environment]::SetEnvironmentVariable($loaderName, $runtimeLib + [IO.Path]::PathSeparator + $oldLoader)
$env:YOYOVIDEO_PRIVACY_QA_ROOT = $root
$env:YOYOVIDEO_PRIVACY_QA_FOCUS = '1'

function Pass([string]$Name) { $checks.Add($Name); Write-Host "PASS $Name" }
function Check([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function SurfaceVisible($State) { return ($State.native_visible -or $State.frame_active) }
function SurfaceHidden($State) { return (-not $State.native_visible -and -not $State.frame_active) }
function ParentAttached($State) {
    if ($null -eq $State.macos) { return $true }
    $visible = @($State.macos.hosts | Where-Object visible)
    return $visible.Count -eq 1 -and $visible[0].parent_is_main
}
function Read-State {
    $file = Join-Path $root 'state.json'
    if (-not (Test-Path -LiteralPath $file)) { return $null }
    try { $record = [IO.File]::ReadAllText($file) | ConvertFrom-Json } catch { return $null }
    if ($record.pid -ne $script:player.Id) { return $null }
    if ($record.qa_error) { throw "QA command failed: $($record.qa_error)" }
    return $record
}
function Wait-State([scriptblock]$Predicate, [string]$Description, [int]$Seconds = 30) {
    $until = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $script:player.Refresh()
        if ($script:player.HasExited) { throw "Player exited ($($script:player.ExitCode)) while waiting for: $Description" }
        $record = Read-State
        if ($record -and $record.seq -ge $script:seq -and (& $Predicate $record.state)) { return $record.state }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $until)
    throw "Timed out: $Description (inspect $root)"
}
function Write-Command([string]$Command, [hashtable]$Extra = @{}) {
    $script:seq++
    $body = @{ pid = $script:player.Id; seq = $script:seq; command = $Command }
    foreach ($key in $Extra.Keys) { $body[$key] = $Extra[$key] }
    $temporary = Join-Path $root 'request.tmp'
    $target = Join-Path $root 'request.json'
    [IO.File]::WriteAllText($temporary, ($body | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $target -Force
}
function Send([string]$Command, [hashtable]$Extra = @{}) {
    Write-Command $Command $Extra
    $null = Wait-State { param($s) $true } "acknowledge $Command"
}
function Launch([string]$InitialMedia = '') {
    $script:launches++
    $script:seq = 0
    $arguments = @()
    if ($InitialMedia) { $arguments += (Join-Path $root "media/$InitialMedia") }
    $start = @{
        FilePath = $Executable
        WorkingDirectory = $repo
        PassThru = $true
        RedirectStandardOutput = (Join-Path $root "run-$($script:launches).stdout.log")
        RedirectStandardError = (Join-Path $root "run-$($script:launches).stderr.log")
    }
    if ($arguments.Count) { $start.ArgumentList = $arguments }
    $script:player = Start-Process @start
    $null = Wait-State { param($s) $s.ready -and $s.main_visible } 'playback runtime ready' 45
    Send 'focus_main'
}
function Close-Player {
    Write-Command 'close'
    if (-not $script:player.WaitForExit(30000)) { throw 'QA player did not exit gracefully.' }
    if ($script:player.ExitCode -ne 0) { throw "QA player exited with code $($script:player.ExitCode)" }
}
function Pin-Prompt { $null = Wait-State { param($s) $s.ui.visible -and -not $s.ui.busy } 'active PIN dialog' }
function Unlock {
    Send 'toggle'; Pin-Prompt; Send 'submit_good'
    $null = Wait-State { param($s) -not $s.privacy.enabled -and -not $s.ui.visible } 'PIN-authorized unlock'
    Send 'focus_main'
}
function Set-Clock([string]$Time) { Send 'clock' @{ local_time = $Time } }

try {
    $text = & $Executable --build-info
    if ($LASTEXITCODE -ne 0) { throw 'Build-info probe failed.' }
    $info = $text | ConvertFrom-Json
    if (-not $info.privacy_qa -or -not $info.mpv_runtime) { throw 'Build with --features privacy-qa first.' }
    $ffmpeg = (Get-Command ffmpeg -ErrorAction Stop).Source
    foreach ($name in @('red', 'blue')) {
        & $ffmpeg -hide_banner -loglevel error -f lavfi -i "color=c=${name}:s=320x180:r=15:d=180" -an -c:v libx264 -preset ultrafast -pix_fmt yuv420p -movflags +faststart -y (Join-Path $root "media/$name.mp4")
        if ($LASTEXITCODE -ne 0) { throw 'Failed to generate synthetic media.' }
    }

    Launch
    $state = Wait-State { param($s) -not $s.privacy.configured -and (SurfaceHidden $s) } 'empty launch without a visible video surface'
    Pass 'empty launch keeps the Open File surface available'
    Send 'open' @{ path = 'red.mp4' }
    $state = Wait-State { param($s) $s.current_file -eq 'red.mp4' -and (SurfaceVisible $s) -and -not $s.backend_paused } 'visible red media before protection'
    Check (ParentAttached $state) 'Video host is not attached to the main window.'
    $initial = $state.position
    $null = Wait-State { param($s) $s.position -gt ($initial + 1.0) } 'actual playing progress before concealment'
    Pass 'protected fixture is genuinely playing before any lock'

    Send 'protect_current'; Pin-Prompt
    $state = Read-State; Check ($state.state.ui.mode -eq 0) 'First protection must require PIN setup.'
    Send 'submit_good'
    $null = Wait-State { param($s) $s.privacy.configured -and -not $s.ui.visible -and -not $s.privacy.enabled } 'PIN setup and protection-list save'
    Send 'settings'; Pin-Prompt; Send 'submit_good'
    $null = Wait-State { param($s) $s.ui.mode -eq 2 -and $s.ui.protected_rows -eq 1 } 'authorized settings list'
    Send 'save_daily_schedule'; Send 'cancel'; Send 'focus_main'
    $null = Wait-State { param($s) -not $s.ui.visible -and -not $s.privacy.enabled } 'saved daily 09:00-18:00 schedule'
    Pass 'PIN setup and authorized protection/schedule editing use the actual window callbacks'

    Set-Clock '2026-10-08T09:00:00'
    $state = Wait-State { param($s) $s.blocked -and (SurfaceHidden $s) -and $s.backend_paused -and $s.backend_muted -and -not $s.frame_active } 'scheduled pause, mute and real native hide'
    Check (-not $state.user_muted) 'Privacy mute overwrote the user mute preference.'
    Check (-not $state.status_mentions_red) 'A protected filename leaked through the status.'
    Check ($state.history_redacted[0] -and $state.playlist_redacted[0] -and $state.recent_redacted[0]) 'Protected rows were not redacted.'
    $lockedPosition = $state.position
    Start-Sleep -Milliseconds 900
    $state = (Read-State).state
    Check ([Math]::Abs($state.position - $lockedPosition) -lt 0.3) 'Protected playback kept advancing.'
    Pass 'entering the restricted period pauses, temporarily mutes and hides the playing protected item'

    Send 'play'; Send 'screenshot'
    $state = Wait-State { param($s) $s.blocked -and $s.backend_paused -and (SurfaceHidden $s) } 'blocked resume and screenshot'
    Check (-not (Test-Path -LiteralPath (Join-Path $root 'requested-screenshot.png'))) 'Restricted screenshot was created.'
    Send 'open_popup'; $null = Wait-State { param($s) $s.popup } 'real menu opened'
    Send 'close_popup'; $null = Wait-State { param($s) -not $s.popup -and $s.blocked -and (SurfaceHidden $s) } 'popup close cannot reveal the protected surface'
    Pass 'resume, screenshot and popup close cannot bypass protection'

    Send 'open' @{ path = 'blue.mp4' }; Send 'focus_main'
    $state = Wait-State { param($s) $s.current_file -eq 'blue.mp4' -and (SurfaceVisible $s) -and -not $s.backend_paused } 'ordinary video remains available during privacy mode'
    Send 'history' @{ index = 1 }; Send 'recent' @{ index = 1 }; Send 'open' @{ path = 'red.mp4' }
    $state = Wait-State { param($s) $s.current_file -eq 'blue.mp4' -and -not $s.blocked -and -not $s.backend_paused } 'denied history/recent/drop do not replace current ordinary media'
    Check (-not $state.status_mentions_red) 'Denied target leaked its filename.'
    Pass 'ordinary media plays; protected history, recent and drop opens are transactional'

    Set-Clock '2026-10-08T10:00:00'; Unlock
    Send 'open' @{ path = 'red.mp4' }; Send 'focus_main'
    $null = Wait-State { param($s) $s.current_file -eq 'red.mp4' -and (SurfaceVisible $s) -and -not $s.privacy.enabled -and $s.privacy.manual } 'manual-off override within the restricted period'
    Set-Clock '2026-10-08T10:01:00'; Close-Player; Launch
    $null = Wait-State { param($s) $s.privacy.manual -and -not $s.privacy.enabled } 'manual off survives restart'
    Send 'open' @{ path = 'red.mp4' }; Send 'focus_main'
    $null = Wait-State { param($s) $s.current_file -eq 'red.mp4' -and (SurfaceVisible $s) } 'protected media is permitted by persisted manual off'
    Set-Clock '2026-10-08T18:00:00'
    $null = Wait-State { param($s) $s.privacy.manual -and -not $s.privacy.enabled } 'current period end does not cancel manual override'
    Set-Clock '2026-10-09T09:00:00'
    $null = Wait-State { param($s) $s.privacy.enabled -and -not $s.privacy.manual -and $s.blocked -and $s.backend_paused -and (SurfaceHidden $s) } 'next start takes back automatic control'
    Pass 'manual override survives restart and expires only at the next start'
    Unlock
    $null = Wait-State { param($s) -not $s.blocked -and $s.paused -and $s.backend_paused -and -not $s.backend_muted } 'unlock never automatically resumes or keeps privacy mute'
    Pass 'unlock keeps playback paused and restores the user mute preference'

    Send 'toggle'
    $null = Wait-State { param($s) $s.privacy.enabled -and $s.privacy.manual } 'manual privacy enabled before requesting unlock'
    Start-Sleep -Milliseconds 500
    Send 'settings'; Pin-Prompt
    for ($i = 0; $i -lt 5; $i++) { Send 'submit_bad'; $null = Wait-State { param($s) -not $s.ui.busy -and $s.ui.pin_empty } 'wrong PIN result and cleared input' }
    $state = Wait-State { param($s) $s.privacy.cooldown -eq 30 -and $s.privacy.enabled } 'five failures enter cooldown'
    Send 'submit_good'
    $state = (Read-State).state
    Check $state.privacy.enabled 'Correct PIN bypassed the active cooldown.'
    Send 'cancel'; Close-Player; Launch 'red.mp4'
    $state = Wait-State { param($s) $s.privacy.cooldown -eq 30 -and $s.privacy.enabled } 'cooldown persists across restart'
    Check (-not $state.current_file -and $state.backend_idle -and (SurfaceHidden $state)) 'Protected command-line startup bypassed privacy.'
    # Advance past the persisted cooldown while staying inside the restricted window: an
    # out-of-window time would legitimately drop privacy and turn the toggle into a re-arm.
    Set-Clock '2026-10-10T09:00:31'; Unlock
    Pass 'five failed PINs enforce a persisted 30-second cooldown without logging digits'

    Close-Player
    $privacyFile = Join-Path $root 'user/config/privacy.toml'
    [IO.File]::WriteAllText($privacyFile, 'intentionally malformed privacy fixture [')
    Launch; Send 'open' @{ path = 'blue.mp4' }
    $state = Wait-State { param($s) $s.privacy.fail_closed -and $s.privacy.enabled -and -not $s.current_file -and (SurfaceHidden $s) } 'corrupt configuration fails closed for all media'
    Close-Player
    Check ([IO.File]::ReadAllText($privacyFile) -eq 'intentionally malformed privacy fixture [') 'Corrupt configuration was silently overwritten.'
    Pass 'corrupt privacy configuration is fail-closed and is not reset or overwritten'

    [IO.File]::WriteAllText((Join-Path $root 'report.json'), (@{ success = $true; platform = $Platform; checks = $checks; fixture = $root; launches = $script:launches } | ConvertTo-Json -Depth 5))
    Write-Host "Native privacy acceptance passed. Evidence: $root"
} catch {
    [IO.File]::WriteAllText((Join-Path $root 'report.json'), (@{ success = $false; platform = $Platform; checks = $checks; error = $_.Exception.Message; fixture = $root } | ConvertTo-Json -Depth 5))
    Write-Host "Native privacy acceptance failed. Evidence: $root"
    throw
} finally {
    if ($script:player) { $script:player.Refresh(); if (-not $script:player.HasExited) { $script:player.Kill(); $null = $script:player.WaitForExit(10000) } }
    $env:PATH = $oldPath
    $env:YOYOVIDEO_PRIVACY_QA_ROOT = $oldQa
    $env:YOYOVIDEO_PRIVACY_QA_FOCUS = $oldQaFocus
    [Environment]::SetEnvironmentVariable($loaderName, $oldLoader)
}
