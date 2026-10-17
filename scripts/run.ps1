# Run the app in debug mode.
#
#   .\scripts\run.ps1                       open with an empty folder box
#   .\scripts\run.ps1 H:\OneDrive\Pictures  open straight into that folder
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$folder = $args
if ($folder.Count -gt 0) {
    cargo run -p pinacoteca-app -- $folder
} else {
    cargo run -p pinacoteca-app
}
exit $LASTEXITCODE
