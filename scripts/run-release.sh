#!/usr/bin/env sh
# Run the optimized app. Best for large folders.
#
#   scripts/run-release.sh /path/to/photos
set -eu
cd "$(dirname "$0")/.."
exec cargo run --release -p pinacoteca-app -- "$@"
