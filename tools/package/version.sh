#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# version.sh — prints the native app's version, `[package].version` of
# crates/native/Cargo.toml, which every package and its file name carry.
#
# Usage: version.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="${root}/crates/native/Cargo.toml"

version="$(awk '
	/^\[/ { in_package = ($0 == "[package]") }
	in_package && /^version[[:space:]]*=/ {
		sub(/^version[[:space:]]*=[[:space:]]*"/, "")
		sub(/".*$/, "")
		print
		exit
	}
' "${manifest}")"

if [[ -z "${version}" ]]; then
	echo "version.sh: no [package] version in ${manifest}" >&2
	exit 1
fi
echo "${version}"
