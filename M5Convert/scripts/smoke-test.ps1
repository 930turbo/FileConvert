param(
    [string]$Exe = "$env:LOCALAPPDATA\Programs\M5Convert\m5convert.exe"
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path "$PSScriptRoot\..").Path
if (!(Test-Path $Exe)) { throw "Converter not found: $Exe" }
$Temp = Join-Path $env:TEMP "M5Convert-smoke-$PID"
New-Item -ItemType Directory -Force $Temp | Out-Null
try {
    Copy-Item "$Root\tests\fixtures\sample.png" $Temp
    Copy-Item "$Root\tests\fixtures\sample.svg" $Temp

    Write-Host 'PNG menu targets:'
    & $Exe targets "$Temp\sample.png" --menu
    if ($LASTEXITCODE -ne 0) { throw 'PNG target probe failed.' }

    & $Exe convert --to jpeg "$Temp\sample.png"
    if ($LASTEXITCODE -ne 0 -or !(Test-Path "$Temp\sample.jpg")) { throw 'PNG -> JPEG failed.' }

    & $Exe convert --to png "$Temp\sample.svg"
    if ($LASTEXITCODE -ne 0 -or !(Test-Path "$Temp\sample.png")) {
        # The existing input PNG forces collision-safe naming.
        if (!(Test-Path "$Temp\sample (2).png")) { throw 'SVG -> PNG failed.' }
    }

    Write-Host 'Built-in image/SVG smoke tests passed.'
} finally {
    Remove-Item $Temp -Recurse -Force -ErrorAction SilentlyContinue
}
