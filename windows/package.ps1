param([string]$Python = 'python', [string]$SigningThumbprint = '')
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    if ($SigningThumbprint) { $env:DADO_SIGN_CERT_THUMBPRINT = $SigningThumbprint }
    cargo build --locked --release --bin Dado-Player
    if ($LASTEXITCODE) { throw 'Windows-Build fehlgeschlagen.' }
    cargo fetch --locked --manifest-path packaging/launcher/Cargo.toml
    if ($LASTEXITCODE) { throw 'Launcher-Abhängigkeiten fehlen.' }
    Copy-Item target/release/Dado-Player.exe dist/Dado-Player.exe
    & $Python packaging/build_portable.py
    if ($LASTEXITCODE) { throw 'Portables Paket konnte nicht erstellt werden.' }
    New-Item -ItemType Directory -Path Releases -Force | Out-Null
    & $Python packaging/organize_releases.py --install-windows packaging/out/Dado-Player.exe
    if ($LASTEXITCODE) { throw 'Dado-Player.exe konnte nicht ersetzt werden. Laufende portable App schließen und Paketierung erneut ausführen.' }
} finally { Pop-Location }
