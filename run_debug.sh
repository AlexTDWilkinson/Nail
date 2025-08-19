#!/bin/bash

# Run the IDE in debug mode
export NAIL_DEBUG=1
export RUST_LOG=debug

echo "Starting Nail IDE in debug mode..."
echo "Logs will be written to nail.log"
echo "To watch logs in another terminal: tail -f nail.log"
echo ""

cargo run -- "$@"