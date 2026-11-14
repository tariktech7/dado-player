param([string]$TvIp = '', [Parameter(Mandatory=$true)][string]$SigningProfile, [string]$SdkRoot = '')
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
  if(Test-Path tizen.local.json){$local=Get-Content tizen.local.json -Raw | ConvertFrom-Json;if(!$SdkRoot){$SdkRoot=$local.sdkRoot};if(!$TvIp){$TvIp=$local.tvIp}}
  if(!$SdkRoot){$SdkRoot=$env:TIZEN_SDK_HOME}
  if(!$SdkRoot -or !$TvIp){throw 'Set SdkRoot/TvIp or create tizen.local.json (see README).'}
  if($SigningProfile -in @('default','develop')){throw 'Use a named Samsung TV certificate profile containing this TV DUID; default SDK certificates only work in emulators.'}
  $sdb=Join-Path $SdkRoot 'tools/sdb.exe';$tizen=Join-Path $SdkRoot 'tools/ide/bin/tizen.bat';$env:TIZEN_CLI=$tizen
  & $sdb connect "${TvIp}:26101"
  if($LASTEXITCODE){throw 'TV not reachable. Enable Developer Mode, enter this PC IP and reboot the TV.'}
  & npm.cmd run build
  if($LASTEXITCODE){throw 'Web build failed'}
  & npm.cmd run package -- --profile $SigningProfile
  if($LASTEXITCODE){throw 'Samsung certificate signing failed'}
  $version=(Get-Content package.json -Raw | ConvertFrom-Json).version
  $package=(Resolve-Path "../../Releases/Dado-Player-Tizen-$version.wgt").Path
  & $tizen install -s "${TvIp}:26101" -n (Split-Path $package -Leaf) -- (Split-Path $package -Parent)
  if($LASTEXITCODE){throw 'Installation failed. Check certificate DUID, Developer Mode and permit-to-install.'}
  & $tizen run -s "${TvIp}:26101" -p 'DadoPlayTV.Player'
  if($LASTEXITCODE){throw 'TV launch failed'}
} finally {Pop-Location}
