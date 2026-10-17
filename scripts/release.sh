#!/usr/bin/env sh
# Build the optimized release binary.
set -eu
cd "$(dirname "$0")/.."
cargo build --release "$@"
echo
echo "binary: target/release/pinacoteca"
