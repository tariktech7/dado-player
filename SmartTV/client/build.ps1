param(
  [string]$SigningProfile = '',
  [string]$SdkRoot = '',
  [string]$LegacySigningProfile = ''
)
$ErrorActionPreference = 'Stop'
$previousProfile = $env:DADO_TIZEN_PROFILE
Push-Location $PSScriptRoot
try {
  if (!$SdkRoot -and (Test-Path tizen.local.json)) {
    $SdkRoot = (Get-Content tizen.local.json -Raw | ConvertFrom-Json).sdkRoot
  }
  if (!$SdkRoot) { $SdkRoot = $env:TIZEN_SDK_HOME }
  if ($SdkRoot) { $env:TIZEN_CLI = Join-Path $SdkRoot 'tools/ide/bin/tizen.bat' }

  & npm.cmd ci --no-fund --no-audit
  if ($LASTEXITCODE) { throw 'npm ci failed' }

  Remove-Item Env:DADO_TIZEN_PROFILE -ErrorAction SilentlyContinue
  & npm.cmd run check
  if ($LASTEXITCODE) { throw 'Source validation failed' }
  if (!$LegacySigningProfile) { $LegacySigningProfile = $SigningProfile }
  $packageDir = if ($env:DADO_PACKAGE_DIR) { $env:DADO_PACKAGE_DIR } else { '../../Releases' }
  $version = (Get-Content package.json -Raw | ConvertFrom-Json).releaseVersion
  $packages = foreach ($variant in @(
    @{ Profile = $SigningProfile; Prefix = 'Dado-Player-Tizen' },
    @{ Profile = $LegacySigningProfile; Prefix = 'Dado-Player-Tizen5'; Engine = '2019' }
  )) {
    if ($variant.Engine) {
      $env:DADO_TIZEN_PROFILE = $variant.Engine
      & npm.cmd run build | Out-Host
      if ($LASTEXITCODE) { throw 'Tizen 5 build failed' }
    }
    $profile = if ($variant.Profile) { $variant.Profile } else { $env:TIZEN_SIGNING_PROFILE }
    if ($profile) { & npm.cmd run package -- --profile $profile | Out-Host }
    else { & npm.cmd run package | Out-Host }
    if ($LASTEXITCODE) { throw 'Packaging failed' }
    $suffix = if ($profile) { '.wgt' } else { '-unsigned.wgt' }
    $file = Join-Path $packageDir "$($variant.Prefix)-$version$suffix"
    & npm.cmd run verify:package -- $file | Out-Host
    if ($LASTEXITCODE) { throw 'Widget verification failed' }
    $file
  }
  & npm.cmd run verify:package -- $packages[0] $packages[1]
  if ($LASTEXITCODE) { throw 'Tizen application files differ by profile' }
  $env:DADO_TIZEN_PROFILE = $previousProfile
  & npm.cmd run build
  if ($LASTEXITCODE) { throw 'Could not restore the selected build profile' }
} finally {
  $env:DADO_TIZEN_PROFILE = $previousProfile
  Pop-Location
}
