#!/bin/bash

# Set the RUST_LOG environment variable to debug
# export RUST_LOG=debug
export RUST_BACKTRACE=full
export RUSTFLAGS="-A unused_imports -A unused_variables -A dead_code -A unreachable_code"

# Run cargo watch with the run command
cargo watch -x run

# Logs are written to the nail.log in the current directory, as they can't be displayed on the console
