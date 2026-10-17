# Build the workspace in debug mode.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build @args
exit $LASTEXITCODE
