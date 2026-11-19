#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# windows.sh — packages the release executable cargo built under
# crates/app/target/release as a per-user Inno Setup setup .exe and a
# portable ZIP (one Handpic/ folder: the exe and licenses/), both for x64, and
# a ZIP holding only the setup, for the updater (sign.sh signs it and the
# portable ZIP). Prints their paths. It builds nothing. Runs under Git Bash.
#
# Usage: windows.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The executable's name.
binary="handpic.exe"

release="${root}/crates/app/target/release"
version="$(bash "${root}/tools/package/version.sh")"
file_version="$(bash "${root}/tools/package/version.sh" windows)"
setup_name="Handpic-${version}-windows-x64-setup"
zip="${release}/Handpic-${version}-windows-x64.zip"
setup_zip="${release}/${setup_name}.exe.zip"

if [[ ! -f "${release}/${binary}" ]]; then
	echo "windows.sh: no ${release}/${binary}; build the release binary first." >&2
	exit 1
fi

# winget's Inno Setup is not on PATH, so look in its install folders too.
function find_iscc() {
	local candidate
	if command -v iscc >/dev/null 2>&1; then
		command -v iscc
		return
	fi
	for candidate in \
		"$(cygpath -u "${LOCALAPPDATA:-}")/Programs/Inno Setup 6/ISCC.exe" \
		"/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
		"/c/Program Files/Inno Setup 6/ISCC.exe"; do
		if [[ -x "${candidate}" ]]; then
			echo "${candidate}"
			return
		fi
	done
	echo "windows.sh: ISCC.exe (Inno Setup 6) not found." >&2
	echo "  Install it (winget install JRSoftware.InnoSetup) or put ISCC.exe on PATH." >&2
	return 1
}

iscc="$(find_iscc)"
echo "windows.sh: ${iscc}" >&2

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT
mkdir -p "${stage}/Handpic"
cp "${release}/${binary}" "${stage}/Handpic/${binary}"
bash "${root}/tools/package/notices.sh" "${stage}/Handpic/licenses"

# MSYS would rewrite ISCC's /-options as paths.
MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' "${iscc}" \
	"/DAppVersion=${version}" \
	"/DFileVersion=${file_version}" \
	"/DBinary=${binary}" \
	"/DReleaseDir=$(cygpath -w "${release}")" \
	"/DLicensesDir=$(cygpath -w "${stage}/Handpic/licenses")" \
	"/O$(cygpath -w "${release}")" \
	"/F${setup_name}" \
	"$(cygpath -w "${root}/crates/app/packaging/windows/handpic.iss")" >&2

# Windows' own bsdtar writes a ZIP; Git Bash's GNU tar and a stock Windows
# have no zip.
bsdtar="$(cygpath -u "${SYSTEMROOT}")/System32/tar.exe"
rm -f "${zip}" "${setup_zip}"
"${bsdtar}" -a -c -f "$(cygpath -w "${zip}")" -C "$(cygpath -w "${stage}")" Handpic
"${bsdtar}" -a -c -f "$(cygpath -w "${setup_zip}")" -C "$(cygpath -w "${release}")" "${setup_name}.exe"

echo "${release}/${setup_name}.exe"
echo "${zip}"
echo "${setup_zip}"
