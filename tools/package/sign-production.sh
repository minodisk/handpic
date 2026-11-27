#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# sign-production.sh — signs the given updater archives in place with the
# production key, which it reads base64-encoded from the UPDATE_SIGNING_KEY
# environment variable (the `release` environment's secret of that name),
# through `mise run app:sign` against the checked-in
# crates/app/packaging/update.pub.
# The key is decoded into a private temporary folder, removed on exit, and
# never printed. It fails, naming the secret, when the value is empty, does
# not decode to a 64-byte key, or does not match update.pub (sign.sh's own
# `zipsign verify`). release.yml's guard runs it on a scratch archive before
# any build, and each package.yml leg on its archives. Run it from the
# repository root.
#
# Usage: UPDATE_SIGNING_KEY=<base64> sign-production.sh <archive>...

if [[ $# -lt 1 ]]; then
	echo "usage: UPDATE_SIGNING_KEY=<base64> sign-production.sh <archive>..." >&2
	exit 1
fi
if [[ -z "${UPDATE_SIGNING_KEY:-}" ]]; then
	echo "sign-production.sh: the UPDATE_SIGNING_KEY secret is empty or missing" >&2
	exit 1
fi

umask 077
dir="$(mktemp -d)"
trap 'rm -rf "${dir}"' EXIT
key="${dir}/update.key"

if ! printf '%s' "${UPDATE_SIGNING_KEY}" | base64 -d >"${key}" 2>/dev/null; then
	echo "sign-production.sh: the UPDATE_SIGNING_KEY secret is not base64" >&2
	exit 1
fi
size="$(wc -c <"${key}" | tr -d ' ')"
if [[ "${size}" != 64 ]]; then
	echo "sign-production.sh: the UPDATE_SIGNING_KEY secret decodes to ${size} bytes, not a 64-byte key" >&2
	exit 1
fi

if ! mise run app:sign -- "${key}" crates/app/packaging/update.pub "$@"; then
	echo "sign-production.sh: signing with the UPDATE_SIGNING_KEY secret failed; does it match crates/app/packaging/update.pub?" >&2
	exit 1
fi
