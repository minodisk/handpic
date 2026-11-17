#!/bin/sh
# install.sh — installs Handpic for the current user from the release's
# Handpic-<version>-linux-x64.tar.gz: the binary to ~/.local/bin, the desktop
# entry (its Exec rewritten to the binary's absolute path) to
# ~/.local/share/applications and the icon to the hicolor theme under
# ~/.local/share, the same layout as `mise run native:install:linux`. The
# whole archive is checked against the given SHA-256 before anything changes,
# and an unsafe or unexpected entry refuses the install. Needs Python 3, not
# root. Take the digest from a trusted channel (the release's SHA256SUMS).
#
# Adapted from the install.sh of GPUI Kit's packaging guide
# (https://gpui-kit.com/docs/packaging/), whose code examples are licensed
# under the Apache License, Version 2.0.
#
# Usage: sh install.sh ARCHIVE EXPECTED_SHA256

set -eu
[ "$#" -eq 2 ] || {
	echo 'Usage: sh install.sh ARCHIVE EXPECTED_SHA256' >&2
	exit 2
}
python3 - "$1" "$2" <<'PY'
from pathlib import Path, PurePosixPath
import hashlib
import io
import os
import re
import shutil
import sys
import tarfile
import tempfile

# The executable's name.
binary_name = "handpic"
identifier = "dev.minodisk.handpic"
top = "Handpic"

archive = Path(sys.argv[1])
expected = sys.argv[2].lower()
if not re.fullmatch(r"[0-9a-f]{64}", expected):
    raise SystemExit("Expected a 64-character SHA-256 digest")
digest = hashlib.sha256()
verified = tempfile.TemporaryFile()
with archive.open("rb") as archive_stream:
    for chunk in iter(lambda: archive_stream.read(1024 * 1024), b""):
        digest.update(chunk)
        verified.write(chunk)
if digest.hexdigest() != expected:
    raise SystemExit("Checksum mismatch; nothing installed")
verified.seek(0)

binary_entry = f"{top}/bin/{binary_name}"
icon_entry = f"{top}/share/icons/hicolor/256x256/apps/{identifier}.png"
desktop_entry = f"{top}/share/applications/{identifier}.desktop"
with tarfile.open(fileobj=verified, mode="r:gz") as bundle:
    members = {}
    for member in bundle:
        path = PurePosixPath(member.name)
        if (not path.parts or path.is_absolute() or ".." in path.parts
                or path.parts[0] != top
                or not (member.isfile() or member.isdir())
                or str(path) in members):
            raise SystemExit("Unsafe or duplicate archive entry")
        members[str(path)] = member
    if not all(name in members and members[name].isfile()
               for name in (binary_entry, icon_entry, desktop_entry)):
        raise SystemExit("Archive is missing its executable, icon or desktop entry")

    bin_dir = Path.home() / ".local/bin"
    data_dir = Path.home() / ".local/share"
    app_dir = data_dir / "applications"
    icon_dir = data_dir / "icons/hicolor/256x256/apps"
    for directory in (bin_dir, app_dir, icon_dir):
        directory.mkdir(parents=True, exist_ok=True)

    binary = bin_dir / binary_name
    icon = icon_dir / f"{identifier}.png"
    desktop = app_dir / f"{identifier}.desktop"
    # Desktop Entry quoting has two escape layers; this handles a home path with spaces.
    exec_path = str(binary).replace("\\", "\\\\\\\\")
    for character in ('"', '`', '$'):
        exec_path = exec_path.replace(character, "\\\\" + character)
    exec_path = exec_path.replace("%", "%%")
    with bundle.extractfile(members[desktop_entry]) as entry_stream:
        lines = entry_stream.read().decode("utf-8").splitlines()
    if sum(line.startswith("Exec=") for line in lines) != 1:
        raise SystemExit("Desktop entry has no single Exec line")
    desktop_content = "".join(
        (f'Exec="{exec_path}"' if line.startswith("Exec=") else line) + "\n"
        for line in lines
    )

    staged_files = []

    def stage_file(destination, input_stream, mode):
        with tempfile.NamedTemporaryFile(
            dir=destination.parent, prefix=".handpic-install-", delete=False
        ) as staged:
            staged_path = Path(staged.name)
            staged_files.append(staged_path)
            shutil.copyfileobj(input_stream, staged)
        staged_path.chmod(mode)
        return staged_path

    try:
        with bundle.extractfile(members[binary_entry]) as entry_stream:
            staged_binary = stage_file(binary, entry_stream, 0o755)
        with bundle.extractfile(members[icon_entry]) as entry_stream:
            staged_icon = stage_file(icon, entry_stream, 0o644)
        staged_desktop = stage_file(
            desktop, io.BytesIO(desktop_content.encode("utf-8")), 0o644
        )
        for staged, destination in (
            (staged_binary, binary),
            (staged_icon, icon),
            (staged_desktop, desktop),
        ):
            os.replace(staged, destination)
    finally:
        for staged in staged_files:
            staged.unlink(missing_ok=True)
verified.close()
print(f"Installed Handpic for the current user: {binary}")
PY
