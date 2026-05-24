#!/usr/bin/env bash
# Runs the same checks as CI. Execute before opening a PR.
set -euo pipefail

cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

echo "All checks passed."