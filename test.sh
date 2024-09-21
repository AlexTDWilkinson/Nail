#!/bin/bash
RUSTFLAGS="-A unused_imports -A unused_variables -A dead_code -A unreachable_code" cargo test
