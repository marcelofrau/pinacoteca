# Run the analysis harnesses (never shipped; see crates/pinacoteca-bench).
#
#   .\scripts\bench.ps1 H:\OneDrive\Pictures             decoders + mip pyramid
#   .\scripts\bench.ps1 --limit 50 H:\OneDrive\Pictures
#   .\scripts\bench.ps1 thumbs H:\OneDrive\Pictures      thumbnail pipeline end to end
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

if ($args.Count -ge 1 -and $args[0] -eq 'thumbs') {
    $rest = @()
    if ($args.Count -gt 1) { $rest = $args[1..($args.Count - 1)] }
    cargo run --release -p pinacoteca-bench --bin scan_thumbs -- $rest
} else {
    cargo run --release -p pinacoteca-bench --bin pinacoteca-bench -- $args
}
exit $LASTEXITCODE
