#!/usr/bin/env bash
# Commit and push the local review history (the develop skill's local review
# rounds).
#
# Writing a branch like `if ! git diff --cached --quiet; then ... fi`
# agent-side makes a compound command that prompts every time, so the judgment
# is shut inside this script as one atomic call.
#
# This script never writes the index itself (`git status --porcelain` takes an
# optional lock, `git push` does not touch the index). Index writes go through
# commit-push.sh, which also owns waiting on and reclaiming index.lock.
#
# Steps:
#   1. Confirm the review history directory exists (its absence is a caller
#      error, e.g. a typo in the branch name; this prevents the accident of
#      treating it as "no diff" and only pushing)
#   2. Guarantee the index is empty (leftover staged changes contradict
#      commit-push.sh's premise, so fail explicitly)
#   3. If the target directory has a diff (untracked included), commit and push
#      with commit-push.sh (which runs mise run fmt → mise run ci:local before pushing)
#   4. With no diff, push only (the case where the preceding Address phase
#      already committed and pushed. A new branch may have no upstream, so push
#      with the same judgment commit-push.sh uses)
#
# Sandbox: this script assumes it is always called with the Bash tool's
# `dangerouslyDisableSandbox: true`. Setting an upstream writes to the shared
# `.git/config`, which under the sandbox always fails with
# `could not lock config file ...: Operation not permitted` (measured).
#
# usage: commit-review-history.sh <branch-name>
# exit code:
#   0 = ok
#   1 = review history directory missing / leftover staged changes / a sub-command failed
#   2 = bad arguments
set -euo pipefail

if [[ $# -ne 1 ]] || [[ -z "${1:-}" ]]; then
	echo "Usage: $0 <branch-name>" >&2
	exit 2
fi

branch="$1"
history_dir="docs/plans/review-history/${branch}/"

# A missing directory is a caller error (a typo in the branch name, say).
# Failing to distinguish it from "no diff" and only pushing would carry on with
# the review history dropped.
if [[ ! -d "$history_dir" ]]; then
	echo "Error: review history directory not found: ${history_dir}" >&2
	exit 1
fi

if ! git diff --cached --quiet; then
	echo "Error: staged changes already present; commit them and re-run" >&2
	exit 1
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=_lib.sh
source "$script_dir/_lib.sh"

# Untracked files must count as a diff (the first review history always is), so
# judge with `git status --porcelain` rather than `git diff`.
if [[ -n "$(git status --porcelain -- "$history_dir")" ]]; then
	bash "$script_dir/commit-push.sh" "docs: add local review result for ${branch}" "$history_dir"
	exit 0
fi

# Already committed (and past fmt/ci), so just push
current_branch="$(git rev-parse --abbrev-ref HEAD)"
if git rev-parse --abbrev-ref --symbolic-full-name "@{upstream}" >/dev/null 2>&1; then
	git push
else
	git push -u origin "$current_branch"
	# push -u returns exit 0 even when the config write fails, so verify and repair afterwards.
	ensure_upstream "$current_branch"
fi
