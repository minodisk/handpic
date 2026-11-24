#!/bin/sh
# install.sh — installs the latest release of Handpic for the current user,
# where its updater expects it: /Applications/Handpic.app on macOS (arm64,
# x86_64), ~/.local/bin/handpic on Linux (x86_64) through the release's own
# install.sh asset. Every download is checked against the release's
# SHA256SUMS before anything is installed. Needs curl, and Python 3 on Linux;
# not root.
#
# Usage: curl -fsSL https://raw.githubusercontent.com/minodisk/handpic/main/install.sh | sh

set -eu

repo="minodisk/handpic"
tmp=""
staged=""

fail() {
	echo "install.sh: $*" >&2
	exit 1
}

need() {
	command -v "$1" >/dev/null 2>&1 || fail "$1 is required but was not found"
}

cleanup() {
	if [ -n "${staged}" ]; then rm -rf "${staged}"; fi
	if [ -n "${tmp}" ]; then rm -rf "${tmp}"; fi
}

# download <asset name>
download() {
	echo "Downloading $1"
	curl -fsSL --proto '=https' -o "${tmp}/$1" "https://github.com/${repo}/releases/download/${tag}/$1" ||
		fail "could not download $1"
}

# verify <asset name>: prints its digest after checking it against SHA256SUMS.
verify() {
	expected="$(awk -v name="$1" '$2 == name { print $1 }' "${tmp}/SHA256SUMS")"
	[ -n "${expected}" ] || fail "$1 is not listed in SHA256SUMS"
	actual="$(${sha256} "${tmp}/$1" | awk '{ print $1 }')"
	[ "${actual}" = "${expected}" ] || fail "checksum mismatch for $1; nothing installed"
	echo "${expected}"
}

install_macos() {
	mkdir "${tmp}/extracted"
	tar -xzf "${tmp}/${tarball}" -C "${tmp}/extracted"
	app="${tmp}/extracted/Handpic.app"
	[ -d "${app}" ] || fail "${tarball} holds no Handpic.app"
	codesign --verify --strict "${app}" || fail "Handpic.app in ${tarball} fails codesign --verify"
	[ -w /Applications ] || fail "/Applications is not writable by $(id -un); nothing installed"
	staged="/Applications/.Handpic.app.install"
	rm -rf "${staged}"
	mv "${app}" "${staged}"
	rm -rf /Applications/Handpic.app
	mv "${staged}" /Applications/Handpic.app
	staged=""
	echo "Installed Handpic ${version}: /Applications/Handpic.app"
}

install_linux() {
	download install.sh
	verify install.sh >/dev/null
	sh "${tmp}/install.sh" "${tmp}/${tarball}" "${digest}"
}

main() {
	os="$(uname -s)"
	arch="$(uname -m)"
	# Under Rosetta uname reports x86_64 on an Apple Silicon Mac; install the native build.
	if [ "${os}" = Darwin ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null)" = 1 ]; then
		arch=arm64
	fi
	case "${os} ${arch}" in
	"Darwin arm64" | "Darwin x86_64")
		tarball_suffix="macos-${arch}"
		sha256="shasum -a 256"
		need shasum
		need codesign
		need tar
		;;
	"Linux x86_64")
		tarball_suffix="linux-x64"
		sha256="sha256sum"
		need sha256sum
		need python3
		;;
	*) fail "unsupported platform: ${os} ${arch} (Handpic supports macOS arm64 / x86_64 and Linux x86_64)" ;;
	esac
	need curl
	need awk

	latest="$(curl -fsSL --proto '=https' -o /dev/null -w '%{url_effective}' "https://github.com/${repo}/releases/latest")" ||
		fail "could not reach https://github.com/${repo}/releases/latest"
	tag="${latest##*/}"
	case "${tag}" in
	v[0-9]*) ;;
	*) fail "could not resolve the latest release from ${latest}" ;;
	esac
	version="${tag#v}"
	tarball="Handpic-${version}-${tarball_suffix}.tar.gz"

	tmp="$(mktemp -d)"
	trap cleanup EXIT
	trap 'exit 1' HUP INT TERM

	download SHA256SUMS
	download "${tarball}"
	digest="$(verify "${tarball}")"
	case "${os}" in
	Darwin) install_macos ;;
	Linux) install_linux ;;
	esac
}

main "$@"
