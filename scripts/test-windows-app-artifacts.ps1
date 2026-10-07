[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,
    [string]$ShortcutPath
)

$ErrorActionPreference = 'Stop'
$ExecutablePath = (Resolve-Path -LiteralPath $ExecutablePath).Path
$bytes = [System.IO.File]::ReadAllBytes($ExecutablePath)
$problems = [System.Collections.Generic.List[string]]::new()

function Read-U16([long]$Offset) {
    if ($Offset -lt 0 -or $Offset + 2 -gt $bytes.Length) { throw 'Truncated PE executable' }
    [BitConverter]::ToUInt16($bytes, [int]$Offset)
}
function Read-U32([long]$Offset) {
    if ($Offset -lt 0 -or $Offset + 4 -gt $bytes.Length) { throw 'Truncated PE executable' }
    [BitConverter]::ToUInt32($bytes, [int]$Offset)
}

if ((Read-U16 0) -ne 0x5a4d) { throw 'Not a Windows executable (missing MZ header)' }
$pe = Read-U32 0x3c
if ((Read-U32 $pe) -ne 0x4550) { throw 'Not a Windows executable (missing PE header)' }
$optional = $pe + 24
$magic = Read-U16 $optional
$directoryOffset = switch ($magic) {
    0x10b { 96 }
    0x20b { 112 }
    default { throw 'Unsupported PE optional header' }
}
$subsystem = Read-U16 ($optional + 68)
if ($subsystem -ne 2) {
    $problems.Add("Executable uses PE subsystem $subsystem; expected Windows GUI (2), not Console (3)")
}

# Check the produced binary, not Rust source text: a skipped resource compiler
# must not silently create a package with generic/missing shell icons.
$resourceRva = Read-U32 ($optional + $directoryOffset + 16)
$sectionCount = Read-U16 ($pe + 6)
$sectionTable = $optional + (Read-U16 ($pe + 20))
$resourceRoot = $null
for ($i = 0; $i -lt $sectionCount; $i++) {
    $section = $sectionTable + 40 * $i
    $virtualSize = Read-U32 ($section + 8)
    $virtualAddress = Read-U32 ($section + 12)
    $rawSize = Read-U32 ($section + 16)
    $rawAddress = Read-U32 ($section + 20)
    if ($resourceRva -ne 0 -and $resourceRva -ge $virtualAddress -and
        $resourceRva -lt $virtualAddress + [Math]::Max($virtualSize, $rawSize)) {
        $resourceRoot = $rawAddress + $resourceRva - $virtualAddress
        break
    }
}
$resourceTypes = @()
if ($null -ne $resourceRoot) {
    $entryCount = (Read-U16 ($resourceRoot + 12)) + (Read-U16 ($resourceRoot + 14))
    for ($i = 0; $i -lt $entryCount; $i++) {
        $id = Read-U32 ($resourceRoot + 16 + 8 * $i)
        if (($id -band 2147483648L) -eq 0) { $resourceTypes += $id }
    }
}
if (3 -notin $resourceTypes -or 14 -notin $resourceTypes) {
    $problems.Add('Executable is missing embedded RT_ICON / RT_GROUP_ICON resources')
}

if ($ShortcutPath) {
    if (-not $IsWindows) { throw 'Checking an installed shortcut requires Windows' }
    $ShortcutPath = (Resolve-Path -LiteralPath $ShortcutPath).Path
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    try {
        if (-not [string]::Equals($shortcut.TargetPath, $ExecutablePath, [StringComparison]::OrdinalIgnoreCase)) {
            $problems.Add("Shortcut does not launch the installed executable directly: $($shortcut.TargetPath)")
        }
        $icon = $shortcut.IconLocation
        $comma = $icon.LastIndexOf(',')
        $iconPath = if ($comma -ge 0) { $icon.Substring(0, $comma) } else { $icon }
        if (-not [string]::Equals($iconPath, $ExecutablePath, [StringComparison]::OrdinalIgnoreCase)) {
            $problems.Add("Shortcut icon must use the installed executable, not a build-machine path: $icon")
        }
    } finally {
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shortcut)
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)
    }
}
if ($problems.Count) { throw ($problems -join "`n") }
Write-Host "Windows app artifacts verified: GUI subsystem, embedded icon$(if ($ShortcutPath) { ', installed shortcut' })"
