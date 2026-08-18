#!/bin/sh
# One-time bootstrap.
#
#   curl -fsSL https://nail.alex-wilkinson.ca/install | sudo sh
#       Installs into /opt/nail and puts nail on PATH for every user.
#
#   ./install.sh nail-<version>-linux-x86_64.tar.xz
#       The same, plus installing that release from a file rather than
#       fetching one.
#
# /opt/nail is the only place a release can live, and that is what the sudo is
# for. A release ships with its dependencies already compiled, and cargo
# records absolute paths in the fingerprints that decide whether a compiled
# thing can be reused. Warmed at one path and read from another, every
# dependency recompiles the first time anybody builds anything, which is the
# one thing a prebuilt toolchain exists to prevent. One fixed path is what
# makes the shipped cache true on every machine.
#
# The store is handed to whoever ran this, so nail installs, removes and
# updates versions of itself afterwards with no privileges at all. The sudo is
# for this script and nothing after it.
#
# Only the launcher goes on PATH, as `nail`. Installed versions deliberately do
# NOT: one on PATH would shadow it, and a file's version line would stop
# deciding which compiler runs, which is the entire point.
# POSIX sh, deliberately. This is fetched with `curl | sh`, and piping into an
# interpreter ignores the shebang, so on Debian and Ubuntu the script runs
# under dash. Nothing here may be a bashism, `set -o pipefail` included.
set -eu

ORIGIN="${NAIL_ORIGIN:-https://nail.alex-wilkinson.ca}"

RELEASE=""
for argument in "$@"; do
	case "$argument" in
	-*)
		echo "error: unknown option $argument" >&2
		exit 1
		;;
	*) RELEASE="$argument" ;;
	esac
done

if [ "$(id -u)" -ne 0 ]; then
	echo "error: nail installs to /opt/nail, so this needs sudo:" >&2
	echo "         curl -fsSL https://nail.alex-wilkinson.ca/install | sudo sh" >&2
	echo "" >&2
	echo "       One fixed path is what lets a release ship its dependencies already" >&2
	echo "       compiled. Somewhere else, they would all compile again on this machine." >&2
	echo "       The store becomes yours, so nothing you do afterwards needs sudo." >&2
	exit 1
fi

# SUDO_USER, so the store belongs to the person who ran sudo rather than to
# root. Everything after this install is then theirs to do without it.
OWNER="${SUDO_USER:-root}"
ROOT=/opt/nail
BIN_DIR=/usr/local/bin
SHARE=/usr/share
MIME_DIR="$SHARE/mime"

# Piped into sh there is no script on disk, and $0 is just "sh", so the
# checkout probes below have to be skipped rather than resolved against
# whatever directory the user happened to be standing in.
if [ -f "$0" ]; then
	HERE="$(cd "$(dirname "$0")" && pwd)"
else
	HERE=""
fi

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

# Runs a command as the person who ran sudo, since this script is root and
# their preferences are not root's.
as_owner() {
	if [ "$OWNER" != root ]; then
		sudo -u "$OWNER" "$@"
	else
		"$@"
	fi
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

mkdir -p "$ROOT/bin" "$ROOT/versions" "$BIN_DIR"

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

# The store is handed to the person who ran sudo, so nail never needs it
# again: not to install a version a file asks for, not to reclaim disk, not to
# update itself.
chown -R "$OWNER" "$ROOT"

# Everyone on the machine builds against one warm cache, which means everyone
# who builds has to be able to write it: a build writes compiled artifacts into
# the store, there is no such thing as a read-only cargo cache. The `nail`
# group is who that is. Every human account joins now, and `nail share <user>`
# adds anyone made later. setgid keeps new files in the group, and the default
# ACL keeps them group-writable whatever umask the writer had.
groupadd -f nail
for account in $(awk -F: '$3 >= 1000 && $3 < 65000 && $7 !~ /(nologin|false)$/ {print $1}' /etc/passwd); do
	usermod -aG nail "$account" 2>/dev/null || true
done
chgrp -R nail "$ROOT"
chmod -R g+w "$ROOT"
find "$ROOT" -type d -exec chmod g+s {} +
command -v setfacl >/dev/null && setfacl -R -d -m g:nail:rwx "$ROOT" 2>/dev/null || true
step "shared with everyone on this machine, through the ${bold}nail$off group"

# One name. The symlink is root-owned and never changes again, because
# `nail self-update` rewrites what it points at rather than the link.
ln -sf "$ROOT/bin/nail" "$BIN_DIR/nail"

# Earlier builds put the launcher on PATH three times, as hammer, nail and
# nailc. One command is the whole interface now, so the other two go, and so
# does the binary they pointed at.
rm -f /usr/local/bin/hammer /usr/local/bin/nailc "$ROOT/bin/hammer"
step "linked ${bold}$BIN_DIR/nail$off"

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
	install -Dm644 "$DESKTOP_SRC/nail.desktop" "$SHARE/applications/nail.desktop"
	install -Dm644 "$DESKTOP_SRC/nail.xml" "$MIME_DIR/packages/nail.xml"
	install -Dm644 "$DESKTOP_SRC/nail.svg" "$SHARE/icons/hicolor/scalable/apps/nail.svg"

	# These rebuild the desktop's lookup tables. Without them the files are on
	# disk and nothing has read them, so nothing changes.
	command -v update-mime-database >/dev/null && update-mime-database "$MIME_DIR" || true
	command -v update-desktop-database >/dev/null && update-desktop-database "$SHARE/applications" || true
	command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -f -t "$SHARE/icons/hicolor" 2>/dev/null || true

	# Make it the default for .nail rather than merely one of the options.
	# This is the user's preference, so it goes in the user's config.
	if command -v xdg-mime >/dev/null; then
		as_owner xdg-mime default nail.desktop text/x-nail 2>/dev/null || true
	fi
	step ".nail files open with nail, and have an icon"
fi

# An offline install: unpack a release the user already has, rather than making
# nail fetch one. `nail import` rather than tar, because it checks what it is
# unpacking and puts it where the store expects it.
if [ -n "$RELEASE" ]; then
	echo "installing $RELEASE"
	as_owner "$ROOT/bin/nail" import "$RELEASE"
fi

# /usr/local/bin is on every default PATH worth naming, but a stripped-down
# container or a hand-written profile can leave it off, and silence there looks
# like a failed install.
PATH_NOTE=''
case ":$PATH:" in
*":$BIN_DIR:"*) ;;
*) PATH_NOTE=yes ;;
esac

printf '\n  %sEverything is one command%s\n\n' "$bold" "$off"
printf '    %snail new hello%s      create a new file, ready to compile\n' "$amber" "$off"
printf '    %snail hello%s          open it in the editor\n' "$amber" "$off"
printf '    %snail run hello%s      compile it and run it\n' "$amber" "$off"
printf '    %snail test%s           run every Nail file in tests/\n' "$amber" "$off"
printf '    %snail docs%s <name>    what the library says about a function\n' "$amber" "$off"
printf '    %snail help%s           everything else\n' "$amber" "$off"
printf '\n  %sThe .nail extension is optional: "nail new hello" and\n' "$dim"
printf '  "nail new hello.nail" do the same thing.%s\n' "$off"

if [ -n "$PATH_NOTE" ]; then
	printf '\n  %s%s is not on this shell'"'"'s PATH.%s To use nail in this one:\n\n' "$bold" "$BIN_DIR" "$off"
	printf '    %sexport PATH="%s:$PATH"%s\n' "$amber" "$BIN_DIR" "$off"
	printf '\n  %sIf your shell never picks it up, add that line to its startup file.%s\n' "$dim" "$off"
fi

printf '\n  Every Nail file records the version that wrote it. %snail%s reads that line,\n' "$amber" "$off"
printf '  and fetches that exact version if this machine does not have it, so a\n'
printf '  program that compiled once compiles forever. Double-clicking a .nail\n'
printf '  file does the same.\n'
printf '\n  %sWhat is on this machine so far is the launcher and nothing else. The first\n' "$dim"
printf '  time you open a Nail file it downloads the version that file names, which is\n'
printf '  over a gigabyte and happens once for that version. After that it is offline,\n'
printf '  and programs compile in seconds, because what came down has every library\n'
printf '  already built.%s\n\n' "$off"
