#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# set-version.sh — writes the given version to `[package].version` of
# crates/app/Cargo.toml and to the crate's entry in crates/app/Cargo.lock,
# so `cargo build --release --locked` still passes and the binary carries it.
# The version is refused as version.sh refuses one.
#
# Usage: set-version.sh <version>

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="${1:?usage: set-version.sh <version>}"
manifest="${root}/crates/app/Cargo.toml"
lock="${root}/crates/app/Cargo.lock"

# The crate's name, whose Cargo.lock entry carries the version.
crate="handpic-app"

bash "${root}/tools/package/version.sh" semver "${version}" >/dev/null

# Rewrites the `version = "..."` line of the one section that opens with the
# given line (Cargo.toml's `[package]`) or, given a name, of the
# `[[package]]` entry with that name (Cargo.lock's), and fails unless it
# rewrote exactly one.
function rewrite() {
	local file="$1" header="$2" name="${3:-}" temp
	temp="$(mktemp)"
	awk -v version="${version}" -v header="${header}" -v name="${name}" '
		/^\[/ { selected = (name == "" && $0 == header) }
		name != "" && $0 == "name = \"" name "\"" { selected = 1 }
		selected && /^version[[:space:]]*=/ {
			print "version = \"" version "\""
			count++
			next
		}
		{ print }
		END { exit count != 1 }
	' "${file}" >"${temp}" || {
		rm -f "${temp}"
		echo "set-version.sh: not one version to set in ${file}" >&2
		exit 1
	}
	cat "${temp}" >"${file}"
	rm -f "${temp}"
}

rewrite "${manifest}" "[package]"
rewrite "${lock}" "" "${crate}"

written="$(bash "${root}/tools/package/version.sh")"
if [[ "${written}" != "${version}" ]]; then
	echo "set-version.sh: ${manifest} reads ${written}, not ${version}" >&2
	exit 1
fi
echo "${version}"
