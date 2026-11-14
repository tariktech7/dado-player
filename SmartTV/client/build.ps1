param([string]$SigningProfile = '', [string]$SdkRoot = '')
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
  if(!$SdkRoot -and (Test-Path tizen.local.json)){ $SdkRoot=(Get-Content tizen.local.json -Raw | ConvertFrom-Json).sdkRoot }
  if(!$SdkRoot){$SdkRoot=$env:TIZEN_SDK_HOME}
  if($SdkRoot){$env:TIZEN_CLI=Join-Path $SdkRoot 'tools/ide/bin/tizen.bat'}
  & npm.cmd ci --no-fund --no-audit
  if($LASTEXITCODE){throw 'npm ci failed'}
  & npm.cmd run check
  if($LASTEXITCODE){throw 'Source validation failed'}
  if($SigningProfile){& npm.cmd run package -- --profile $SigningProfile}else{& npm.cmd run package}
  if($LASTEXITCODE){throw 'Packaging failed'}
} finally {Pop-Location}
