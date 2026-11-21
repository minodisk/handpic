#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Check that a published GitHub release carries everything a user and the
# app's updater need.
#
# Usage:
#   verify-release.sh <tag> [<owner/name> [<verifying-key>]]
#     e.g. verify-release.sh v2.0.0
#
# The repository defaults to minodisk/handpic and the verifying key to
# crates/app/packaging/update.pub; give the test feed and the test key
# (crates/app/packaging/test/update.pub) to check a test release. Needs gh,
# sha256sum and zipsign 0.2.2 on PATH:
#   mise exec cargo:zipsign@0.2.2 -- bash verify-release.sh <tag>
#
# Checks:
#   - the release exists and is not a draft
#   - it is a prerelease exactly when the version has a prerelease part
#   - exactly the eleven assets tools/package/assets.sh lists are attached
#     (so no latest.json)
#   - SHA256SUMS names the ten others and the downloaded files pass
#     `sha256sum --check` against it
#   - the updater's archives (every .zip and .tar.gz) pass `zipsign verify`
#     with the verifying key, under their release names (zipsign binds a
#     signature to the file's name)
#
# Output: one `ok` / `missing` line per check, then `STATUS=ok|incomplete`.
# Exit: 0 when everything is present, 1 when anything is missing, 2 on bad
# arguments, without zipsign 0.2.2, or when the release cannot be read.

zipsign_version="0.2.2"

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
  echo "usage: verify-release.sh <tag> [<owner/name> [<verifying-key>]]" >&2
  exit 2
fi
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
tag="$1"
repo="${2:-minodisk/handpic}"
key="${3:-${root}/crates/app/packaging/update.pub}"
version="${tag#v}"

if ! expected="$(bash "${root}/tools/package/assets.sh" "$version")"; then
  echo "verify-release.sh: $tag is not v<version>" >&2
  exit 2
fi
if [ ! -f "$key" ]; then
  echo "verify-release.sh: no verifying key at $key" >&2
  exit 2
fi
if [ "$(zipsign --version 2>/dev/null)" != "zipsign ${zipsign_version}" ]; then
  echo "verify-release.sh: needs zipsign ${zipsign_version} on PATH" >&2
  exit 2
fi

if ! release="$(gh release view "$tag" --repo "$repo" \
  --json isDraft,isPrerelease,assets \
  --jq '"\(.isDraft) \(.isPrerelease)", .assets[].name')"; then
  echo "cannot read release $tag of $repo" >&2
  exit 2
fi
read -r draft prerelease <<<"$(head -n 1 <<<"$release")"
attached="$(tail -n +2 <<<"$release")"

missing=0
report() {
  if [ "$1" = ok ]; then
    echo "ok       $2"
  else
    echo "missing  $2"
    missing=1
  fi
}

if [ "$draft" = false ]; then
  report ok "published (not a draft)"
else
  report missing "published (it is a draft)"
fi

want_prerelease=false
if [[ "$version" == *-* ]]; then
  want_prerelease=true
fi
if [ "$prerelease" = "$want_prerelease" ]; then
  report ok "prerelease flag $prerelease for $version"
else
  report missing "prerelease flag $want_prerelease for $version (found $prerelease)"
fi

present=()
while IFS= read -r name; do
  if grep -qxF "$name" <<<"$attached"; then
    report ok "asset $name"
    present+=("$name")
  else
    report missing "asset $name"
  fi
done <<<"$expected"
while IFS= read -r name; do
  if [ -n "$name" ] && ! grep -qxF "$name" <<<"$expected"; then
    report missing "no other asset (found $name)"
  fi
done <<<"$attached"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
if [ "${#present[@]}" -gt 0 ]; then
  patterns=()
  for name in "${present[@]}"; do
    patterns+=(--pattern "$name")
  done
  if ! gh release download "$tag" --repo "$repo" --dir "$work" "${patterns[@]}"; then
    echo "cannot download the assets of $tag of $repo" >&2
    exit 2
  fi
fi

if [ -f "$work/SHA256SUMS" ]; then
  if diff <(grep -vxF SHA256SUMS <<<"$expected" | LC_ALL=C sort) \
    <(awk '{ name = $2; sub(/^\*/, "", name); print name }' "$work/SHA256SUMS" | LC_ALL=C sort) >/dev/null; then
    report ok "SHA256SUMS names the ten other assets"
  else
    report missing "SHA256SUMS names the ten other assets"
  fi
  if (cd "$work" && sha256sum --check --strict --quiet SHA256SUMS) >&2; then
    report ok "sha256sum --check"
  else
    report missing "sha256sum --check"
  fi
else
  report missing "sha256sum --check (no SHA256SUMS)"
fi

while IFS= read -r name; do
  case "$name" in
  *.zip) kind=zip ;;
  *.tar.gz) kind=tar ;;
  *) continue ;;
  esac
  if [ -f "$work/$name" ] && zipsign verify "$kind" "$work/$name" "$key" >&2; then
    report ok "zipsign $name"
  else
    report missing "zipsign $name"
  fi
done <<<"$expected"

if [ "$missing" -eq 0 ]; then
  echo "STATUS=ok"
else
  echo "STATUS=incomplete"
  exit 1
fi
