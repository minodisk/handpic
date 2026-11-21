#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# prepend-notice.sh — prepends the reinstall notice
# (../references/reinstall-notice.md), its `<version>` replaced by the tag's
# version, to the notes of the draft release of the given tag, waiting up to
# about 8 minutes for release-please to create the draft. A published
# release is left as it is; notes that already start with the notice are not
# changed again.
#
# Usage: prepend-notice.sh <tag> [repo]
# Prints STATUS=ok | already | published | missing; exits 0 for ok and
# already, 1 for published, 2 for missing.

tag="${1:?usage: prepend-notice.sh <tag> [repo]}"
repo="${2:-minodisk/handpic}"
notice="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/references/reinstall-notice.md"

draft=""
for _ in $(seq 1 48); do
  if draft="$(gh release view "$tag" --repo "$repo" --json isDraft --jq .isDraft 2>/dev/null)"; then
    break
  fi
  draft=""
  sleep 10
done
if [ -z "$draft" ]; then
  echo "STATUS=missing"
  exit 2
fi
if [ "$draft" != "true" ]; then
  echo "STATUS=published"
  exit 1
fi

body="$(gh release view "$tag" --repo "$repo" --json body --jq .body)"
if [[ "$body" == "$(head -n 1 "$notice")"* ]]; then
  echo "STATUS=already"
  exit 0
fi

notes="$(mktemp)"
trap 'rm -f "$notes"' EXIT
{
  sed "s/<version>/${tag#v}/g" "$notice"
  printf '\n%s\n' "$body"
} >"$notes"
gh release edit "$tag" --repo "$repo" --notes-file "$notes" >/dev/null
echo "STATUS=ok"
