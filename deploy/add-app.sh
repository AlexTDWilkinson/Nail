#!/usr/bin/env bash
# PER-APP setup. Run once per app on a box already prepared by
# provision-base.sh. Creates everything that app owns and nothing else:
#
#   /srv/<name>            its directory, mode 0750, owned by its own user
#   user <name>            a locked system account, no shell, no home
#   <name>.service         systemd unit, sandboxed to /srv/<name>
#   sites.d/<name>.caddy   its routing fragment, imported by the shared config
#
# Usage:
#   add-app.sh --name sul --port 3000 --bin simple-universal-language
#   add-app.sh --name blog --port 3001 --bin blog --host blog.example.com
#
# Options:
#   --host <domain>   serve this hostname over HTTPS (certificate is automatic).
#                     Omitted means "serve this app on the bare IP over HTTP" -
#                     only one app on the box can hold that.
#   --mem <size>      hard memory ceiling, default 192M.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
	echo "run as root" >&2
	exit 1
fi

NAME="" PORT="" BIN="" HOST="" MEM="192M"
while [[ $# -gt 0 ]]; do
	case "$1" in
		--name) NAME="$2"; shift 2 ;;
		--port) PORT="$2"; shift 2 ;;
		--bin)  BIN="$2";  shift 2 ;;
		--host) HOST="$2"; shift 2 ;;
		--mem)  MEM="$2";  shift 2 ;;
		*) echo "unknown option: $1" >&2; exit 1 ;;
	esac
done

if [[ -z "$NAME" || -z "$PORT" || -z "$BIN" ]]; then
	echo "usage: $0 --name <app> --port <port> --bin <binary> [--host <domain>] [--mem 192M]" >&2
	exit 1
fi
[[ "$NAME" =~ ^[a-z][a-z0-9-]{0,30}$ ]] || { echo "--name must be lowercase letters, digits, dashes" >&2; exit 1; }
[[ "$PORT" =~ ^[0-9]+$ ]] || { echo "--port must be a number" >&2; exit 1; }

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE="$DIR/app.service.template"
[[ -f "$TEMPLATE" ]] || { echo "missing $TEMPLATE" >&2; exit 1; }

# Refuse to steal a port another app already claimed - that would silently
# route one app's traffic into another.
for unit in /etc/systemd/system/*.service; do
	[[ -e "$unit" ]] || continue
	[[ "$(basename "$unit" .service)" == "$NAME" ]] && continue
	if grep -qs "^Environment=PORT=$PORT\$" "$unit"; then
		echo "port $PORT is already used by $(basename "$unit" .service)" >&2
		exit 1
	fi
done

echo "== user and directory =="
id -u "$NAME" >/dev/null 2>&1 || useradd --system --home "/srv/$NAME" --shell /usr/sbin/nologin "$NAME"
mkdir -p "/srv/$NAME/static"
# 0750: this app's files are unreadable to the other apps' accounts.
chown -R "$NAME:$NAME" "/srv/$NAME"
chmod 750 "/srv/$NAME"

if [[ ! -f "/srv/$NAME/env" ]]; then
	: >"/srv/$NAME/env"
fi
chown "$NAME:$NAME" "/srv/$NAME/env"
chmod 600 "/srv/$NAME/env"

echo "== systemd unit =="
# MemoryHigh throttles at 75% of the hard ceiling, so a leak slows down and
# gets logged before the kernel kills it.
MEMHIGH="$MEM"
if [[ "$MEM" =~ ^([0-9]+)M$ ]]; then
	MEMHIGH="$(( ${BASH_REMATCH[1]} * 3 / 4 ))M"
fi
sed -e "s|@NAME@|$NAME|g" \
    -e "s|@PORT@|$PORT|g" \
    -e "s|@BIN@|$BIN|g" \
    -e "s|@MEMMAX@|$MEM|g" \
    -e "s|@MEMHIGH@|$MEMHIGH|g" \
    "$TEMPLATE" >"/etc/systemd/system/$NAME.service"
systemctl daemon-reload
systemctl enable "$NAME" >/dev/null

echo "== caddy fragment =="
if [[ -n "$HOST" ]]; then
	cat >"/etc/caddy/sites.d/$NAME.caddy" <<EOF
$HOST {
	encode zstd gzip
	reverse_proxy 127.0.0.1:$PORT
}
EOF
	# The www. form of the same name is a hostname a visitor will type or a
	# link will carry, and without a block for it Caddy has no certificate to
	# offer, so the browser fails the handshake before it can be told where to
	# go. One canonical host: www redirects to the bare name, path and query
	# intact. Skipped when --host is itself a www. name, which would loop.
	if [[ "$HOST" != www.* ]]; then
		cat >>"/etc/caddy/sites.d/$NAME.caddy" <<EOF

www.$HOST {
	redir https://$HOST{uri} permanent
}
EOF
	fi
else
	# Bare-IP mode. Only one app can answer for "any hostname", so take the
	# placeholder's slot and refuse if a different app already holds it.
	for frag in /etc/caddy/sites.d/*.caddy; do
		[[ -e "$frag" ]] || continue
		base="$(basename "$frag" .caddy)"
		[[ "$base" == "$NAME" || "$base" == "00-placeholder" ]] && continue
		if grep -qE '^\s*:80\s*\{' "$frag"; then
			echo "$base already serves the bare IP; pass --host <domain> for this app" >&2
			exit 1
		fi
	done
	rm -f /etc/caddy/sites.d/00-placeholder.caddy
	cat >"/etc/caddy/sites.d/$NAME.caddy" <<EOF
# No domain yet: answer on the bare droplet IP over plain HTTP. Replace this
# whole file with a hostname block (see --host) once DNS points here, and
# HTTPS turns on by itself.
:80 {
	encode zstd gzip
	reverse_proxy 127.0.0.1:$PORT
}
EOF
fi

caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null
systemctl reload caddy || systemctl restart caddy

cat <<EOF

App '$NAME' registered:
  port     127.0.0.1:$PORT (not reachable from outside)
  dir      /srv/$NAME  (0750, owned by $NAME)
  secrets  /srv/$NAME/env
  memory   high $MEMHIGH / max $MEM
  routing  $( [[ -n "$HOST" ]] && echo "https://$HOST" || echo "http://<droplet-ip>/" )

It will not start until its binary is at /srv/$NAME/$BIN - deploy it.
EOF
