#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# A caller that exports GIT_DIR / GIT_WORK_TREE (a git hook around
# `mise run ci`, say) would point every git call below at the repository they
# name, so the worktree case's commit and `worktree add` would land there.
unset GIT_DIR GIT_WORK_TREE

# Tests for merge.js --write.
#
# Exclusions screen only incoming entries, a run with nothing to add leaves
# .claude/settings.json byte-for-byte unchanged, and a non-permission change
# from another worktree is still written. The --self cases run in a fresh
# temporary directory and need no repository; the all-worktrees case sets up a
# git worktree.
#
# Run it directly, or via `mise run ci`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/merge.js"

work_root="$(mktemp -d)"
trap 'rm -rf "${work_root}"' EXIT

failures=0
cases=0

# The fixture is deliberately unsorted and holds an entry an exclusion pattern
# matches, both of which a write would have changed.
fixture='{
  "permissions": {
    "allow": [
      "Bash(git status)",
      "Bash(gh run watch *)",
      "Bash(cargo test *)"
    ]
  }
}
'

function fail() {
	echo "FAIL: $1: $2" >&2
	failures=$((failures + 1))
}

# setup <name> [local json]
# Creates a case directory with the fixture settings.json, plus a
# settings.local.json when given, and prints its path.
function setup() {
	local dir="${work_root}/$1"
	mkdir -p "${dir}/.claude"
	printf '%s' "${fixture}" >"${dir}/.claude/settings.json"
	if [[ $# -ge 2 ]]; then
		printf '%s\n' "$2" >"${dir}/.claude/settings.local.json"
	fi
	printf '%s' "${dir}"
}

# expect_unchanged <name> [local json]
function expect_unchanged() {
	local name="$1"
	local dir output
	cases=$((cases + 1))
	dir="$(setup "$@")"
	output=$(cd "${dir}" && node "${target}" --self --write 2>&1) || {
		fail "${name}" "merge.js exited non-zero: ${output}"
		return
	}
	if ! cmp -s "${dir}/.claude/settings.json" <(printf '%s' "${fixture}"); then
		fail "${name}" "settings.json changed"
	fi
	if [[ "${output}" != *"Added permissions.allow: 0"* ]]; then
		fail "${name}" "missing 'Added permissions.allow: 0' in: ${output}"
	fi
}

expect_unchanged "no local file"
expect_unchanged "empty local file" '{}'
expect_unchanged "local with only an excluded entry" '{"permissions":{"allow":["Write(notes.md)"]}}'

cases=$((cases + 1))
dir="$(setup "local adds entries" '{"permissions":{"allow":["Bash(mise run ci)","Write(notes.md)","Bash(gh pr view *)"]}}')"
output=$(cd "${dir}" && node "${target}" --self --write 2>&1) || fail "local adds entries" "merge.js exited non-zero: ${output}"
allow=$(node -e 'console.log(JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).permissions.allow.join("\n"))' "${dir}/.claude/settings.json")
if ! grep -qxF 'Bash(mise run ci)' <<<"${allow}"; then
	fail "local adds entries" "allowed entry was not added"
fi
if ! grep -qxF 'Bash(gh pr view *)' <<<"${allow}"; then
	fail "local adds entries" "read-only gh entry was not added"
fi
if grep -qxF 'Write(notes.md)' <<<"${allow}"; then
	fail "local adds entries" "excluded incoming entry was added"
fi
if ! grep -qxF 'Bash(gh run watch *)' <<<"${allow}"; then
	fail "local adds entries" "excluded entry already in settings.json was removed"
fi
if [[ "${output}" != *"Added permissions.allow: 2"* ]]; then
	fail "local adds entries" "missing 'Added permissions.allow: 2' in: ${output}"
fi

cases=$((cases + 1))
dir="$(setup "hooks from another worktree")"
other="${work_root}/hooks-other"
git -C "${dir}" init -q
git -C "${dir}" -c user.name=t -c user.email=t@example.com commit -q --allow-empty -m init
git -C "${dir}" worktree add -q "${other}" -b other
mkdir -p "${other}/.claude"
printf '%s\n' '{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"true"}]}]}}' >"${other}/.claude/settings.json"
output=$(cd "${dir}" && node "${target}" --write 2>&1) || fail "hooks from another worktree" "merge.js exited non-zero: ${output}"
if ! grep -q '"PreToolUse"' "${dir}/.claude/settings.json"; then
	fail "hooks from another worktree" "hooks block from the other worktree was not written: ${output}"
fi

# The whole script again with GIT_DIR / GIT_WORK_TREE exported at a decoy
# repository, which must come out untouched. MERGE_TEST_NESTED keeps the
# nested run from recursing.
function decoy_state() {
	git -C "$1" rev-parse HEAD
	git -C "$1" for-each-ref
	git -C "$1" worktree list --porcelain
}

if [[ -z "${MERGE_TEST_NESTED:-}" ]]; then
	cases=$((cases + 1))
	decoy="${work_root}/decoy"
	git init -q "${decoy}"
	git -C "${decoy}" -c user.name=t -c user.email=t@example.com commit -q --allow-empty -m init
	before="$(decoy_state "${decoy}")"
	output=$(MERGE_TEST_NESTED=1 GIT_DIR="${decoy}/.git" GIT_WORK_TREE="${decoy}" bash "${BASH_SOURCE[0]}" 2>&1) ||
		fail "exported GIT_DIR / GIT_WORK_TREE" "the nested run exited non-zero: ${output}"
	if [[ "$(decoy_state "${decoy}")" != "${before}" ]]; then
		fail "exported GIT_DIR / GIT_WORK_TREE" "the repository they name changed"
	fi
fi

if [[ "${failures}" -gt 0 ]]; then
	echo "merge: ${failures} failure(s) in ${cases} cases" >&2
	exit 1
fi

echo "merge: ${cases} cases passed"
