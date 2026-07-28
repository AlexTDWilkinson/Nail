#!/bin/bash

# Guard against bit rot in feature-gated stdlib modules (e.g. duckdb).
# Plain `cargo build` skips optional features, so gated runtime code would
# otherwise never be compiled during development. Run this after changing
# any feature-gated module. First run is slow (builds bundled DuckDB);
# incremental runs are fast.

set -e

echo "Checking nail lib with ALL features enabled..."
cargo check --lib --all-features
echo "✓ All feature-gated code compiles"
