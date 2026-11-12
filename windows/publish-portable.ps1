param([string]$Git = 'git')
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $version = (Get-Content windows/VERSION -Raw).Trim()
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Ungültige Windows-Version.' }
    $ref = "refs/tags/portable-v$version"
    $file = Get-Item Releases/Dado-Player.exe
    if ($file.Length -gt 100MB) { throw 'Portable EXE überschreitet das GitHub-Dateilimit.' }
    $localFile = (Get-FileHash $file.FullName).Hash
    if ($localFile -ne (Get-FileHash packaging/out/Dado-Player.exe).Hash) { throw 'Release entspricht nicht dem zuletzt gepackten Launcher.' }
    $source = & $Git rev-parse HEAD
    if ($LASTEXITCODE) { throw 'Kein Quellcommit gefunden.' }
    $blob = & $Git hash-object -w -- $file.FullName
    if ($LASTEXITCODE) { throw 'EXE konnte nicht in Git übernommen werden.' }
    $tree = "100644 blob $blob`tDado-Player.exe" | & $Git mktree
    if ($LASTEXITCODE) { throw 'Release-Baum konnte nicht erstellt werden.' }
    $commit = & $Git commit-tree $tree -m "Portable Windows $version from $source"
    if ($LASTEXITCODE) { throw 'Release-Commit fehlgeschlagen.' }
    # Lightweight tag, single package tree. Never overwrite a published version.
    & $Git push origin "${commit}:$ref"
    if ($LASTEXITCODE) { throw 'Update-Tag wurde nicht veröffentlicht.' }
} finally { Pop-Location }
