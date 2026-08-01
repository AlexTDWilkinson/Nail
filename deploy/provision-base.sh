#!/usr/bin/env bash
# BOX-LEVEL setup. Run ONCE per droplet, as root, no matter how many apps will
# live on it. Nothing here is SUL-specific - it installs the shared pieces that
# every app on the box uses:
#
#   - Caddy, the single public listener on :80/:443
#   - /etc/caddy/sites.d/, where each app drops its own routing fragment
#   - ufw: only 22/80/443 reachable from the internet
#   - fail2ban, swap
#
#   scp -r deploy root@<ip>:/tmp/deploy
#   ssh root@<ip> 'bash /tmp/deploy/provision-base.sh'
#
# Per-app setup is a separate script: add-app.sh.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
	echo "run as root" >&2
	exit 1
fi

echo "== packages =="
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq debian-keyring debian-archive-keyring apt-transport-https \
	curl gnupg ufw rsync fail2ban

echo "== swap =="
# Small droplets ship with none, which turns any memory spike into an OOM kill.
if ! swapon --show | grep -q .; then
	fallocate -l 1G /swapfile
	chmod 600 /swapfile
	mkswap /swapfile >/dev/null
	swapon /swapfile
	grep -q '^/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' >>/etc/fstab
	sysctl -qw vm.swappiness=10
	grep -q '^vm.swappiness' /etc/sysctl.conf || echo 'vm.swappiness=10' >>/etc/sysctl.conf
fi

echo "== fail2ban =="
cat >/etc/fail2ban/jail.local <<'EOF'
[sshd]
enabled = true
maxretry = 5
findtime = 10m
bantime = 15m
EOF
systemctl enable --now fail2ban >/dev/null
systemctl restart fail2ban

echo "== caddy =="
if ! command -v caddy >/dev/null; then
	curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
		| gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
	curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
		| tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
	apt-get update -qq
	apt-get install -y -qq caddy
fi

echo "== caddy site fragments =="
# This script is copied into every app repo, so several copies may run against
# the same box over time. Everything here is therefore idempotent and
# non-destructive: the shared Caddyfile is written ONLY if it is not already
# set up, so a second repo's copy can never clobber the first's.
mkdir -p /etc/caddy/sites.d
if ! grep -qs 'import /etc/caddy/sites.d' /etc/caddy/Caddyfile; then
	cat >/etc/caddy/Caddyfile <<'EOF'
# Box-level Caddy config. Do not add sites here - each app installs its own
# fragment in /etc/caddy/sites.d/<app>.caddy via its add-app.sh.
#
# The admin API is deliberately left enabled: it binds 127.0.0.1:2019 only
# (unreachable from outside, and ufw blocks it regardless), and `systemctl
# reload caddy` goes through it. Turning it off makes every reload fail.

import /etc/caddy/sites.d/*.caddy
EOF
else
	echo "   already configured, left as-is"
fi

# An empty sites.d makes the import match nothing, which Caddy rejects at boot.
# A harmless placeholder keeps the config valid until the first app lands.
if ! compgen -G '/etc/caddy/sites.d/*.caddy' >/dev/null; then
	cat >/etc/caddy/sites.d/00-placeholder.caddy <<'EOF'
# Replaced in effect by the first real app; harmless to leave in place.
:80 {
	respond "no site configured for this hostname yet" 404
}
EOF
fi

caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null
systemctl enable caddy >/dev/null
systemctl restart caddy

echo "== firewall =="
ufw allow OpenSSH
ufw allow 80/tcp
ufw allow 443/tcp
ufw --force enable

cat <<'EOF'

Box ready. Nothing app-specific is installed yet.

For each app, from that app's own repo:
  ssh root@<ip> 'bash /tmp/<app>/add-app.sh --name <app> --port <port> --bin <binary>'

Apps bind 127.0.0.1 only, so the proxy is the sole public entrance.
EOF
