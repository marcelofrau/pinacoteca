# The same gates CI runs: formatting, lints and tests.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

Write-Host "== cargo fmt ==" -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo clippy ==" -ForegroundColor Cyan
cargo clippy --all-targets --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo test ==" -ForegroundColor Cyan
cargo test --workspace
exit $LASTEXITCODE
