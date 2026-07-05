[CmdletBinding()]
param(
    [ValidateSet("windows-x64", "macos-universal", "linux-x64")]
    [string]$Platform = "windows-x64",

    [int]$TimeoutSeconds = 5,

    [string]$RuntimeBin,

    [string]$RuntimeLib
)

$ErrorActionPreference = "Stop"

function Fail([string]$Message) {
    Write-Error $Message
    exit 1
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($RuntimeBin)) {
    $runtimeBin = Join-Path $repoRoot "third_party/mpv/$Platform/bin"
} else {
    $runtimeBin = [System.IO.Path]::GetFullPath($RuntimeBin)
}
if ([string]::IsNullOrWhiteSpace($RuntimeLib)) {
    $runtimeLib = Join-Path $repoRoot "third_party/mpv/$Platform/lib"
} else {
    $runtimeLib = [System.IO.Path]::GetFullPath($RuntimeLib)
}

if ($Platform -eq "windows-x64" -and -not (Test-Path -LiteralPath (Join-Path $runtimeBin "mpv-2.dll") -PathType Leaf)) {
    Fail "Missing Windows runtime DLL. Run: pwsh -NoProfile -File scripts/bootstrap-runtime.ps1 -Platform windows-x64"
}

$probeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yoyovideo-runtime-smoke-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Force (Join-Path $probeRoot "src") | Out-Null

Set-Content -LiteralPath (Join-Path $probeRoot "Cargo.toml") -Value @"
[package]
name = "yoyovideo-runtime-smoke"
version = "0.1.0"
edition = "2024"

[dependencies]
yoyovideo_desktop = { package = "yoyovideo-desktop", path = "$(($repoRoot.Path -replace '\\', '/') + '/apps/yoyovideo-desktop')", features = ["mpv-runtime"] }
yoyo_core = { package = "yoyo-core", path = "$(($repoRoot.Path -replace '\\', '/') + '/crates/yoyo-core')" }
"@

Set-Content -LiteralPath (Join-Path $probeRoot "src/main.rs") -Value @"
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use yoyo_core::{BackendEvent, MediaLocator, PlayerBackend};

fn main() {
    let media = std::env::temp_dir().join("yoyovideo-smoke.wav");
    write_wav(&media);
    let mut backend = yoyovideo_desktop::build_desktop_backend().expect("backend init");
    backend.open(&MediaLocator::File(media)).expect("open media");
    let start = Instant::now();
    let mut duration = false;
    let mut position = false;
    let mut tracks = false;
    let mut errors = Vec::new();
    while start.elapsed() < Duration::from_secs($TimeoutSeconds) {
        for event in backend.drain_events() {
            println!("event={event:?}");
            match event {
                BackendEvent::DurationChanged(Some(value)) if value > 0.0 => duration = true,
                BackendEvent::PositionChanged(value) if value >= 0.0 => position = true,
                BackendEvent::TracksChanged { audio, subtitles: _, video: _ } if !audio.is_empty() => tracks = true,
                BackendEvent::Error(message) => errors.push(message),
                _ => {}
            }
        }
        if duration && position && tracks {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    if !errors.is_empty() {
        panic!("backend errors: {errors:?}");
    }
    if !(duration && position && tracks) {
        panic!("missing expected events: duration={duration} position={position} tracks={tracks}");
    }
    println!("runtime_smoke=ok");
}

fn write_wav(path: &PathBuf) {
    let sample_rate = 44_100u32;
    let seconds = 2u32;
    let samples = sample_rate * seconds;
    let data_size = samples * 2;
    let mut file = File::create(path).expect("wav");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    for index in 0..samples {
        let t = index as f32 / sample_rate as f32;
        let sample = ((t * 440.0 * std::f32::consts::TAU).sin() * 3000.0) as i16;
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
}
"@

if ($Platform -eq "windows-x64") {
    $env:PATH = "$runtimeBin;$env:PATH"
}
if ($Platform -eq "macos-universal") {
    $env:DYLD_LIBRARY_PATH = "$runtimeLib;$env:DYLD_LIBRARY_PATH"
}
if ($Platform -eq "linux-x64") {
    $env:LD_LIBRARY_PATH = "$runtimeLib;$env:LD_LIBRARY_PATH"
}

Push-Location $probeRoot
try {
    & cargo run
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
    Remove-Item -LiteralPath $probeRoot -Recurse -Force -ErrorAction SilentlyContinue
}
