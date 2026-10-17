# Run the optimized app. Best for large folders.
#
#   .\scripts\run-release.ps1 H:\OneDrive\Pictures
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$folder = $args
if ($folder.Count -gt 0) {
    cargo run --release -p pinacoteca-app -- $folder
} else {
    cargo run --release -p pinacoteca-app
}
exit $LASTEXITCODE
