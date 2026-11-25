$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    python scripts/release_version.py --check
    if ($LASTEXITCODE) { throw 'Produktversionen sind nicht synchron.' }
    cargo build --locked --release --bin Dado-Player
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
    python packaging/prepare_windows_vlc.py target/release
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
