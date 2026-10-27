$ErrorActionPreference = 'Stop'
function Get-Sha256([string]$filePath) {
    Write-Host '[y2d] calculating file SHA-256'
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($filePath)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
Write-Host '[y2d] preparing bundled FFmpeg tools'
$root = Split-Path -Parent $PSScriptRoot
$destination = Join-Path $root 'src-tauri/binaries'
$expected = 'a46195b14ea18385570a9b154868813495195f199b9c886d67b16d474c883b34'
$noticeDir = Join-Path $destination 'ffmpeg-notices'
$marker = Join-Path $noticeDir 'archive.sha256'
if ((Test-Path $marker) -and ((Get-Content $marker -Raw).Trim() -eq $expected) -and
    (Test-Path (Join-Path $destination 'ffmpeg.exe')) -and (Test-Path (Join-Path $destination 'ffprobe.exe'))) {
    Write-Host '[y2d] pinned FFmpeg bundle already prepared'
    exit 0
}
$temp = Join-Path ([IO.Path]::GetTempPath()) ('y2d-tools-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $archive = Join-Path $temp 'ffmpeg.zip'
    # Immutable release snapshot; never fetch the moving "latest" asset at build time.
    $url = 'https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-26-14-25/ffmpeg-N-128100-gceb466cbcd-win64-lgpl.zip'
    Write-Host '[y2d] downloading FFmpeg archive; SHA-256 verification is required'
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
    if ((Get-Sha256 $archive) -ne $expected) {
        throw 'FFmpeg archive hash differs from the pinned build. Update the pinned URL/hash together after reviewing upstream.'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        New-Item -ItemType Directory -Force -Path $noticeDir | Out-Null
        foreach ($name in @('ffmpeg.exe', 'ffprobe.exe')) {
            $entry = @($zip.Entries | Where-Object { $_.FullName.EndsWith('/bin/' + $name) })
            if ($entry.Count -ne 1) { throw "Archive must contain exactly one $name" }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry[0], (Join-Path $destination $name), $true)
        }
        foreach ($entry in $zip.Entries) {
            if ($entry.Name -and ($entry.FullName -match '/(LICENSE|COPYING|README)' -or $entry.FullName -match '/licenses/')) {
                $safeName = $entry.FullName.Replace('/', '_').Replace('\', '_')
                [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $noticeDir $safeName), $true)
            }
        }
        Set-Content -LiteralPath $marker -Value $expected -Encoding ASCII
    } finally { $zip.Dispose() }
    Write-Host '[y2d] bundled FFmpeg and ffprobe prepared'
} catch {
    Write-Error "[y2d] FFmpeg preparation failed: $_"
    throw
} finally {
    # This path was created above beneath the OS temporary directory.
    if (([IO.Path]::GetFullPath($temp)).StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath()))) {
        Remove-Item -LiteralPath $temp -Recurse -Force
    }
}
