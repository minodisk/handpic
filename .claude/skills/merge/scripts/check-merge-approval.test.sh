#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Tests for check-merge-approval.sh's classification.
#
# The classification is the sole gate deciding what merges without a human, so a
# path silently falling to the safe side is the failure this guards against.
# Every case therefore pins the expected exit code and, for approval and
# unclassified, the line for the path.
#
# Run it directly, or via `mise run ci`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/check-merge-approval.sh"

failures=0
cases=0

# Indent a multi-line block for the failure output.
function indent() {
	local text="$1"
	printf '  %s\n' "${text//$'\n'/$'\n'  }"
}

# expect <expected exit> <expected verdict: approval|unclassified|safe> <path>...
# Feeds the paths on stdin and checks the exit code, plus that each path's
# verdict line is present (or absent, for safe).
function expect() {
	local expected_status="$1"
	local expected_verdict="$2"
	shift 2
	local paths=("$@")
	local output status=0

	cases=$((cases + 1))

	output=$(printf '%s\n' "${paths[@]}" |
		CHECK_MERGE_APPROVAL_STDIN=1 bash "${target}" --paths-from-stdin 1) || status=$?

	if [[ "${status}" -ne "${expected_status}" ]]; then
		echo "FAIL: ${paths[*]}" >&2
		echo "  expected exit ${expected_status}, got ${status}" >&2
		indent "${output}" >&2
		failures=$((failures + 1))
		return
	fi

	local path
	for path in "${paths[@]}"; do
		case "${expected_verdict}" in
		approval)
			if ! grep -q "^approval ${path}: " <<<"${output}"; then
				echo "FAIL: ${path} was not classified as approval" >&2
				indent "${output}" >&2
				failures=$((failures + 1))
			fi
			;;
		unclassified)
			if ! grep -qx "unclassified ${path}" <<<"${output}"; then
				echo "FAIL: ${path} was not classified as unclassified" >&2
				indent "${output}" >&2
				failures=$((failures + 1))
			fi
			;;
		safe)
			if grep -q "^approval ${path}: \|^unclassified ${path}\$" <<<"${output}"; then
				echo "FAIL: ${path} was not classified as safe" >&2
				indent "${output}" >&2
				failures=$((failures + 1))
			fi
			;;
		esac
	done
}

# --- safe ---------------------------------------------------------------
expect 0 safe README.md CLAUDE.md docs/plans/20260101-foo/plan.md todo.md
expect 0 safe crates/cli/src/main.rs crates/cli/src/arw.rs
expect 0 safe .gitignore .gitattributes _typos.toml .shellcheckrc

# --- approval: self-referential -----------------------------------------
expect 1 approval .claude/settings.json .claude/agents/planner.md
expect 1 approval .github/workflows/ci.yml .github/actions/setup/action.yml
expect 1 approval CODEOWNERS

# Markdown under .claude / .github must NOT fall through to the *.md safe rule.
expect 1 approval .claude/skills/develop/SKILL.md .github/copilot-instructions.md

# --- approval: build-time code execution ---------------------------------
expect 1 approval build.rs crates/cli/build.rs

# A manifest on its own is safe; Cargo.lock below is what gates a dependency
# actually arriving. Pinned so the two never drift back together.
expect 0 safe Cargo.toml crates/cli/Cargo.toml

# --- approval: release-shaping ------------------------------------------
expect 1 approval release-please-config.json .release-please-manifest.json

# --- approval: toolchain and supply chain --------------------------------
expect 1 approval Cargo.lock .cargo/config.toml
expect 1 approval mise.toml mise.lock rust-toolchain.toml
expect 1 approval tools/git/delete_merged_branches.sh
expect 1 approval lefthook.yml .vscode/settings.json

# --- unclassified (fail-closed) ------------------------------------------
# A directory nobody has classified must not fall silently to the safe side.
expect 1 unclassified some-new-dir/thing.rs Makefile

# --- mixed: one approval path poisons the whole PR ------------------------
# A PR of mostly safe paths still requires approval when one path does not.
cases=$((cases + 1))
mixed_status=0
mixed_output=$(printf 'README.md\ncrates/cli/src/main.rs\nCargo.lock\n' |
	CHECK_MERGE_APPROVAL_STDIN=1 bash "${target}" --paths-from-stdin 1) || mixed_status=$?
if [[ "${mixed_status}" -ne 1 ]] ||
	! grep -q '^approval Cargo.lock: ' <<<"${mixed_output}" ||
	! grep -qx 'TOTAL=3 APPROVAL=1 UNCLASSIFIED=0 SAFE=2' <<<"${mixed_output}"; then
	echo "FAIL: one approval path among safe ones should require approval" >&2
	indent "${mixed_output}" >&2
	failures=$((failures + 1))
fi

# --- argument validation --------------------------------------------------
cases=$((cases + 1))
if CHECK_MERGE_APPROVAL_STDIN=1 bash "${target}" --paths-from-stdin 0 </dev/null 2>/dev/null; then
	echo "FAIL: a pr-number of 0 should be rejected" >&2
	failures=$((failures + 1))
fi

cases=$((cases + 1))
if printf 'README.md\n' | bash "${target}" --paths-from-stdin 1 >/dev/null 2>&1; then
	echo "FAIL: --paths-from-stdin without CHECK_MERGE_APPROVAL_STDIN=1 should be rejected" >&2
	failures=$((failures + 1))
fi

# --- zero changed files ----------------------------------------------------
cases=$((cases + 1))
zero_status=0
zero_output=$(printf '' | CHECK_MERGE_APPROVAL_STDIN=1 bash "${target}" --paths-from-stdin 1) || zero_status=$?
if [[ "${zero_status}" -ne 1 ]] || ! grep -q '^unclassified (no files): ' <<<"${zero_output}"; then
	echo "FAIL: zero changed files should be approval required" >&2
	indent "${zero_output}" >&2
	failures=$((failures + 1))
fi

if [[ "${failures}" -gt 0 ]]; then
	echo "check-merge-approval: ${failures} failure(s) in ${cases} cases" >&2
	exit 1
fi

echo "check-merge-approval: ${cases} cases passed"
