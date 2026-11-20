#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# test-release.sh — publishes the artifacts of a `Package` run as the release
# `v<version>` of a scratch repository, the test feed a build packaged with
# `update_test_repo=<owner/name>` updates from. The version is read from the
# artifacts' names; a version with a prerelease part (`2.0.0-rc.1`) is
# published as a GitHub prerelease. The files are uploaded as the run made
# them, never renamed (zipsign binds each signature to the file's name), after
# `sha256sum --check` against the run's SHA256SUMS. It refuses
# minodisk/handpic: a test build trusts the checked-in test key, and its
# releases never belong on the real feed. Needs `sha256sum` and `gh`,
# authenticated with write access to the scratch repository, which must hold
# a commit for the tag.
#
# Usage: test-release.sh <run-id> <owner/name>

if [[ $# -ne 2 ]]; then
	echo "usage: test-release.sh <run-id> <owner/name>" >&2
	exit 1
fi
run="$1"
repo="$2"

if [[ ! "${run}" =~ ^[0-9]+$ ]]; then
	echo "test-release.sh: ${run} is not a run id" >&2
	exit 1
fi
if [[ ! "${repo}" =~ ^[A-Za-z0-9-]+/[A-Za-z0-9._-]+$ ]]; then
	echo "test-release.sh: ${repo} is not owner/name" >&2
	exit 1
fi
if [[ "$(printf '%s' "${repo}" | tr '[:upper:]' '[:lower:]')" == "minodisk/handpic" ]]; then
	echo "test-release.sh: refusing to publish a test build to ${repo}" >&2
	exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dist="$(mktemp -d)"
trap 'rm -rf "${dist}"' EXIT

# Each artifact is downloaded into a folder of its own name.
gh run download "${run}" --repo minodisk/handpic --dir "${dist}"
files=()
for folder in "${dist}"/*/; do
	for file in "${folder}"*; do
		mv "${file}" "${dist}/"
		files+=("$(basename "${file}")")
	done
	rmdir "${folder}"
done

zips=("${dist}"/Handpic-*-windows-x64.zip)
if [[ ${#zips[@]} -ne 1 || ! -f "${zips[0]}" ]]; then
	echo "test-release.sh: run ${run} has no single portable ZIP" >&2
	exit 1
fi
version="$(basename "${zips[0]}")"
version="${version#Handpic-}"
version="${version%-windows-x64.zip}"
bash "${root}/tools/package/version.sh" semver "${version}" >/dev/null

(cd "${dist}" && sha256sum --check --strict SHA256SUMS)
for file in "${files[@]}"; do
	if [[ "${file}" != SHA256SUMS ]] && ! awk -v name="${file}" '$2 == name || $2 == "*" name { found = 1 } END { exit !found }' "${dist}/SHA256SUMS"; then
		echo "test-release.sh: SHA256SUMS does not name ${file}" >&2
		exit 1
	fi
done

prerelease=false
if [[ "${version}" == *-* ]]; then
	prerelease=true
fi
assets=()
for file in "${files[@]}"; do
	assets+=("${dist}/${file}")
done
gh release create "v${version}" "${assets[@]}" \
	--repo "${repo}" \
	--title "v${version}" \
	--notes "A test build of Handpic ${version} from Package run ${run}, signed with the checked-in test key. Never distribute it." \
	--prerelease="${prerelease}"
