#!/usr/bin/env bash
# Installs a Nail bundle to /opt/nail. Everything the IDE needs - compiler,
# Rust toolchain, crate sources, warm build cache - is inside the bundle;
# no network and no other packages are required.
set -euo pipefail

TARBALL="${1:?usage: sudo ./install.sh nail-<version>-linux-x86_64.tar.xz}"

if [ "$(id -u)" -ne 0 ]; then
    echo "error: run with sudo (installs to /opt/nail)" >&2
    exit 1
fi

INSTALL_USER="${SUDO_USER:-root}"

rm -rf /opt/nail
tar -xJf "$TARBALL" -C /opt

# Builds run as the user: the shared cache and cargo-home (cargo takes a
# lock file there) must be writable by them. Everything else stays root-owned.
chown -R "$INSTALL_USER" /opt/nail/cache /opt/nail/cargo-home

ln -sf /opt/nail/bin/nail /usr/local/bin/nail
ln -sf /opt/nail/bin/nailc /usr/local/bin/nailc

echo "Nail installed. Run: nail"
