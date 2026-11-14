#!/usr/bin/env bash
set -euo pipefail

# Usage: commit-push.sh "<commit message>" <path>...
# Format and verify the whole repository, then stage only the given paths,
# commit with the message, and push.
# With no upstream set, it pushes with -u origin <current-branch>.
#
# Sandbox: this script assumes it is always called with the Bash tool's
# `dangerouslyDisableSandbox: true`. Setting an upstream writes to the shared
# `.git/config`, which under the sandbox always fails with
# `could not lock config file ...: Operation not permitted` (measured).
#
# Always run `mise run fmt` → `mise run ci:local` before pushing. fmt runs over
# the whole repository; ci:local runs the checks the branch's changed paths
# select, uncommitted ones included (tools/ci/local.sh), and leaves the rest to
# the PR's GitHub checks. Only the <path>... given as arguments go
# into the commit (`git add "$@"`), so a file formatted outside that set just
# stays in the working tree uncommitted. This assumes it is called with an empty
# index (no staged changes), so it fails if there are any (the guard below).
# Pushing unformatted Markdown on the first push makes the fmt CI fail after PR
# creation, forcing a follow-up formatting push — and then the automated review
# has run against the first push (an older head) and the head counts as
# unreviewed. Running fmt here every time keeps the head from drifting.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=_lib.sh
source "$script_dir/_lib.sh"

if [ "$#" -lt 2 ]; then
	echo "Usage: $0 \"<commit message>\" <path>..." >&2
	exit 1
fi

# To keep the "commit only the given paths" premise, verify the index is empty
# first. Pre-existing staged changes would join git add "$@" in the same commit.
if ! git diff --cached --quiet; then
	echo "Error: staged changes already present; call commit-push.sh with a clean index" >&2
	exit 1
fi

msg="$1"
shift

# fmt/ci run over the whole repository and are heavy. Given a bad path, the
# failure only surfaces at the git add after formatting and verification are
# done, leaving the working tree modified for nothing. To avoid that, verify
# every path exists before fmt/ci and fail early.
for path in "$@"; do
	if [ ! -e "$path" ]; then
		echo "Error: path not found: $path" >&2
		exit 1
	fi
done

mise run fmt
mise run ci:local

# Wait for the lock *after* fmt/ci. They take tens of seconds, so waiting
# beforehand tells you nothing about availability at the moment the index is
# actually written.
wait_for_index_lock

git add "$@"
git commit -m "$msg"

branch="$(git rev-parse --abbrev-ref HEAD)"
if git rev-parse --abbrev-ref --symbolic-full-name "@{upstream}" >/dev/null 2>&1; then
	git push
else
	git push -u origin "$branch"
	# push -u returns exit 0 even when the config write fails, so verify and repair afterwards.
	ensure_upstream "$branch"
fi
