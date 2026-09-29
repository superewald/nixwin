#!/usr/bin/env bash
# nixwin installer
#
# Downloads the latest published nixwin release for this host and installs the
# binary into a directory the invoking user can write to. The script never
# escalates privileges: it writes only inside the invoking user's home
# directory, installs no packages, and edits no shell configuration.
#
# Requirements: bash, curl or wget, and a POSIX environment (uname, sed, awk).

set -euo pipefail

REPO="superewald/nixwin"
API="https://api.github.com/repos/${REPO}"
USER_AGENT="nixwin-install"

BIN_DIR=""
VERSION=""
DRY_RUN=0

usage() {
	cat <<'EOF'
Install nixwin from a published GitHub release.

Usage:
  curl -fsSL https://raw.githubusercontent.com/superewald/nixwin/main/install.sh | bash
  install.sh [options]

Options:
  --bin-dir <dir>   Install into <dir> instead of $HOME/.local/bin.
  --version <tag>   Install a specific release tag instead of the latest one.
  --dry-run         Print what would happen, without downloading or writing.
  -h, --help        Show this help.

Environment:
  NIXWIN_INSTALL_BIN_DIR   Same as --bin-dir.

The install directory must be inside your home directory. The script does not
use sudo, does not touch system paths, and does not modify your shell rc; add
the install directory to PATH yourself if it is not already there.
EOF
}

die() {
	printf 'install.sh: %s\n' "$1" >&2
	exit 1
}

info() {
	printf '%s\n' "$1"
}

step() {
	printf '==> %s\n' "$1"
}

while [ $# -gt 0 ]; do
	case "$1" in
	--bin-dir)
		[ $# -ge 2 ] || die "--bin-dir requires a directory"
		BIN_DIR="$2"
		shift 2
		;;
	--version)
		[ $# -ge 2 ] || die "--version requires a release tag"
		VERSION="$2"
		shift 2
		;;
	--dry-run)
		DRY_RUN=1
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		usage >&2
		die "unknown argument: $1"
		;;
	esac
done

[ -n "$BIN_DIR" ] || BIN_DIR="${NIXWIN_INSTALL_BIN_DIR:-}"
[ -n "$BIN_DIR" ] || BIN_DIR="$HOME/.local/bin"

# --- host detection -------------------------------------------------------

detect_os() {
	case "$(uname -s)" in
	Linux) printf 'linux' ;;
	Darwin) printf 'darwin' ;;
	*) die "unsupported operating system: $(uname -s). nixwin builds target Linux and macOS hosts." ;;
	esac
}

detect_arch() {
	case "$(uname -m)" in
	x86_64 | amd64) printf 'x86_64' ;;
	aarch64 | arm64) printf 'aarch64' ;;
	i386 | i486 | i586 | i686) printf 'x86' ;;
	armv7l | armv6l) printf 'aarch' ;;
	*) die "unsupported CPU architecture: $(uname -m)" ;;
	esac
}

HOST_OS="$(detect_os)"
HOST_ARCH="$(detect_arch)"

# --- install directory ----------------------------------------------------

# Print an absolute, symlink-resolved form of a path that need not exist yet.
# realpath(1) is not portable, so canonicalize the deepest existing ancestor by
# changing into it and re-append the components that do not exist.
resolve_path() {
	target="$1"
	suffix=""

	case "$target" in
	/*) ;;
	*) target="$PWD/$target" ;;
	esac

	while [ ! -d "$target" ] && [ "$target" != "/" ]; do
		suffix="$(basename -- "$target")/$suffix"
		target="${target%/*}"
		[ -n "$target" ] || target="/"
	done

	# [ -d ] follows symlinks, so this stops at a symlink to a directory, and
	# pwd -P below resolves it along with every other component.
	if [ -d "$target" ]; then
		base="$(cd -- "$target" 2>/dev/null && pwd -P)" || base="$target"
	else
		base="$target"
	fi

	if [ -z "$suffix" ]; then
		printf '%s' "$base"
	elif [ "$base" = "/" ]; then
		printf '/%s' "${suffix%/}"
	else
		printf '%s/%s' "$base" "${suffix%/}"
	fi
}

# Fail unless the given path is the home directory or below it.
assert_under_home() {
	home="$(resolve_path "$HOME")"
	case "$1" in
	"$home" | "$home"/*) ;;
	*) die "refusing to install into $1: it is outside your home directory ($home)" ;;
	esac
}

# The script promises to stay inside the invoking user's home directory, so
# refuse a target that would escape it.
check_bin_dir() {
	dir="$1"

	# Expand a leading ~ without requiring the directory to exist.
	# shellcheck disable=SC2088  # the tilde is a literal prefix to strip, not an unexpanded glob
	case "$dir" in
	"~") dir="$HOME" ;;
	"~/"*) dir="$HOME/${dir#"~/"}" ;;
	esac

	[ -n "$dir" ] || die "empty install directory"

	resolved="$(resolve_path "$dir")"

	# Guard against a home directory of "/" disabling the check entirely.
	[ "$(resolve_path "$HOME")" != "/" ] ||
		die "HOME is '/', refusing to install without a resolvable home directory"
	[ -d "$HOME" ] || die "HOME ($HOME) is not an existing directory"

	assert_under_home "$resolved"

	printf '%s' "$resolved"
}

BIN_DIR="$(check_bin_dir "$BIN_DIR")"

# --- release lookup -------------------------------------------------------

http_get() {
	url="$1"
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL -H "User-Agent: ${USER_AGENT}" -H "Accept: application/vnd.github+json" "$url"
	elif command -v wget >/dev/null 2>&1; then
		wget -q -O - --user-agent="${USER_AGENT}" --header="Accept: application/vnd.github+json" "$url"
	else
		die "neither curl nor wget was found; install one of them and re-run"
	fi
}

if [ -n "$VERSION" ]; then
	RELEASE_URL="${API}/releases/tags/${VERSION}"
else
	RELEASE_URL="${API}/releases/latest"
fi

step "looking up ${REPO} release${VERSION:+ $VERSION}"
RELEASE_JSON="$(http_get "$RELEASE_URL")" || {
	if [ -n "$VERSION" ]; then
		die "could not read release ${VERSION} from ${RELEASE_URL}. check the tag name, or build from source: https://github.com/${REPO}"
	fi
	die "could not read the release from ${RELEASE_URL}. If no release exists yet, build from source: https://github.com/${REPO}"
}

TAG="$(printf '%s' "$RELEASE_JSON" |
	sed -n 's/.*"tag_name":"\([^"]*\)".*/\1/p' | head -n1)"
[ -n "$TAG" ] || die "the release metadata did not contain a tag name"

# The release payload is JSON with a flat "assets" array. Trim everything up to
# the array, then read one line per asset object and pull out its name and
# download URL. The `|| [ -n "$asset" ]` is load-bearing: the API sends no
# trailing newline, so the last asset would otherwise never reach the loop body.
ASSET_TABLE="$(
	printf '%s' "${RELEASE_JSON#*\"assets\":[}" |
		tr '{' '\n' |
		tail -n +2 |
		while IFS= read -r asset || [ -n "$asset" ]; do
			name="$(printf '%s' "$asset" | sed -n 's/.*"name":"\([^"]*\)".*/\1/p' | head -n1)"
			url="$(printf '%s' "$asset" | sed -n 's/.*"browser_download_url":"\([^"]*\)".*/\1/p' | head -n1)"
			if [ -n "$name" ] && [ -n "$url" ]; then
				printf '%s\t%s\n' "$name" "$url"
			fi
		done
)"

if [ -z "$ASSET_TABLE" ]; then
	die "release ${TAG} publishes no downloadable assets"
fi

# Release asset names are nixwin-<arch>[-<libc>], optionally qualified with the
# operating system. nixwin publishes arch-only names for Linux, so those are
# trusted there. Other hosts only accept an OS-qualified name: an arch-only
# match on macOS would silently install a Linux binary, and this script must not
# report success for a platform it cannot verify.
pick_asset() {
	patterns=(
		"^nixwin-${HOST_OS}-${HOST_ARCH}-musl$"
		"^nixwin-${HOST_OS}-${HOST_ARCH}-gnu$"
		"^nixwin-${HOST_OS}-${HOST_ARCH}$"
	)
	if [ "$HOST_OS" = "linux" ]; then
		patterns+=(
			"^nixwin-${HOST_ARCH}-musl$"
			"^nixwin-${HOST_ARCH}-gnu$"
			"^nixwin-${HOST_ARCH}$"
		)
	fi
	for pattern in "${patterns[@]}"; do
		match="$(printf '%s\n' "$ASSET_TABLE" | awk -F'\t' -v p="$pattern" '$1 ~ p { print; exit }')"
		if [ -n "$match" ]; then
			printf '%s' "$match"
			return 0
		fi
	done
	return 1
}

ASSET="$(pick_asset || true)"

if [ -z "$ASSET" ]; then
	info "release ${TAG} has no asset for ${HOST_OS}/${HOST_ARCH}." >&2
	info "available assets:" >&2
	printf '%s\n' "$ASSET_TABLE" | awk -F'\t' '{ printf "  - %s\n", $1 }' >&2
	die "build from source instead: https://github.com/${REPO}#building"
fi

ASSET_NAME="$(printf '%s' "$ASSET" | cut -f1)"
ASSET_URL="$(printf '%s' "$ASSET" | cut -f2)"
TARGET="${BIN_DIR}/nixwin"

info "release:  ${TAG}"
info "asset:    ${ASSET_NAME}"
info "platform: ${HOST_OS}/${HOST_ARCH}"
info "target:   ${TARGET}"

if [ "$DRY_RUN" -eq 1 ]; then
	info "dry run, nothing was downloaded or written"
	exit 0
fi

# --- download and install -------------------------------------------------

# Keep the download inside the home directory so a failed or interrupted run
# never leaves temporary files in a shared location such as /tmp.
TMP_DIR="$(mktemp -d "${HOME}/.nixwin-install.XXXXXX")"
cleanup() {
	rm -rf -- "$TMP_DIR"
}
trap cleanup EXIT

ARCHIVE="${TMP_DIR}/${ASSET_NAME}"

step "downloading ${ASSET_URL}"
if ! http_get "$ASSET_URL" >"$ARCHIVE"; then
	die "download failed: ${ASSET_URL}"
fi

step "installing ${TARGET}"
mkdir -p -- "$BIN_DIR"

# A symlink inside the home directory can still redirect the write somewhere
# else, and a dangling one is invisible to the pre-flight check. The directory
# now exists, so its real location is knowable: re-check it before writing.
assert_under_home "$(resolve_path "$BIN_DIR")"

if [ -e "$TARGET" ]; then
	backup="${TARGET}.bak"
	info "backing up the existing binary to ${backup}"
	mv -- "$TARGET" "$backup"
fi

# Releases ship a raw binary. If a future release ships a tarball instead,
# extract the binary rather than writing an archive to the install path.
case "$ASSET_NAME" in
*.tar.gz | *.tgz | *.tar.xz | *.tar.bz2 | *.tar)
	tar -xf "$ARCHIVE" -C "$TMP_DIR"
	binary="$(find "$TMP_DIR" -type f -name nixwin | head -n1)"
	[ -n "$binary" ] || die "no nixwin binary inside ${ASSET_NAME}"
	mv -- "$binary" "$TARGET"
	;;
*) mv -- "$ARCHIVE" "$TARGET" ;;
esac

chmod 0755 -- "$TARGET"

step "installed nixwin ${TAG} to ${TARGET}"
"${TARGET}" --version 2>/dev/null || true

case ":${PATH}:" in
*":${BIN_DIR}:"*) ;;
*) info ""; info "note: ${BIN_DIR} is not on your PATH. Add it with:"; info "  export PATH=\"${BIN_DIR}:\$PATH\"" ;;
esac
