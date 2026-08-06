#!/bin/sh
# One-time bootstrap. This is the only step that ever needs root.
#
#   sudo ./install.sh                                  # fetch hammer, set up /opt/nail
#   sudo ./install.sh nail-<version>-linux-x86_64.tar.xz   # also install that release offline
#
# After this, hammer installs, removes and updates versions of Nail with no
# privileges at all, because /opt/nail belongs to the user who ran this.
#
# Only hammer goes on PATH. Installed versions deliberately do NOT: if one were
# on PATH it would shadow hammer, and a file's version line would stop
# deciding which compiler runs, which is the entire point.
# POSIX sh, deliberately. This is fetched with `curl | sudo sh`, and piping
# into an interpreter ignores the shebang, so on Debian and Ubuntu the script
# runs under dash. Nothing here may be a bashism, `set -o pipefail` included.
set -eu

ORIGIN="${NAIL_ORIGIN:-https://nail.alex-wilkinson.ca}"
ROOT=/opt/nail
# Piped into sh there is no script on disk, and $0 is just "sh", so the
# checkout probes below have to be skipped rather than resolved against
# whatever directory the user happened to be standing in.
if [ -f "$0" ]; then
	HERE="$(cd "$(dirname "$0")" && pwd)"
else
	HERE=""
fi

if [ "$(id -u)" -ne 0 ]; then
	echo "error: run with sudo (it creates $ROOT and links into /usr/local/bin)" >&2
	exit 1
fi

OWNER="${SUDO_USER:-root}"

cat <<'BANNER'

  ███╗   ██╗ █████╗ ██╗██╗
  ████╗  ██║██╔══██╗██║██║
  ██╔██╗ ██║███████║██║██║
  ██║╚██╗██║██╔══██║██║██║
  ██║ ╚████║██║  ██║██║███████╗
  ╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝╚══════╝

  🔨  Program it once. Run it forever.

BANNER

mkdir -p "$ROOT/bin" "$ROOT/versions"

# hammer comes from the release machine's build if this is a source checkout,
# and from the web otherwise.
if [ -n "$HERE" ] && [ -x "$HERE/../target/release/hammer" ]; then
	cp "$HERE/../target/release/hammer" "$ROOT/bin/hammer"
else
	echo "fetching hammer"
	curl -fL --proto '=https' "$ORIGIN/hammer/x86_64-linux" -o "$ROOT/bin/hammer"
fi
chmod 755 "$ROOT/bin/hammer"

# The user owns the store, so hammer never needs sudo again: not to install a
# version a file asks for, not to reclaim disk, not to update itself.
chown -R "$OWNER" "$ROOT"

# One binary, three names. argv[0] tells hammer which to be, so `nailc` in a
# Makefile and `#!/usr/bin/env nail` in a script both keep working while only
# one thing is on PATH. The symlinks are root-owned and never change again -
# `hammer self-update` rewrites the target, not these.
for name in hammer nail nailc; do
	ln -sf "$ROOT/bin/hammer" "/usr/local/bin/$name"
done

# Desktop integration: what makes a .nail file look like a Nail file. Without
# this the desktop treats them as unknown text, so they get no icon, no "Open
# with Nail", and double-clicking one does nothing.
#
# Installed from the repo when this runs from a checkout, and fetched otherwise,
# because the web installer is a bare script with nothing beside it.
DESKTOP_SRC="${HERE:+$HERE/desktop}"
if [ -z "$DESKTOP_SRC" ] || [ ! -f "$DESKTOP_SRC/nail.desktop" ]; then
	DESKTOP_SRC="$(mktemp -d)"
	for file in nail.desktop nail.xml nail.svg; do
		curl -fsSL --proto '=https' "$ORIGIN/desktop/$file" -o "$DESKTOP_SRC/$file" || true
	done
fi

if [ -f "$DESKTOP_SRC/nail.desktop" ]; then
	install -Dm644 "$DESKTOP_SRC/nail.desktop" /usr/share/applications/nail.desktop
	install -Dm644 "$DESKTOP_SRC/nail.xml" /usr/share/mime/packages/nail.xml
	install -Dm644 "$DESKTOP_SRC/nail.svg" /usr/share/icons/hicolor/scalable/apps/nail.svg

	# These rebuild the desktop's lookup tables. Without them the files are on
	# disk and nothing has read them, so nothing changes.
	command -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true
	command -v update-desktop-database >/dev/null && update-desktop-database /usr/share/applications || true
	command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true

	# Make it the default for .nail rather than merely one of the options.
	# This is the user's preference, so it goes in the user's config.
	if [ "$OWNER" != root ] && command -v xdg-mime >/dev/null; then
		sudo -u "$OWNER" xdg-mime default nail.desktop text/x-nail 2>/dev/null || true
	fi
	echo "registered .nail files with the desktop"
fi

# An offline install: unpack a release the user already has, rather than
# making hammer fetch one.
if [ -n "${1:-}" ]; then
	echo "installing $1"
	sudo -u "$OWNER" tar -xf "$1" -C "$ROOT/versions"
fi

# `nail` and `hammer` are the same binary under different names, which is
# convenient and confusing in equal measure, so say which one is for what
# rather than naming one and leaving the other a mystery.
cat <<DONE

Installed. Two names, one program:

  nail hello.nail       open a file in the editor
  nail                  open the editor with nothing in it

  hammer new hi.nail    start a file, ready to compile
  hammer run hi.nail    compile it and run it
  hammer list           which versions of Nail are on this machine
  hammer help           everything else

You will mostly type nail. hammer is for the versions underneath: it reads the
line at the top of a file, fetches exactly the Nail that wrote it if this
machine does not have it, and hands the file over. Double-clicking a .nail file
does the same thing.

Nothing is downloaded until you open a file, so the first one takes a while.
DONE
