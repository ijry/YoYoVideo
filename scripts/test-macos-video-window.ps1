#requires -Version 7.0
[CmdletBinding()]
param([string]$Executable, [string]$OutputDirectory, [string]$Platform)
$ErrorActionPreference = 'Stop'
if (-not $IsMacOS) { throw 'This acceptance test requires a real macOS WindowServer.' }
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $Executable) { $Executable = Join-Path $repo 'target/debug/yoyovideo-desktop' }
$Executable = (Resolve-Path -LiteralPath $Executable).Path
if (-not $Platform) { $Platform = if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'macos-aarch64' } else { 'macos-x86_64' } }
if ($Platform -notin @('macos-aarch64', 'macos-x86_64')) { throw 'Invalid macOS runtime platform' }
$cache = Join-Path $repo '.cache'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$cache = (Resolve-Path -LiteralPath $cache).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $cache ('macos-window-' + [guid]::NewGuid().ToString('N')) }
$root = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $root.StartsWith($cache + '/', [StringComparison]::Ordinal)) { throw 'Native QA output must stay inside this checkout .cache.' }
if (Test-Path -LiteralPath $root) { throw 'Refusing to overwrite an existing native QA fixture.' }
New-Item -ItemType Directory -Path (Join-Path $root 'media') -Force | Out-Null
$root = (Resolve-Path -LiteralPath $root).Path
[IO.File]::WriteAllText((Join-Path $root 'fixture.txt'), 'yoyovideo-privacy-qa-v1')
[IO.File]::WriteAllText((Join-Path $root 'clock.txt'), '2026-10-09T08:00:00')
$oldQa = $env:YOYOVIDEO_PRIVACY_QA_ROOT
$oldDylib = $env:DYLD_LIBRARY_PATH
$env:YOYOVIDEO_PRIVACY_QA_ROOT = $root
$env:DYLD_LIBRARY_PATH = (Join-Path $repo "third_party/mpv/$Platform/lib") + ':' + $oldDylib
$checks = [Collections.Generic.List[string]]::new()
$script:player = $null
$script:seq = 0
$script:lastRecord = $null
$failure = $null
function Pass([string]$Name) { $checks.Add($Name); Write-Host "PASS $Name"; Write-Host ($script:lastRecord.state | ConvertTo-Json -Depth 10 -Compress) }
function Read-State {
    $path = Join-Path $root 'state.json'
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    try { $record = [IO.File]::ReadAllText($path) | ConvertFrom-Json } catch { return $null }
    if ($record.pid -ne $script:player.Id) { return $null }
    $script:lastRecord = $record
    if ($record.qa_error) { throw "Native QA command failed: $($record.qa_error)" }
    if ($record.state.macos.error) { throw "Native window observation failed: $($record.state.macos.error)" }
    return $record
}
function Wait-State([scriptblock]$Predicate, [string]$Description, [int]$Seconds = 20) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    $stable = 0
    do {
        $script:player.Refresh()
        if ($script:player.HasExited) { throw "Player exited ($($script:player.ExitCode)) while waiting for $Description" }
        $record = Read-State
        if ($record -and $record.seq -ge $script:seq -and (& $Predicate $record.state)) {
            $stable++
            if ($stable -ge 3) { return $record.state }
        } else { $stable = 0 }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out: $Description. Last state: $($script:lastRecord.state | ConvertTo-Json -Depth 10 -Compress)"
}
function Write-Command([string]$Command, [hashtable]$Extra = @{}) {
    $script:seq++
    $body = @{ pid = $script:player.Id; seq = $script:seq; command = $Command }
    foreach ($key in $Extra.Keys) { $body[$key] = $Extra[$key] }
    $temporary = Join-Path $root 'request.tmp'
    $destination = Join-Path $root 'request.json'
    [IO.File]::WriteAllText($temporary, ($body | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $destination -Force
}
function Send([string]$Command, [hashtable]$Extra = @{}) {
    Write-Command $Command $Extra
    $null = Wait-State { param($state) $true } "acknowledge $Command"
}
function Aligned($State) {
    $visible = @($State.macos.hosts | Where-Object visible)
    if ($visible.Count -ne 1 -or -not $visible[0].parent_is_main) { return $false }
    $actual = $visible[0].frame
    $expected = $State.macos.expected_host
    foreach ($key in @('x', 'y', 'width', 'height')) {
        if ([math]::Abs($actual.$key - $expected.$key) -gt 1.0) { return $false }
    }
    return $actual.width -gt 0 -and $actual.height -gt 0
}
try {
    # An ordinary release must never be driven against a real user's preferences.
    $text = & $Executable --build-info
    if ($LASTEXITCODE -ne 0) { throw 'Native build-info probe failed' }
    $info = $text | ConvertFrom-Json
    if (-not $info.privacy_qa -or -not $info.mpv_runtime) { throw 'Only an isolated --features privacy-qa build may be tested.' }
    foreach ($color in @('red', 'blue')) {
        & ffmpeg -hide_banner -loglevel error -f lavfi -i "color=c=${color}:s=320x180:r=15:d=180" -an -c:v libx264 -preset ultrafast -pix_fmt yuv420p -y (Join-Path $root "media/$color.mp4")
        if ($LASTEXITCODE -ne 0) { throw 'Synthetic media generation failed' }
    }
    # This script refuses Windows above: Start-Process's WindowStyle is not supported on macOS.
    $script:player = Start-Process -FilePath $Executable -WorkingDirectory $repo -PassThru -RedirectStandardOutput (Join-Path $root 'player.stdout.log') -RedirectStandardError (Join-Path $root 'player.stderr.log')
    $null = Wait-State { param($s) $s.ready -and $s.macos.main.visible } 'real main window and playback runtime' 45
    Send 'focus_main'
    $null = Wait-State { param($s) $s.macos.visible_windows -eq 1 -and -not $s.native_visible } 'one visible window on an empty launch'
    Pass 'empty startup shows only the player, not a detached black video window'

    Send 'move_main' @{ window_position = @(110, 85) }
    Send 'resize_main' @{ window_size = @(860, 620) }
    Send 'minimize_main'
    $null = Wait-State { param($s) $s.macos.main.minimized -and -not $s.native_visible } 'empty player minimized without revealing its host'
    Send 'restore_main'
    $null = Wait-State { param($s) -not $s.macos.main.minimized -and $s.macos.visible_windows -eq 1 -and -not $s.native_visible } 'empty restore still has only one visible window'
    Pass 'moving, resizing, minimizing and restoring an empty player never reveals the host'

    Send 'open' @{ path = 'red.mp4' }
    $state = Wait-State { param($s) $s.current_file -eq 'red.mp4' -and $s.native_visible -and -not $s.backend_paused -and $s.position -gt 0.4 } 'real media playback in the native host'
    $initial = $state.position
    $null = Wait-State { param($s) $s.position -gt ($initial + 0.5) } 'actual playback progress'
    $null = Wait-State { param($s) Aligned $s } 'host anchored to the main content viewport, not desktop coordinates' 8
    Pass 'playing video is anchored to the main viewport in AppKit screen coordinates'

    Send 'move_main' @{ window_position = @(175, 115) }
    $null = Wait-State { param($s) Aligned $s } 'host follows a moved player'
    Send 'resize_main' @{ window_size = @(980, 680) }
    $null = Wait-State { param($s) Aligned $s } 'host follows resized native content'
    Pass 'native video tracks main-window movement and resizing'

    Send 'focus_main'
    $null = Wait-State { param($s) $s.macos.main.key -and -not @($s.macos.hosts | Where-Object key).Count } 'repeated video synchronization does not steal main-window focus'
    Pass 'video visibility synchronization does not steal keyboard focus'

    Send 'open_popup'
    $null = Wait-State { param($s) $s.popup -and -not $s.native_visible } 'native video hidden behind a menu'
    Send 'close_popup'
    $null = Wait-State { param($s) -not $s.popup -and (Aligned $s) } 'menu dismissal restores an aligned host'
    Pass 'menu suppression hides and restores the native host without detaching it'

    Send 'minimize_main'
    $null = Wait-State { param($s) $s.macos.main.minimized -and -not $s.native_visible } 'playing host stays hidden while main is minimized'
    Send 'restore_main'
    $null = Wait-State { param($s) -not $s.macos.main.minimized -and (Aligned $s) } 'restored playback remains attached to the main viewport'
    Pass 'minimize/restore cannot leave a floating playback rectangle'

    Write-Command 'close'
    if (-not $script:player.WaitForExit(30000) -or $script:player.ExitCode -ne 0) { throw 'Native player did not shut down cleanly.' }
    Pass 'all native hosts and render contexts shut down without a crash'
} catch {
    $failure = $_.Exception.Message
    if ($null -ne $script:player -and -not $script:player.HasExited -and (Test-Path -LiteralPath '/usr/bin/sample')) {
        & /usr/bin/sample $script:player.Id 1 -file (Join-Path $root 'player.sample.log') | Out-Host
    }
    if (Test-Path -LiteralPath (Join-Path $root 'player.stderr.log')) { Get-Content -LiteralPath (Join-Path $root 'player.stderr.log') -Tail 120 | Out-Host }
    throw
} finally {
    if ($null -ne $script:player) {
        $script:player.Refresh()
        if (-not $script:player.HasExited) {
            try { Write-Command 'close'; $null = $script:player.WaitForExit(10000) } catch {}
            if (-not $script:player.HasExited) { $script:player.Kill(); $script:player.WaitForExit() }
        }
    }
    [ordered]@{ platform = $Platform; passed = ($null -eq $failure); checks = @($checks); failure = $failure; last_state = $script:lastRecord; fixture = $root } |
        ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $root 'report.json')
    $env:YOYOVIDEO_PRIVACY_QA_ROOT = $oldQa
    $env:DYLD_LIBRARY_PATH = $oldDylib
    Write-Host "Native macOS evidence: $root"
}
