$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    python scripts/release_version.py --check
    if ($LASTEXITCODE) { throw 'Produktversionen sind nicht synchron.' }
    cargo build --locked --release --bin Dado-Player
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
