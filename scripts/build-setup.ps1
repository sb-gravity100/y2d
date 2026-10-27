$ErrorActionPreference = 'Stop'
function Get-Sha256([string]$filePath) {
    Write-Host '[y2d] calculating file SHA-256'
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($filePath)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    Write-Host '[y2d] building production application and silent installation payload'
    npm run tauri build -- --bundles nsis
    if ($LASTEXITCODE -ne 0) { throw 'Application bundle build failed' }
    $version = (Get-Content package.json -Raw | ConvertFrom-Json).version
    $payload = Join-Path $root "src-tauri/target/release/bundle/nsis/y2d_${version}_x64-setup.exe"
    if (!(Test-Path -LiteralPath $payload)) { throw "Missing x64 installation payload: $payload" }
    $env:Y2D_INSTALLER_PAYLOAD = [IO.Path]::GetFullPath($payload)
    Write-Host '[y2d] embedding the payload in the custom setup executable'
    cargo build --release --manifest-path src-setup/Cargo.toml --features custom-protocol --target-dir src-setup/target
    if ($LASTEXITCODE -ne 0) { throw 'Custom setup build failed' }
    New-Item -ItemType Directory -Force -Path artifacts | Out-Null
    $output = Join-Path $root "artifacts/y2d_${version}_x64-setup.exe"
    Copy-Item -LiteralPath (Join-Path $root 'src-setup/target/release/y2d-setup.exe') -Destination $output -Force
    $hash = (Get-Sha256 $output)
    Set-Content -LiteralPath "$output.sha256" -Value "$hash  $([IO.Path]::GetFileName($output))" -Encoding ASCII
    Write-Host "[y2d] custom installer ready: $output"
} finally {
    Remove-Item Env:Y2D_INSTALLER_PAYLOAD -ErrorAction SilentlyContinue
    Pop-Location
}
