param(
    [string]$Target = 'x86_64-pc-windows-msvc'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path "$PSScriptRoot\..").Path
$Dist = Join-Path $Root 'dist'
$Stage = Join-Path $Root 'target\installer-stage'
$MsixStage = Join-Path $Root 'target\msix-stage'
$Setup = Join-Path $Root 'scripts\setup-runtime.ps1'
$IExpress = Join-Path $env:SystemRoot 'System32\iexpress.exe'

foreach ($required in @('m5convert.exe','m5convert_shell.dll')) {
    if (!(Test-Path (Join-Path $Dist $required))) {
        throw "Missing dist\$required. Run scripts\build.ps1 first."
    }
}
if (!(Test-Path $IExpress)) {
    throw "Windows IExpress was not found at $IExpress."
}

function Find-WindowsSdkTool([string]$Name) {
    $root = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (!(Test-Path $root)) { return $null }
    return Get-ChildItem $root -Directory |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$Name" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
}

$MakeAppx = Find-WindowsSdkTool 'makeappx.exe'
$SignTool = Find-WindowsSdkTool 'signtool.exe'
if (!$MakeAppx -or !$SignTool) {
    throw 'The GitHub Windows runner is missing MakeAppx.exe or SignTool.exe.'
}

Remove-Item $MsixStage,$Stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $MsixStage,$Stage | Out-Null

Copy-Item (Join-Path $Root 'package\AppxManifest.xml') (Join-Path $MsixStage 'AppxManifest.xml')
Copy-Item (Join-Path $Root 'package\Assets') $MsixStage -Recurse -Force

$Msix = Join-Path $Dist 'M5Convert.Shell.msix'
$Cer = Join-Path $Dist 'M5Convert.cer'
Remove-Item $Msix,$Cer -Force -ErrorAction SilentlyContinue

$Cert = New-SelfSignedCertificate -Type Custom -Subject 'CN=M5Convert' `
    -KeyUsage DigitalSignature -CertStoreLocation Cert:\CurrentUser\My `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3','2.5.29.19={text}')
try {
    Export-Certificate -Cert $Cert -FilePath $Cer -Force | Out-Null

    & $MakeAppx pack /d $MsixStage /p $Msix /o | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }

    & $SignTool sign /fd SHA256 /sha1 $Cert.Thumbprint /s My $Msix | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed.' }
}
finally {
    Remove-Item "Cert:\CurrentUser\My\$($Cert.Thumbprint)" -Force -ErrorAction SilentlyContinue
}

Copy-Item (Join-Path $Dist 'm5convert.exe') $Stage
Copy-Item (Join-Path $Dist 'm5convert_shell.dll') $Stage
Copy-Item $Msix,$Cer,$Setup $Stage
foreach ($optional in @('ffmpeg.exe','pdfium.dll')) {
    $src = Join-Path $Dist $optional
    if (Test-Path $src) { Copy-Item $src $Stage }
}

$Files = Get-ChildItem $Stage -File | Sort-Object Name
$Sed = Join-Path $Stage 'M5Convert.sed'
$TargetExe = Join-Path $Dist 'M5Convert-Setup.exe'

$Strings = @()
$FileEntries = @()
for ($i = 0; $i -lt $Files.Count; $i++) {
    $key = "FILE$i"
    $Strings += "$key=`"$($Files[$i].Name)`""
    $FileEntries += "%$key%="
}

$StageEscaped = $Stage
$SedText = @"
[Version]
Class=IEXPRESS
SEDVersion=3
[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=0
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=
TargetName=$TargetExe
FriendlyName=M5Convert Setup
AppLaunched=powershell.exe -NoProfile -ExecutionPolicy Bypass -File setup-runtime.ps1
PostInstallCmd=<None>
AdminQuietInstCmd=
UserQuietInstCmd=
SourceFiles=SourceFiles
[Strings]
$($Strings -join "`r`n")
[SourceFiles]
SourceFiles0=$StageEscaped\
[SourceFiles0]
$($FileEntries -join "`r`n")
"@

Set-Content -LiteralPath $Sed -Value $SedText -Encoding ASCII
& $IExpress /N /Q $Sed
if ($LASTEXITCODE -ne 0 -or !(Test-Path $TargetExe)) {
    throw 'IExpress failed to create M5Convert-Setup.exe.'
}

$Size = (Get-Item $TargetExe).Length
Write-Host ("Created {0} ({1:N1} MiB)" -f $TargetExe, ($Size / 1MB))
