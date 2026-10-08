$ErrorActionPreference = 'Stop'
$InstallDir = "$env:LOCALAPPDATA\Programs\M5Convert"
$Exe = Join-Path $InstallDir 'm5convert.exe'
$Dll = Join-Path $InstallDir 'm5convert_shell.dll'
if (!(Test-Path $Exe)) { throw "Missing $Exe" }
if (!(Test-Path $Dll)) { throw "Missing $Dll" }

Write-Host '--- engines ---'
& $Exe engines
if ($LASTEXITCODE -ne 0) { throw 'Engine probe failed.' }

Write-Host '--- idle process check ---'
$running = Get-Process m5convert -ErrorAction SilentlyContinue
if ($running) { throw 'm5convert.exe is still running while idle.' }
Write-Host 'No resident m5convert.exe process.'

Write-Host '--- classic shell registration ---'
Get-ItemProperty 'HKCU:\Software\Classes\*\shell\M5Convert' | Format-List
Get-ItemProperty 'HKCU:\Software\Classes\CLSID\{4987C7B0-BFC5-41B0-94C8-7D06E3F1471C}\InprocServer32' | Format-List

Write-Host '--- modern package (optional) ---'
Get-AppxPackage -Name 'M5Convert.Shell' -ErrorAction SilentlyContinue | Select-Object Name,Version,InstallLocation

Write-Host 'Verification probes complete. A real Explorer right-click and at least one conversion of each installed engine are still required.'
