#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# sign.sh — signs the archives the native app's updater installs, in place,
# with zipsign and a private key file: a `.zip` (the portable ZIP, the setup
# ZIP) gets its signature before its data, a `.tar.gz` (the Linux and the
# macOS tarballs) as a gzip member after it. zipsign binds the signature to
# the file's name, so a signed file is never renamed, and SHA256SUMS is
# computed after signing. Then verifies each file with the given verifying
# key, so a private key that does not match it fails here. Prints the paths.
# Needs zipsign 0.2.2 on PATH; `mise run app:sign` provides it.
#
# Usage: sign.sh <private-key> <verifying-key> <archive>...

# The zipsign release every signature is made with (mise.toml's tasks pin the
# same).
zipsign_version="0.2.2"

if [[ $# -lt 3 ]]; then
	echo "usage: sign.sh <private-key> <verifying-key> <archive>..." >&2
	exit 1
fi
key="$1"
verifying_key="$2"
shift 2

if [[ "$(zipsign --version)" != "zipsign ${zipsign_version}" ]]; then
	echo "sign.sh: needs zipsign ${zipsign_version}, found $(zipsign --version)" >&2
	exit 1
fi

for archive in "$@"; do
	case "${archive}" in
	*.zip) kind=zip ;;
	*.tar.gz) kind=tar ;;
	*)
		echo "sign.sh: ${archive} is neither a .zip nor a .tar.gz" >&2
		exit 1
		;;
	esac
	zipsign sign "${kind}" "${archive}" "${key}" >&2
	zipsign verify "${kind}" "${archive}" "${verifying_key}" >&2
	echo "${archive}"
done
