#!/usr/bin/env sh
# Run the analysis harnesses (never shipped; see crates/pinacoteca-bench).
#
#   scripts/bench.sh /path/to/photos            decoders + mip pyramid
#   scripts/bench.sh --limit 50 /path/to/photos
#   scripts/bench.sh thumbs /path/to/photos     thumbnail pipeline end to end
set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "thumbs" ]; then
    shift
    exec cargo run --release -p pinacoteca-bench --bin scan_thumbs -- "$@"
fi

exec cargo run --release -p pinacoteca-bench --bin pinacoteca-bench -- "$@"
