#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# notices.sh — copies the license notices every package of the native app
# ships into the given folder, so each platform carries the same set.
#
# Usage: notices.sh <licenses-dir>

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dest="${1:?usage: notices.sh <licenses-dir>}"

# Each notice's source in the repository and its name in the folder.
notices=(
	"crates/native/packaging/NOTICE-gpui:NOTICE-gpui"
	"crates/native/LICENSE-lucide:LICENSE-lucide"
	"crates/native/themes/LICENSE:LICENSE-themes"
	"crates/core/models/LICENSE:LICENSE-yunet"
	"crates/core/models/LICENSE-mediapipe:LICENSE-mediapipe"
	"crates/core/models/LICENSE-siglip2:LICENSE-siglip2"
)

mkdir -p "${dest}"
for notice in "${notices[@]}"; do
	cp "${root}/${notice%%:*}" "${dest}/${notice#*:}"
done
