#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# version.sh — prints the native app's version, `[package].version` of
# crates/native/Cargo.toml (or the version given), in the form a package
# format takes:
#
#   semver   2.0.0     2.0.0-rc.1   every file name, Inno AppVersion, the binary
#   numeric  2.0.0     2.0.0        CFBundleShortVersionString, CFBundleVersion
#   windows  2.0.0.0   2.0.0.0      Inno VersionInfoVersion
#   deb      2.0.0     2.0.0~rc.1   the deb control file's Version
#
# It refuses a version that is not x.y.z or x.y.z-<prerelease>, the
# prerelease being dot-separated alphanumeric identifiers (no `-`, which the
# deb form could not carry).
#
# Usage: version.sh [semver|numeric|windows|deb] [<version>]

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="${root}/crates/native/Cargo.toml"
form="${1:-semver}"

if [[ $# -ge 2 ]]; then
	version="$2"
else
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
fi

number='(0|[1-9][0-9]*)'
if [[ ! "${version}" =~ ^${number}\.${number}\.${number}(-([0-9A-Za-z]+(\.[0-9A-Za-z]+)*))?$ ]]; then
	echo "version.sh: ${version} is not x.y.z or x.y.z-<prerelease>" >&2
	exit 1
fi
core="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.${BASH_REMATCH[3]}"
prerelease="${BASH_REMATCH[5]}"

case "${form}" in
semver) echo "${version}" ;;
numeric) echo "${core}" ;;
windows) echo "${core}.0" ;;
deb) echo "${core}${prerelease:+~${prerelease}}" ;;
*)
	echo "version.sh: unknown form ${form}; use semver, numeric, windows or deb" >&2
	exit 1
	;;
esac
