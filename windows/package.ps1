param([string]$Python = 'python')
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --locked --release --bin Dado-Player
    if ($LASTEXITCODE) { throw 'Windows-Build fehlgeschlagen.' }
    cargo fetch --locked --manifest-path packaging/launcher/Cargo.toml
    if ($LASTEXITCODE) { throw 'Launcher-Abhängigkeiten fehlen.' }
    Copy-Item target/release/Dado-Player.exe dist/Dado-Player.exe
    & $Python packaging/build_portable.py
    if ($LASTEXITCODE) { throw 'Portables Paket konnte nicht erstellt werden.' }
    New-Item -ItemType Directory -Path Releases -Force | Out-Null
    Copy-Item packaging/out/Dado-Player.exe Releases/Dado-Player.exe
} finally { Pop-Location }
