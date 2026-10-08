param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\M5Convert"
)
$ErrorActionPreference = 'Stop'
$Clsid = '{4987C7B0-BFC5-41B0-94C8-7D06E3F1471C}'
$Classes = 'HKCU:\Software\Classes'

Get-AppxPackage -Name 'M5Convert.Shell' -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction SilentlyContinue
Remove-Item "$Classes\*\shell\M5Convert" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "$Classes\CLSID\$Clsid" -Recurse -Force -ErrorAction SilentlyContinue

# Explorer may hold the DLL until its process exits. Rename first when possible;
# this lets uninstallation succeed without killing Explorer.
if (Test-Path $InstallDir) {
    try {
        Remove-Item $InstallDir -Recurse -Force
    } catch {
        $old = "$InstallDir.remove-on-restart"
        Move-Item $InstallDir $old -Force
        Write-Warning "Explorer is still holding the shell DLL. Files were moved to $old; remove that folder after the next sign-out/restart."
    }
}
Write-Host 'M5Convert unregistered.'
