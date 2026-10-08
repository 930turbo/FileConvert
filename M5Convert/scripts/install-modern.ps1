param(
    [Parameter(Mandatory=$true)][string]$InstallDir
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$Root = (Resolve-Path "$PSScriptRoot\..").Path
$Manifest = Join-Path $Root 'package\AppxManifest.xml'

function Find-WindowsSdkTool([string]$Name) {
    $root = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (!(Test-Path $root)) { return $null }
    return Get-ChildItem $root -Directory | Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
}

$MakeAppx = Find-WindowsSdkTool 'makeappx.exe'
$SignTool = Find-WindowsSdkTool 'signtool.exe'
if (!$MakeAppx -or !$SignTool) {
    throw 'Modern Windows 11 menu packaging needs the official Microsoft Windows SDK (MakeAppx.exe and SignTool.exe). The classic menu is already installed and does not need it.'
}

$Cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq 'CN=M5Convert' -and $_.HasPrivateKey } | Select-Object -First 1
if (!$Cert) {
    $Cert = New-SelfSignedCertificate -Type Custom -Subject 'CN=M5Convert' `
        -KeyUsage DigitalSignature -CertStoreLocation Cert:\CurrentUser\My `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3','2.5.29.19={text}')
}

# Trust only this local development certificate for the current user. No CA or
# network operation is involved.
$Cer = Join-Path $env:TEMP 'M5Convert-local.cer'
Export-Certificate -Cert $Cert -FilePath $Cer -Force | Out-Null
Import-Certificate -FilePath $Cer -CertStoreLocation Cert:\CurrentUser\TrustedPeople | Out-Null
Remove-Item $Cer -Force -ErrorAction SilentlyContinue

$Stage = Join-Path $env:TEMP 'M5Convert-sparse-package'
$Msix = Join-Path $InstallDir 'M5Convert.Shell.msix'
Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Stage | Out-Null
Copy-Item $Manifest (Join-Path $Stage 'AppxManifest.xml')

& $MakeAppx pack /d $Stage /p $Msix /o | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }
& $SignTool sign /fd SHA256 /sha1 $Cert.Thumbprint /s My $Msix | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'SignTool failed.' }

Get-AppxPackage -Name 'M5Convert.Shell' -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction SilentlyContinue
Add-AppxPackage -Path $Msix -ExternalLocation $InstallDir -ForceApplicationShutdown
Write-Host 'Windows 11 modern context-menu identity registered.'
