param(
    [string]$SourceDir = "$(Split-Path $PSScriptRoot -Parent)\dist",
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\M5Convert",
    [switch]$ModernMenu
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Clsid = '{4987C7B0-BFC5-41B0-94C8-7D06E3F1471C}'

$SourceDir = (Resolve-Path $SourceDir).Path
foreach ($required in @('m5convert.exe','m5convert_shell.dll')) {
    if (!(Test-Path (Join-Path $SourceDir $required))) { throw "Missing $required. Run scripts\build.ps1 first." }
}

New-Item -ItemType Directory -Force $InstallDir | Out-Null
Copy-Item "$SourceDir\*" $InstallDir -Recurse -Force

$Classes = 'HKCU:\Software\Classes'
$Com = "$Classes\CLSID\$Clsid\InprocServer32"
New-Item -Path $Com -Force | Out-Null
Set-Item -Path $Com -Value (Join-Path $InstallDir 'm5convert_shell.dll')
New-ItemProperty -Path $Com -Name 'ThreadingModel' -PropertyType String -Value 'Apartment' -Force | Out-Null

# Classic context menu registration. This remains a fallback even when the
# Windows 11 sparse identity package is enabled.
$Verb = "$Classes\*\shell\M5Convert"
New-Item -Path $Verb -Force | Out-Null
Set-Item -Path $Verb -Value 'Convert with M5Convert'
New-ItemProperty -Path $Verb -Name 'ExplorerCommandHandler' -PropertyType String -Value $Clsid -Force | Out-Null

if ($ModernMenu) {
    & "$PSScriptRoot\install-modern.ps1" -InstallDir $InstallDir
}

Write-Host "M5Convert installed in $InstallDir"
Write-Host 'Nothing was added to Startup and no service/background process was installed.'
Write-Host 'If Explorer already had the old extension loaded, reopen Explorer windows or restart Explorer once.'
