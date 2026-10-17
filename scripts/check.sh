#!/usr/bin/env sh
# The same gates CI runs: formatting, lints and tests.
set -eu
cd "$(dirname "$0")/.."

echo "== cargo fmt =="
cargo fmt --all -- --check

echo "== cargo clippy =="
cargo clippy --all-targets --workspace -- -D warnings

echo "== cargo test =="
cargo test --workspace
