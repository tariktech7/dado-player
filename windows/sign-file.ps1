param(
    [Parameter(Mandatory)][string]$File,
    [Parameter(Mandatory)][string]$Thumbprint,
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)
$ErrorActionPreference = 'Stop'
$path = (Resolve-Path -LiteralPath $File).Path
$cert = Get-Item -LiteralPath "Cert:/CurrentUser/My/$Thumbprint"
if (!$cert.HasPrivateKey -or $cert.NotAfter -le (Get-Date)) { throw 'Gültiges Zertifikat mit privatem Schlüssel erforderlich.' }
if ($cert.Subject -eq $cert.Issuer) { throw 'Ein selbstsigniertes Zertifikat beseitigt die öffentliche Herausgeberwarnung nicht.' }
if ('1.3.6.1.5.5.7.3.3' -notin @($cert.EnhancedKeyUsageList | ForEach-Object { [string]$_.ObjectId })) { throw 'Zertifikat ist nicht für Code Signing ausgestellt.' }
$tool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if ($tool) { $toolPath = $tool.Source } else {
    $toolPath = Get-ChildItem "${env:ProgramFiles(x86)}/Windows Kits/10/bin/*/x64/signtool.exe" |
        Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (!$toolPath) { throw 'SignTool aus dem Windows SDK fehlt.' }
& $toolPath sign /sha1 $Thumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $path
if ($LASTEXITCODE) { throw 'Authenticode-Signierung fehlgeschlagen.' }
& $toolPath verify /pa /all $path
if ($LASTEXITCODE) { throw 'Authenticode-Vertrauensprüfung fehlgeschlagen.' }
$signature = Get-AuthenticodeSignature -LiteralPath $path
if ($signature.Status -ne 'Valid' -or !$signature.TimeStamperCertificate) { throw 'Signatur oder vertrauenswürdiger Zeitstempel fehlt.' }
Write-Host "Signiert und geprüft: $path ($($signature.SignerCertificate.Subject))"
