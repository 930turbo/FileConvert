param(
    [string]$Target = 'x86_64-pc-windows-msvc'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Root = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $Root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo is not installed. Install the official Rust toolchain from https://rustup.rs/ and rerun.'
}
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw 'rustup is required so the Windows MSVC target can be verified.'
}

rustup target add $Target | Out-Host
cargo build --release --target $Target -p m5convert -p m5convert-shell
if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }

$Dist = Join-Path $Root 'dist'
if (Test-Path $Dist) { Remove-Item $Dist -Recurse -Force }
New-Item -ItemType Directory -Force $Dist, "$Dist\Assets" | Out-Null
Copy-Item "$Root\target\$Target\release\m5convert.exe" $Dist
Copy-Item "$Root\target\$Target\release\m5convert_shell.dll" $Dist
Copy-Item "$Root\package\Assets\*" "$Dist\Assets" -Force

foreach ($name in @('ffmpeg.exe','pdfium.dll')) {
    $candidate = Join-Path $Root "vendor\$name"
    if (Test-Path $candidate) { Copy-Item $candidate $Dist }
}

Write-Host "Built: $Dist"
Write-Host 'Run .\scripts\install.ps1 to install for the current user.'
