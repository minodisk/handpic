#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# linux.sh — packages the release binary cargo built under
# crates/native/target/release for x86_64 Linux: a tarball (one Handpic/
# folder: bin/, the desktop entry and the icon under share/, and licenses/),
# the install.sh that installs it per user, and a .deb (the binary under
# /usr/lib/handpic, its Depends from dpkg-shlibdeps). Prints their paths. It
# builds nothing. Needs dpkg-dev.
#
# Usage: linux.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# The executable's name.
binary="handpic"
identifier="dev.minodisk.handpic"

packaging="${root}/crates/native/packaging/linux"
release="${root}/crates/native/target/release"
version="$(bash "${root}/tools/package/version.sh")"
tarball="${release}/Handpic-${version}-linux-x64.tar.gz"
installer="${release}/install.sh"
deb="${release}/handpic_${version}_amd64.deb"
icon="${root}/crates/native/icons/128x128@2x.png"
icon_path="share/icons/hicolor/256x256/apps/${identifier}.png"
desktop_path="share/applications/${identifier}.desktop"

if [[ "$(uname -m)" != x86_64 ]]; then
	echo "linux.sh: packages x86_64 only, not $(uname -m)." >&2
	exit 1
fi
if [[ ! -x "${release}/${binary}" ]]; then
	echo "linux.sh: no ${release}/${binary}; build the release binary first." >&2
	exit 1
fi

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT

# The tarball keeps the checked-in desktop entry; install.sh writes its Exec.
portable="${stage}/portable/Handpic"
install -Dm755 "${release}/${binary}" "${portable}/bin/${binary}"
install -Dm644 "${icon}" "${portable}/${icon_path}"
install -Dm644 "${packaging}/${identifier}.desktop" "${portable}/${desktop_path}"
bash "${root}/tools/package/notices.sh" "${portable}/licenses"
tar -czf "${tarball}" -C "${stage}/portable" Handpic

install -m755 "${packaging}/install.sh" "${installer}"

debroot="${stage}/deb"
install -Dm755 "${release}/${binary}" "${debroot}/usr/lib/handpic/${binary}"
install -Dm644 "${icon}" "${debroot}/usr/${icon_path}"
install -d "${debroot}/usr/share/applications"
sed "s|^Exec=.*|Exec=/usr/lib/handpic/${binary}|" "${packaging}/${identifier}.desktop" >"${debroot}/usr/${desktop_path}"
grep -qx "Exec=/usr/lib/handpic/${binary}" "${debroot}/usr/${desktop_path}"
bash "${root}/tools/package/notices.sh" "${debroot}/usr/share/doc/handpic"

# dpkg-shlibdeps reads the package's name from a debian/control.
shlibs="${stage}/shlibs"
mkdir -p "${shlibs}/debian"
printf 'Source: handpic\n\nPackage: handpic\nArchitecture: amd64\n' >"${shlibs}/debian/control"
depends="$(cd "${shlibs}" && dpkg-shlibdeps -O "-e${debroot}/usr/lib/handpic/${binary}")"
depends="${depends#shlibs:Depends=}"
# GPUI opens these at run time, so they are not among the binary's NEEDED:
# wgpu's Vulkan loader (libvulkan.so.1) and the Wayland client library
# (wayland-backend's dlopen). wgpu's GL fallback (libEGL) is left out.
for package in libvulkan1 libwayland-client0; do
	if [[ ! "${depends}" =~ (^|, )${package}([ ,]|$) ]]; then
		depends="${depends}, ${package}"
	fi
done

installed_size="$(du -sk "${debroot}/usr" | cut -f1)"
mkdir -p "${debroot}/DEBIAN"
sed -e "s|@VERSION@|${version}|" \
	-e "s|@INSTALLED_SIZE@|${installed_size}|" \
	-e "s|@DEPENDS@|${depends}|" \
	"${packaging}/control" >"${debroot}/DEBIAN/control"

rm -f "${deb}"
dpkg-deb --root-owner-group --build "${debroot}" "${deb}" >&2

echo "${tarball}"
echo "${installer}"
echo "${deb}"
