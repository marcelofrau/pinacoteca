#!/usr/bin/env sh
# Run the app in debug mode.
#
#   scripts/run.sh                 open with an empty folder box
#   scripts/run.sh /path/to/photos open straight into that folder
set -eu
cd "$(dirname "$0")/.."
exec cargo run -p pinacoteca-app -- "$@"
