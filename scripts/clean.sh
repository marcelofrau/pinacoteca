#!/usr/bin/env sh
# Remove build output.
set -eu
cd "$(dirname "$0")/.."
cargo clean
