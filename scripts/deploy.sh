#!/usr/bin/env bash
# Build the Nail website locally and ship it to the droplet.
#
#   ./scripts/deploy.sh root@<droplet-ip>
#   ./scripts/deploy.sh                     # host from DEPLOY_HOST in .env
#
# Nothing is built on the droplet - it receives a finished binary plus the data
# files the server reads at runtime.
#
# Build steps mirror run_website.sh: transpile examples/nail_website.nail to
# Rust, regenerate the server's Cargo.toml, then cargo build --release. Set
# SKIP_TRANSPILE=1 to build the existing generated main.rs as-is.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# This app's identity on the box - must match what add-app.sh registered.
APP=nail
# Hardcoded in the Nail source (examples/nail_website.nail), not settable by
# env: the stdlib HTTP server takes its port from the program itself.
APP_PORT=8080
BIN=nail_website_server

# Files the server reads at runtime, relative to its working directory. Paths
# are kept identical on the droplet, so read_file("examples/...") still
# resolves. Keep in sync with the read_file calls in the generated main.rs.
DATA_PATHS=(
	examples/website_examples
	examples/website_screenshots
	examples/website_assets
	examples/nail_website.nail
	tests
	nail_language_spec.md
	README.md
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
	echo "== transpiling nail_website.nail =="
	mkdir -p nail_website_server/src
	cargo run --quiet --bin nailc examples/nail_website.nail --transpile
	[[ -s examples/nail_website.rs ]] || { echo "transpile produced nothing" >&2; exit 1; }
	mv examples/nail_website.rs nail_website_server/src/main.rs
	cargo run --quiet --bin nailc examples/nail_website.nail \
		--cargo-toml "--nail-path=.." --package-name=nail_website_server \
		> nail_website_server/Cargo.toml
fi

echo "== building server =="
(cd nail_website_server && cargo build --release)

cp "nail_website_server/target/release/$BIN" "$STAGE/$BIN"
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
