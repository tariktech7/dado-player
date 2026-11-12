$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build --locked --release --bin Dado-Player
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
