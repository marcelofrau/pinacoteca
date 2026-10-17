# Remove build output.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo clean
exit $LASTEXITCODE
