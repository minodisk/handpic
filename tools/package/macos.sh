#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# macos.sh — packages the Handpic.app `mise run app:bundle:macos` laid out
# under crates/app/target/release for this Mac's architecture, signed ad
# hoc (no Developer ID, no notarization): a DMG, the app beside an
# /Applications link, for people, and a tarball holding Handpic.app, for the
# updater (sign.sh signs it). Prints their paths. It builds nothing.
#
# Usage: macos.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The executable's name.
binary="handpic"

release="${root}/crates/app/target/release"
app="${release}/Handpic.app"
version="$(bash "${root}/tools/package/version.sh")"
arch="$(uname -m)"
dmg="${release}/Handpic-${version}-macos-${arch}.dmg"
tarball="${release}/Handpic-${version}-macos-${arch}.tar.gz"

if [[ ! -x "${app}/Contents/MacOS/${binary}" ]]; then
	echo "macos.sh: no ${app}; run \`mise run app:bundle:macos\` first." >&2
	exit 1
fi

codesign --force --sign - "${app}"
codesign --verify --strict --verbose=2 "${app}"

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT
ditto "${app}" "${stage}/Handpic.app"
ln -s /Applications "${stage}/Applications"

# hdiutil's own size estimate for -srcfolder can come up short and fail with
# "No space left on device", so size the image from the stage with headroom:
# 20 % plus 20 MB.
stage_kb="$(du -sk "${stage}" | cut -f1)"
size_kb=$((stage_kb + stage_kb / 5 + 20 * 1024))

rm -f "${dmg}"
hdiutil create -volname Handpic -srcfolder "${stage}" -size "${size_kb}k" -ov -format UDZO "${dmg}" >&2

# COPYFILE_DISABLE keeps bsdtar from adding `._` entries for extended
# attributes.
rm -f "${tarball}"
COPYFILE_DISABLE=1 tar -czf "${tarball}" -C "${stage}" Handpic.app

echo "${dmg}"
echo "${tarball}"
