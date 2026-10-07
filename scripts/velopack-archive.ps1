#requires -Version 7.0
. (Join-Path $PSScriptRoot 'velopack-common.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem
function Read-VelopackEntry([IO.Compression.ZipArchiveEntry]$Entry,[int]$Limit=8388608) {
    if($null -eq $Entry -or $Entry.Length -gt $Limit){throw 'Missing or oversized package metadata'}
    $stream=$Entry.Open();try{return ,(Read-VelopackPrefix $stream ([int][Math]::Min([long]$Limit+1,$Entry.Length+1)))}finally{$stream.Dispose()}
}
function Assert-VelopackZipPaths([IO.Compression.ZipArchive]$Archive,[string]$Platform) {
    if($Archive.Entries.Count -gt 50000){throw 'Too many archive entries'}
    $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase);[long]$total=0
    foreach($entry in $Archive.Entries) {
        $name=$entry.FullName
        if($name -match '(^/|\\|:|[\x00-\x1f])' -or ($name.TrimEnd('/').Split('/') | Where-Object {$_ -in @('','..','.')})){throw 'Unsafe archive path'}
        if(-not $seen.Add($name.TrimEnd('/'))){throw 'Duplicate archive path'}
        $total+=$entry.Length
        if($entry.Length -gt 2147483648 -or $total -gt 4294967296){throw 'Uncompressed archive exceeds limits'}
        $kind=($entry.ExternalAttributes -shr 16) -band 0xf000
        if($kind -eq 0xa000) {
            if($Platform -notlike 'macos-*' -or $name -notmatch '(^|/)Contents/MacOS/sq\.version$'){throw 'Unexpected symbolic link in package'}
            $target=[Text.Encoding]::UTF8.GetString((Read-VelopackEntry $entry 1024))
            if($target -cne '../Resources/sq.version'){throw 'Unsafe symbolic link target'}
        }
    }
}
function Read-VelopackNuspec([byte[]]$Bytes) {
    $settings=[Xml.XmlReaderSettings]::new();$settings.DtdProcessing=[Xml.DtdProcessing]::Prohibit;$settings.XmlResolver=$null;$settings.MaxCharactersInDocument=8388608
    $input=[IO.MemoryStream]::new($Bytes,$false);$reader=[Xml.XmlReader]::Create($input,$settings)
    try {$doc=[Xml.XmlDocument]::new();$doc.XmlResolver=$null;$doc.Load($reader)}finally{$reader.Dispose();$input.Dispose()}
    $metadata=@{}
    foreach($field in @('id','version','channel','mainExe','os','rid','machineArchitecture')) {
        $nodes=$doc.SelectNodes("/*[local-name()='package']/*[local-name()='metadata']/*[local-name()='$field']")
        if($nodes.Count -ne 1){throw 'Ambiguous or incomplete NuGet metadata'}
        $metadata[$field]=$nodes[0].InnerText
    }
    return $metadata
}
function Assert-VelopackArchive {
    param([string]$Path,[string]$Platform,[string]$Version)
    Assert-VelopackVersion $Version; $t=Get-VelopackTarget $Platform
    if((Get-Item -LiteralPath $Path).Length -gt 2147483648){throw 'Update package exceeds size limit'}
    $zip=[IO.Compression.ZipFile]::OpenRead($Path)
    try {
        Assert-VelopackZipPaths $zip $Platform
        $meta=Read-VelopackNuspec (Read-VelopackEntry $zip.GetEntry('YoYoVideo.nuspec'))
        if($meta.id -cne 'YoYoVideo' -or $meta.version -cne $Version -or $meta.channel -cne $t.Channel -or $meta.mainExe -cne $t.Exe -or $meta.os -cne $t.Os -or $meta.rid -cne $t.Rid -or $meta.machineArchitecture -cne $t.Arch){throw 'Package identity, version, channel or architecture mismatch'}
        if($Platform -eq 'linux-x64') {
            $entry=$zip.GetEntry('lib/app/YoYoVideo.AppImage')
            if($null -eq $entry){throw 'Missing bundled AppImage'}
            $stream=$entry.Open();try{Assert-VelopackBinary -Bytes (Read-VelopackPrefix $stream) -Platform $Platform}finally{$stream.Dispose()}
            return [pscustomobject]@{Version=$Version;Platform=$Platform;Payload='lib/app/YoYoVideo.AppImage'}
        }
        $bin=if($Platform -eq 'windows-x64'){'lib/app/'}else{'lib/app/Contents/MacOS/'}
        $resources=if($Platform -eq 'windows-x64'){$bin}else{'lib/app/Contents/Resources/'}
        $installed=Read-VelopackNuspec (Read-VelopackEntry $zip.GetEntry($resources+'sq.version'))
        foreach($field in $meta.Keys){if($installed[$field] -cne $meta[$field]){throw 'Installed metadata differs from update package'}}
        foreach($name in @('README.md','LICENSE','LICENSES/README.md','LICENSES/runtime-provenance.md','LICENSES/Velopack-LICENSE.txt','yoyovideo-build-info.json')) {
            $entry=$zip.GetEntry($resources+$name);if($null -eq $entry -or $entry.Length -eq 0){throw "Missing packaged resource: $name"}
        }
        $info=[Text.Encoding]::UTF8.GetString((Read-VelopackEntry $zip.GetEntry($resources+'yoyovideo-build-info.json') 65536)) | ConvertFrom-Json
        if($info.schema -cne 'yoyovideo-build-info-v1' -or $info.version -cne $Version -or $info.mpv_runtime -ne $true -or $info.updater -ne $true){throw 'Packaged build metadata does not describe the required player'}
        foreach($entryName in @($t.Exe, $(if($Platform -eq 'windows-x64'){'mpv-2.dll'}else{'libmpv.dylib'}))) {
            $entry=$zip.GetEntry($bin+$entryName);if($null -eq $entry){throw "Missing packaged binary: $entryName"}
            $stream=$entry.Open();try{Assert-VelopackBinary -Bytes (Read-VelopackPrefix $stream) -Platform $Platform -Library:($entryName -ne $t.Exe)}finally{$stream.Dispose()}
        }
        $helper=if($Platform -eq 'windows-x64'){'Squirrel.exe'}else{'UpdateMac'}
        if($null -eq $zip.GetEntry($bin+$helper)){throw 'Missing native updater helper'}
        if($Platform -eq 'windows-x64' -and $null -eq $zip.GetEntry($bin+'YoYoVideo_ExecutionStub.exe')){throw 'Missing stable execution stub'}
        if($Platform -like 'macos-*' -and $null -eq $zip.GetEntry('lib/app/Contents/Info.plist')){throw 'Missing app bundle Info.plist'}
        return [pscustomobject]@{Version=$Version;Platform=$Platform;Payload=$bin+$t.Exe}
    } finally {$zip.Dispose()}
}
