#!/usr/bin/env bash
# Build the Nail website locally and ship it to the droplet.
#
#   ./scripts/deploy.sh root@<droplet-ip>
#   ./scripts/deploy.sh                     # host from DEPLOY_HOST in .env
#
# Nothing is built on the droplet - it receives a finished binary plus the data
# files the server reads at runtime.
#
# Build steps mirror run_website.sh: transpile examples/website/main.nail to
# Rust, regenerate the server's Cargo.toml, then cargo build --release. Set
# SKIP_TRANSPILE=1 to build the existing generated main.rs as-is.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# This app's identity on the box - must match what add-app.sh registered.
APP=nail
# Hardcoded in the Nail source (examples/website/main.nail), not settable by
# env: the stdlib HTTP server takes its port from the program itself.
APP_PORT=8080
BIN=nail_website_server

# The generated server project lives under target/ with the other build
# output. Everything in it is regenerated on each build, nothing is tracked.
SERVER_DIR=target/nail_website_server

# Files the server reads at runtime, relative to its working directory. Paths
# are kept identical on the droplet, so read_file("examples/...") still
# resolves. Keep in sync with the read_file calls in the generated main.rs.
# examples/website covers the whole site: sources, snippets, screenshots
# and assets all live inside it.
DATA_PATHS=(
	examples/website
	examples/mcp_dice_server.nail
	tests
	nail_language_spec.md
	README.md
	wasm_demos
	# Served at /install: the bootstrap one-liner the get-started section shows.
	bundle/install.sh
)

# Reads a key out of .env without sourcing it, so odd characters in a value
# (quotes, backslashes, $) stay literal.
env_val() {
	[[ -f .env ]] || return 0
	grep -E "^$1=" .env | tail -1 | cut -d= -f2- | sed 's/^"\(.*\)"$/\1/'
}

HOST="${1:-${DEPLOY_HOST:-}}"
if [[ "${HOST:-}" == --* ]]; then HOST=""; fi
[[ -z "$HOST" ]] && HOST="$(env_val DEPLOY_HOST)"
if [[ -z "$HOST" ]]; then
	echo "usage: $0 user@host   (or set DEPLOY_HOST in .env)" >&2
	exit 1
fi

echo "== documentation tests =="
# Everything this deploy ships that a person will read - README.md, the spec,
# the blog example's posts - carries Nail code and file paths the docs tests
# verify against the compiler and the repository. Stale documentation fails
# here, on this machine, not after it ships.
cargo test --quiet --lib docs

# DEPLOY_PASSWORD in .env means no prompts. The value goes to sshpass through
# the environment, never on a command line, so it stays out of `ps`.
SSH=(ssh -o StrictHostKeyChecking=accept-new)
SCP=(scp -o StrictHostKeyChecking=accept-new)
RSH="ssh -o StrictHostKeyChecking=accept-new"
DEPLOY_PASSWORD="${DEPLOY_PASSWORD:-$(env_val DEPLOY_PASSWORD)}"
if [[ -n "$DEPLOY_PASSWORD" ]]; then
	if ! command -v sshpass >/dev/null; then
		echo "DEPLOY_PASSWORD is set but sshpass is not installed (apt install sshpass)" >&2
		exit 1
	fi
	export SSHPASS="$DEPLOY_PASSWORD"
	SSH=(sshpass -e "${SSH[@]}")
	SCP=(sshpass -e "${SCP[@]}")
	RSH="sshpass -e $RSH"
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

if [[ "${SKIP_TRANSPILE:-0}" != "1" ]]; then
	echo "== transpiling examples/website/main.nail =="
	mkdir -p "$SERVER_DIR/src"
	# Deliberately built WITH profiling: the live timings section on the page
	# is this server reading its own profiler dump. Other production deploys
	# would pass --no-profile here.
	cargo run --quiet --bin nailc examples/website/main.nail --transpile
	[[ -s examples/website/main.rs ]] || { echo "transpile produced nothing" >&2; exit 1; }
	mv examples/website/main.rs "$SERVER_DIR/src/main.rs"
	cargo run --quiet --bin nailc examples/website/main.nail \
		--cargo-toml "--nail-path=../.." --package-name=nail_website_server \
		> "$SERVER_DIR/Cargo.toml"
fi

echo "== building browser demos =="
# The /games pages serve Nail programs compiled to WebAssembly. Built here so
# the rsync below ships fresh artifacts with everything else.
./scripts/build_wasm_demos.sh

echo "== building playground =="
# The editable example panes on the homepage check themselves against the
# real compiler, compiled to WebAssembly.
./scripts/build_playground_wasm.sh

echo "== building server =="
# x86-64-v3 (AVX2, BMI, FMA) matches the droplet's CPU and lets LLVM use
# wider vector registers than the portable baseline allows.
(cd "$SERVER_DIR" && RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release)

cp "$SERVER_DIR/target/release/$BIN" "$STAGE/$BIN"
strip "$STAGE/$BIN" 2>/dev/null || true
chmod +x "$STAGE/$BIN"
echo "   $(du -h "$STAGE/$BIN" | cut -f1) $(file -b "$STAGE/$BIN" | cut -d, -f1-2)"

echo "== uploading runtime data =="
# --relative keeps each path's directory structure under /srv/nail.
rsync -az --relative -e "$RSH" "${DATA_PATHS[@]}" "$HOST:/srv/$APP/"

echo "== uploading binary =="
# Write beside the live binary then mv: rename is atomic, so a request never
# hits a half-copied file, and the running process keeps its old inode.
"${SCP[@]}" -q "$STAGE/$BIN" "$HOST:/srv/$APP/$BIN.new"

# The server runs in the website's own directory, the same rule `nail run`
# applies, so its file-relative reads resolve. add-app.sh registers units
# with WorkingDirectory=/srv/<app>, so this app carries a drop-in override.
# Written idempotently on every deploy: it survives an add-app.sh re-run and
# costs nothing when already in place.
WORKDIR_OVERRIDE="/etc/systemd/system/$APP.service.d/workdir.conf"
"${SSH[@]}" "$HOST" "mkdir -p /etc/systemd/system/$APP.service.d \
	&& printf '[Service]\nWorkingDirectory=/srv/%s/examples/website\n' '$APP' > $WORKDIR_OVERRIDE \
	&& systemctl daemon-reload"

"${SSH[@]}" "$HOST" "mv /srv/$APP/$BIN.new /srv/$APP/$BIN \
	&& chown -R $APP:$APP /srv/$APP \
	&& systemctl restart $APP"

echo "== health check =="
sleep 2
if "${SSH[@]}" "$HOST" "curl -fsS -o /dev/null -w '%{http_code}' http://127.0.0.1:$APP_PORT/"; then
	echo " - up"
else
	echo " - FAILED; recent logs:" >&2
	"${SSH[@]}" "$HOST" "journalctl -u $APP -n 30 --no-pager" >&2
	exit 1
fi
