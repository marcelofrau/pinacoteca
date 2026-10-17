#!/usr/bin/env sh
# Run the test suite.
set -eu
cd "$(dirname "$0")/.."
cargo test --workspace "$@"
