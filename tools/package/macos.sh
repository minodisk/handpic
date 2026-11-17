#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# macos.sh — packages the Handpic.app `mise run native:bundle:macos` laid out
# under crates/native/target/release as a DMG for this Mac's architecture:
# signed ad hoc (no Developer ID, no notarization), the app beside an
# /Applications link. Prints the DMG's path. It builds nothing.
#
# Usage: macos.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The executable's name; the rename-to-handpic plan changes it.
binary="riffle-native"

release="${root}/crates/native/target/release"
app="${release}/Handpic.app"
version="$(bash "${root}/tools/package/version.sh")"
arch="$(uname -m)"
dmg="${release}/Handpic-${version}-macos-${arch}.dmg"

if [[ ! -x "${app}/Contents/MacOS/${binary}" ]]; then
	echo "macos.sh: no ${app}; run \`mise run native:bundle:macos\` first." >&2
	exit 1
fi

codesign --force --sign - "${app}"
codesign --verify --strict --verbose=2 "${app}"

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT
ditto "${app}" "${stage}/Handpic.app"
ln -s /Applications "${stage}/Applications"

rm -f "${dmg}"
hdiutil create -volname Handpic -srcfolder "${stage}" -ov -format UDZO "${dmg}" >&2
echo "${dmg}"
