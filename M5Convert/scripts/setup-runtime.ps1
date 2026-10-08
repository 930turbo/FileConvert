$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$SourceDir = $PSScriptRoot
$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\M5Convert'
$Clsid = '{4987C7B0-BFC5-41B0-94C8-7D06E3F1471C}'

try {
    New-Item -ItemType Directory -Force $InstallDir | Out-Null

    foreach ($name in @('m5convert.exe','m5convert_shell.dll','ffmpeg.exe','pdfium.dll')) {
        $src = Join-Path $SourceDir $name
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $InstallDir $name) -Force
        }
    }

    foreach ($required in @('m5convert.exe','m5convert_shell.dll')) {
        if (!(Test-Path (Join-Path $InstallDir $required))) {
            throw "Installer payload is missing $required."
        }
    }

    $Classes = 'HKCU:\Software\Classes'
    $Com = "$Classes\CLSID\$Clsid\InprocServer32"
    New-Item -Path $Com -Force | Out-Null
    Set-Item -Path $Com -Value (Join-Path $InstallDir 'm5convert_shell.dll')
    New-ItemProperty -Path $Com -Name 'ThreadingModel' -PropertyType String -Value 'Apartment' -Force | Out-Null

    $Verb = "$Classes\*\shell\M5Convert"
    New-Item -Path $Verb -Force | Out-Null
    Set-Item -Path $Verb -Value 'Convert with M5Convert'
    New-ItemProperty -Path $Verb -Name 'ExplorerCommandHandler' -PropertyType String -Value $Clsid -Force | Out-Null

    $Cer = Join-Path $SourceDir 'M5Convert.cer'
    $MsixSource = Join-Path $SourceDir 'M5Convert.Shell.msix'
    if ((Test-Path $Cer) -and (Test-Path $MsixSource)) {
        Import-Certificate -FilePath $Cer -CertStoreLocation Cert:\CurrentUser\TrustedPeople | Out-Null
        $Msix = Join-Path $InstallDir 'M5Convert.Shell.msix'
        Copy-Item $MsixSource $Msix -Force
        Get-AppxPackage -Name 'M5Convert.Shell' -ErrorAction SilentlyContinue |
            Remove-AppxPackage -ErrorAction SilentlyContinue
        Add-AppxPackage -Path $Msix -ExternalLocation $InstallDir -ForceApplicationShutdown
    }

    $shell = New-Object -ComObject WScript.Shell
    [void]$shell.Popup(
        "M5Convert is installed. Right-click a supported file to use it.",
        5,
        "M5Convert",
        64
    )
    exit 0
}
catch {
    $shell = New-Object -ComObject WScript.Shell
    [void]$shell.Popup(
        "M5Convert installation failed:`r`n`r`n$($_.Exception.Message)",
        0,
        "M5Convert Setup",
        16
    )
    exit 1
}
