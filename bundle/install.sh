#!/bin/sh
# One-time bootstrap. This is the only step that ever needs root.
#
#   sudo ./install.sh                                  # fetch nail, set up /opt/nail
#   sudo ./install.sh nail-<version>-linux-x86_64.tar.xz   # also install that release offline
#
# After this, nail installs, removes and updates versions of itself with no
# privileges at all, because /opt/nail belongs to the user who ran this.
#
# Only the launcher goes on PATH, as `nail`. Installed versions deliberately do
# NOT: one on PATH would shadow it, and a file's version line would stop
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

# Named ANSI colours rather than hex, so they follow whatever palette the
# terminal already uses and look right in a light theme as well as a dark one.
# Nothing is coloured when stdout is not a terminal, so piping this stays clean,
# and NO_COLOR is honoured because people who set it mean it.
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
	bold=$(printf '\033[1m')
	dim=$(printf '\033[2m')
	amber=$(printf '\033[1;33m')
	green=$(printf '\033[1;32m')
	off=$(printf '\033[0m')
	# 24-bit colour is what the rainbow needs. Terminals that have it say so.
	case "${COLORTERM:-}" in
	truecolor | 24bit) truecolor=yes ;;
	*) truecolor='' ;;
	esac
else
	bold=''
	dim=''
	amber=''
	green=''
	off=''
	truecolor=''
fi

step() {
	printf '  %s✓%s %s\n' "$green" "$off" "$1"
}

# The banner sweeps through a rainbow when the terminal can do 24-bit colour,
# and sits still in amber when it cannot. Redrawn in place with a cursor-up, so
# it animates without scrolling the screen. About a second, once, on the one
# occasion a person installs this.
banner() {
	if [ -n "$truecolor" ]; then
		awk 'BEGIN {
			art[1] = "  ███╗   ██╗ █████╗ ██╗██╗";
			art[2] = "  ████╗  ██║██╔══██╗██║██║";
			art[3] = "  ██╔██╗ ██║███████║██║██║";
			art[4] = "  ██║╚██╗██║██╔══██║██║██║";
			art[5] = "  ██║ ╚████║██║  ██║██║███████╗";
			art[6] = "  ╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝╚══════╝";
			frames = 28;
			for (f = 0; f < frames; f++) {
				for (i = 1; i <= 6; i++) {
					t = (i + f) * 0.45;
					r = int(sin(t) * 110 + 140);
					g = int(sin(t + 2.09) * 110 + 140);
					b = int(sin(t + 4.19) * 110 + 140);
					printf "\033[38;2;%d;%d;%dm%s\033[0m\n", r, g, b, art[i];
				}
				if (f < frames - 1) {
					printf "\033[6A";
					system("sleep 0.045");
				}
			}
		}'
	else
		printf '%s' "$amber"
		cat <<'BANNER'
  ███╗   ██╗ █████╗ ██╗██╗
  ████╗  ██║██╔══██╗██║██║
  ██╔██╗ ██║███████║██║██║
  ██║╚██╗██║██╔══██║██║██║
  ██║ ╚████║██║  ██║██║███████╗
  ╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝╚══════╝
BANNER
		printf '%s' "$off"
	fi
}

printf '\n'
banner
printf '  🔨  %sProgram it once. Run it forever.%s\n\n' "$dim" "$off"

mkdir -p "$ROOT/bin" "$ROOT/versions"

# The launcher comes from the release machine's build if this is a source
# checkout, and from the web otherwise.
if [ -n "$HERE" ] && [ -x "$HERE/../target/release/nail-launcher" ]; then
	cp "$HERE/../target/release/nail-launcher" "$ROOT/bin/nail"
else
	printf '  %sdownloading%s\n' "$dim" "$off"
	curl -fsSL --proto '=https' "$ORIGIN/nail/x86_64-linux" -o "$ROOT/bin/nail"
fi
chmod 755 "$ROOT/bin/nail"
step "nail installed to ${bold}$ROOT/bin/nail$off"

# The user owns the store, so nail never needs sudo again: not to install a
# version a file asks for, not to reclaim disk, not to update itself.
chown -R "$OWNER" "$ROOT"

# One name. The symlink is root-owned and never changes again, because
# `nail self-update` rewrites what it points at rather than the link.
ln -sf "$ROOT/bin/nail" /usr/local/bin/nail

# Earlier builds put the launcher on PATH three times, as hammer, nail and
# nailc. One command is the whole interface now, so the other two go, and so
# does the binary they pointed at.
rm -f /usr/local/bin/hammer /usr/local/bin/nailc "$ROOT/bin/hammer"
step "linked ${bold}/usr/local/bin/nail$off"

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
	step ".nail files open with nail, and have an icon"
fi

# An offline install: unpack a release the user already has, rather than
# making nail fetch one.
if [ -n "${1:-}" ]; then
	echo "installing $1"
	sudo -u "$OWNER" tar -xf "$1" -C "$ROOT/versions"
fi

printf '\n  %sEverything is one command%s\n\n' "$bold" "$off"
printf '    %snail new hello%s      create a new file, ready to compile\n' "$amber" "$off"
printf '    %snail hello%s          open it in the editor\n' "$amber" "$off"
printf '    %snail run hello%s      compile it and run it\n' "$amber" "$off"
printf '    %snail test%s           run every Nail file in tests/\n' "$amber" "$off"
printf '    %snail docs%s <name>    what the library says about a function\n' "$amber" "$off"
printf '    %snail help%s           everything else\n' "$amber" "$off"
printf '\n  %sThe .nail extension is optional: "nail new hello" and\n' "$dim"
printf '  "nail new hello.nail" do the same thing.%s\n' "$off"
printf '\n  Every Nail file records the version that wrote it. %snail%s reads that line,\n' "$amber" "$off"
printf '  and fetches that exact version if this machine does not have it, so a\n'
printf '  program that compiled once compiles forever. Double-clicking a .nail\n'
printf '  file does the same.\n'
printf '\n  %sNothing is downloaded until you open a file, so the first one takes a\n' "$dim"
printf '  while.%s\n\n' "$off"
