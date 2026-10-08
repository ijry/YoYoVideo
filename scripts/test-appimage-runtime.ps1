#requires -Version 7.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'appimage-common.ps1')
function Check($value,$message){if(-not $value){throw $message}}
function Reject([scriptblock]$action){$failed=$false;try{& $action | Out-Null}catch{$failed=$true};Check $failed 'Expected rejection'}
foreach($name in @('libc.so.6','ld-linux-x86-64.so.2','libstdc++.so.6','libgcc_s.so.1','libGL.so.1','libEGL.so.1','libGLX_mesa.so.0','libdrm.so.2','libvulkan.so.1','libnvidia-glcore.so.555')){Check (Test-AppImageHostLibrary $name) "Must not bundle $name"}
foreach($name in @('libmpv.so.1','libmpv.so.2','libavcodec.so.58','libX11.so.6','libxkbcommon-x11.so.0','libwayland-client.so.0','libglib-2.0.so.0')){Check (-not (Test-AppImageHostLibrary $name)) "Must bundle $name"}
$lines=@('linux-vdso.so.1 (0x123)','libmpv.so.1 => /usr/lib/x86_64-linux-gnu/libmpv.so.1 (0x123)','/lib64/ld-linux-x86-64.so.2 (0x123)')
$deps=@(ConvertFrom-AppImageLdd $lines)
Check ($deps.Count -eq 2 -and $deps[0].Name -eq 'libmpv.so.1') 'ldd parsing failed'
Reject {ConvertFrom-AppImageLdd @('libmpv.so.2 => not found')}
Reject {ConvertFrom-AppImageLdd @('unexpected ldd output')}
$cache=@('libmpv.so.1 (libc6,x86-64) => /lib/x86_64-linux-gnu/libmpv.so.1','libmpv.so.2 (libc6) => /lib/i386-linux-gnu/libmpv.so.2')
$mpv=Resolve-AppImageLibmpv $cache
Check ($mpv.Name -eq 'libmpv.so.1') 'Must support Jammy libmpv1, never rename ABI'
Reject {Resolve-AppImageLibmpv @('libmpv.so.2 (libc6) => /lib/i386-linux-gnu/libmpv.so.2')}
Reject {Assert-AppImageBaseline @('ID=ubuntu','VERSION_ID="24.04"')}
Assert-AppImageBaseline @('ID=ubuntu','VERSION_ID="22.04"')
$launcher=Get-AppImageLauncher
Check ($launcher.Contains('LD_LIBRARY_PATH="$APPDIR/usr/bin') -and $launcher.Contains('exec "$APPDIR/usr/bin/yoyovideo-desktop" "$@"')) 'Launcher must preserve paths and arguments'
Check (($launcher -split [char]10) -contains 'export LD_LIBRARY_PATH="$APPDIR/usr/bin:${LD_LIBRARY_PATH:-}"') 'LD_LIBRARY_PATH must be a single shell line'
$seeds=@(Get-AppImageDlopenLibraries)
foreach($name in @('libxkbcommon-x11.so.0','libwayland-client.so.0','libX11.so.6')){Check ($seeds -contains $name) "Missing dlopen dependency $name"}
Write-Host 'PASS AppImage dependency policy, parsers, baseline, ABI selection and launcher'

# The legacy/.deb staging stays dependency-only for both distro ABI generations.
$repo=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$fixture=Join-Path $repo ('.cache/linux-staging-'+[guid]::NewGuid().ToString('N'))
foreach($dir in @('bin','docs','LICENSES')){New-Item -ItemType Directory -Force -Path (Join-Path $fixture $dir) | Out-Null}
foreach($name in @('README.md','RELEASE-NOTES.md','LICENSES/README.md','LICENSES/runtime-provenance.md','docs/runtime-dependencies.md','docs/manual-smoke-checklist.md','bin/yoyovideo-desktop')){'fixture' | Set-Content -LiteralPath (Join-Path $fixture $name)}
foreach($abi in @('1','2')) {
    @{package="libmpv$abi";library="libmpv.so.$abi";depends=@("libmpv$abi",'libc6')} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixture 'LICENSES/runtime-source.json')
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'verify-package.ps1') -Platform linux-x64 -PackageDir $fixture -RequireRuntime
    Check ($LASTEXITCODE -eq 0) "Dependency-only staging rejected libmpv$abi"
}
