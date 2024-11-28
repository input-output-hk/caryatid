#!/bin/bash
# Use 'tarpaulin' to generate test coverage stats
set -e
cargo install cargo-tarpaulin

# Exclude file from debug/oid-registry which is randomly included!
cargo tarpaulin --target-dir target/tarpaulin --skip-clean --workspace --exclude-files  target/debug/*/*/*/* "$@"
