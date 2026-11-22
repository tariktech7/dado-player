param(
  [string]$TvIp = '',
  [string]$SigningProfile = '',
  [string]$SdkRoot = '',
  [switch]$FromGitHub,
  [switch]$Tizen5,
  [switch]$Run
)
$ErrorActionPreference='Stop'

function Get-ReleasePackage([string]$Directory) {
  $repoApi = 'https://api.github.com/repos/tariktech7/dado-player/releases'
  $token = $env:DADO_GITHUB_TOKEN
  if (!$token) {
    $git = Get-Command git.exe -ErrorAction SilentlyContinue
    if ($git) {
      $lines = "protocol=https`nhost=github.com`npath=tariktech7/dado-player.git`n`n" |
        & $git.Source -c credential.interactive=never credential fill 2>$null
      $tokenLine = $lines | Where-Object { $_.StartsWith('password=') } | Select-Object -First 1
      if ($tokenLine) { $token = $tokenLine.Substring(9) }
    }
  }
  $headers = @{
    'Accept' = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
  }
  if ($token) { $headers['Authorization'] = "Bearer $token" }
  $releases = Invoke-RestMethod -Uri "${repoApi}?per_page=30" -Headers $headers -UserAgent 'DadoPlayer-Tizen-Update' -TimeoutSec 30
  $prefix = if ($Tizen5) { 'Dado-Player-Tizen5-' } else { 'Dado-Player-Tizen-' }
  $selected = foreach ($release in $releases) {
    if ($release.draft -or $release.tag_name -notmatch '^v(\d+\.\d+\.\d+)(?:-beta)?$') { continue }
    $version = [version]$Matches[1]
    $name = "$prefix$version.wgt"
    foreach ($asset in $release.assets) {
      if ($asset.name -eq $name -and $asset.label -match '^tizen-build-(\d+)$') {
        [PSCustomObject]@{ Version=$version; Build=[int64]$Matches[1]; Asset=$asset }
      }
    }
  }
  $selected = $selected | Sort-Object Version,Build -Descending | Select-Object -First 1
  if (!$selected) { throw 'Kein signiertes Tizen-Paket mit Buildnummer im GitHub-Release gefunden.' }
  $asset = $selected.Asset
  if ($asset.size -le 0 -or $asset.size -gt 20MB -or $asset.digest -notmatch '^sha256:([a-f0-9]{64})$') {
    throw 'Ungültige Größe oder fehlende SHA-256-Prüfsumme.'
  }
  $expected = $Matches[1]
  $package = Join-Path $Directory $asset.name
  $headers['Accept'] = 'application/octet-stream'
  Invoke-WebRequest -Uri "$repoApi/assets/$($asset.id)" -Headers $headers -UserAgent 'DadoPlayer-Tizen-Update' -TimeoutSec 90 -OutFile $package
  if ((Get-Item -LiteralPath $package).Length -ne $asset.size -or (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash -ne $expected) {
    throw 'Das heruntergeladene Paket stimmt nicht mit dem GitHub-Release überein.'
  }
  $inspection = & node scripts/verify-package.mjs $package
  if ($LASTEXITCODE -or !($inspection | Out-String | Select-String '"signatureFilesPresent": true')) {
    throw 'Das Paket enthält keine Samsung-Signatur.'
  }
  Write-Host "GitHub-Update $($selected.Version) Build $($selected.Build) geprüft."
  return $package
}

$downloadDir = $null
Push-Location $PSScriptRoot
try {
  if(Test-Path tizen.local.json){$local=Get-Content tizen.local.json -Raw | ConvertFrom-Json;if(!$SdkRoot){$SdkRoot=$local.sdkRoot};if(!$TvIp){$TvIp=$local.tvIp}}
  if(!$SdkRoot){$SdkRoot=$env:TIZEN_SDK_HOME}
  if(!$SdkRoot -or !$TvIp){throw 'Set SdkRoot/TvIp or create tizen.local.json (see README).'}
  if(!$FromGitHub -and (!$SigningProfile -or $SigningProfile -in @('default','develop'))){throw 'Use a named Samsung TV certificate profile containing this TV DUID; default SDK certificates only work in emulators.'}
  $sdb=Join-Path $SdkRoot 'tools/sdb.exe';$tizen=Join-Path $SdkRoot 'tools/ide/bin/tizen.bat';$env:TIZEN_CLI=$tizen
  & $sdb connect "${TvIp}:26101"
  if($LASTEXITCODE){throw 'TV not reachable. Enable Developer Mode, enter this PC IP and reboot the TV.'}
  if ($FromGitHub) {
    $downloadDir = Join-Path ([IO.Path]::GetTempPath()) ('dado-tizen-update-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $downloadDir | Out-Null
    $package = Get-ReleasePackage $downloadDir
  } else {
    & npm.cmd run build
    if($LASTEXITCODE){throw 'Web build failed'}
    & npm.cmd run package -- --profile $SigningProfile
    if($LASTEXITCODE){throw 'Samsung certificate signing failed'}
    $version=(Get-Content package.json -Raw | ConvertFrom-Json).version
    $package=(Resolve-Path "../../Releases/Dado-Player-Tizen-$version.wgt").Path
  }
  & $tizen install -s "${TvIp}:26101" -n (Split-Path $package -Leaf) -- (Split-Path $package -Parent)
  if($LASTEXITCODE){throw 'Installation failed. Check certificate DUID, Developer Mode and permit-to-install.'}
  if ($Run) {
    & $tizen run -s "${TvIp}:26101" -p 'DadoPlayTV.Player'
    if($LASTEXITCODE){throw 'TV launch failed'}
  }
} finally {
  if ($downloadDir -and (Test-Path -LiteralPath $downloadDir)) {
    Get-ChildItem -LiteralPath $downloadDir -File | Remove-Item -Force
    Remove-Item -LiteralPath $downloadDir
  }
  Pop-Location
}
