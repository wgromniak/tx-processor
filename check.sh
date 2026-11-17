#!/bin/sh
set -eu

cargo fmt --all -- --check
CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets --locked
CARGO_BUILD_WARNINGS=deny cargo test --workspace --locked