# Run the test suite.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo test --workspace @args
exit $LASTEXITCODE
