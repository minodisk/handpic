#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# assets.sh — prints the names of the eleven assets a release of the app
# carries for the given version, one per line: the names the packaging
# scripts write and crates/engine/src/update.rs asks for, plus SHA256SUMS.
# A release carries exactly these, and no latest.json. The version is
# refused as version.sh refuses one.
#
# Usage: assets.sh <version>

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="${1:?usage: assets.sh <version>}"

bash "${root}/tools/package/version.sh" semver "${version}" >/dev/null

cat <<EOF
Handpic-${version}-macos-arm64.dmg
Handpic-${version}-macos-x86_64.dmg
Handpic-${version}-macos-arm64.tar.gz
Handpic-${version}-macos-x86_64.tar.gz
Handpic-${version}-windows-x64-setup.exe
Handpic-${version}-windows-x64-setup.exe.zip
Handpic-${version}-windows-x64.zip
Handpic-${version}-linux-x64.tar.gz
install.sh
handpic_${version}_amd64.deb
SHA256SUMS
EOF
