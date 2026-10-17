#!/usr/bin/env sh
# Build the workspace in debug mode.
set -eu
cd "$(dirname "$0")/.."
cargo build "$@"
