#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Decide, from a PR's changed paths, whether it may be auto-merged without user
# approval.
#
# The criterion is **"does a revert undo it?"** — not "does merging cause a side
# effect". A rebuild or redeploy is not cause for approval, because reverting a
# broken change takes the same path back to the previous state. Approval is
# required for what a revert does not undo:
#   (1) self-referential things (this judgment script itself, agent / skill /
#       permission definitions, CODEOWNERS, CI pipeline definitions — anything
#       that rewrites the approval gate itself)
#   (2) things that move external state irreversibly
#   (3) the toolchain and supply chain (mise.toml, Cargo.lock, cargo config,
#       hooks and editor settings that run automatically on a developer machine)
#   (4) build scripts that execute arbitrary code at build time
#
# The judgment is fail-closed.
#   1. Any path matching an approval pattern → approval required
#   2. Every path matching a safe pattern → auto-merge allowed
#   3. Any path matching neither → approval required (reason = unclassified)
# A deny-list alone would silently fall to the auto-merge side whenever a new
# directory appears.
#
# Output goes to stdout, one line per entry.
#   approval <path>: <reason>   matched an approval pattern
#   unclassified <path>         matched neither pattern
# The one exception is zero changed files, which prints a single special line
# `unclassified (no files): PR <pr> has zero changed files` (the path part is
# not a path).
# It ends with the tally line `TOTAL=<n> APPROVAL=<n> UNCLASSIFIED=<n> SAFE=<n>`.
#
# exit code:
#   0 = auto-merge allowed (every path is a safe pattern)
#   1 = approval required (an approval pattern matched, or a path is unclassified)
#   2 = bad arguments / an environment error such as a failed gh call
#
# With `--paths-from-stdin` it does not call gh and reads a newline-separated
# path list from stdin. The judgment, output, and exit code are identical to
# normal mode; only the input route is swapped (used by the test).
# The allow rules in `.claude/settings.json` pass arguments through regardless,
# so enabling it also requires the environment variable
# `CHECK_MERGE_APPROVAL_STDIN=1`. If a command-line argument alone could enable
# it, a path list unrelated to the real PR could be passed to fake the verdict.
#
# Call it by its relative path (the allow rules in `.claude/settings.json`
# assume `bash .claude/...`, so an absolute path produces a consent prompt).

function usage() {
	cat <<EOF
Usage: $0 [--paths-from-stdin] <pr-number>

Judge from a PR's changed paths whether it may be auto-merged.

Arguments:
  pr-number   PR number

Options:
  --paths-from-stdin   Do not call gh; read a newline-separated path list from
                        stdin (for tests; also requires CHECK_MERGE_APPROVAL_STDIN=1)
EOF
}

# Classify one path. **This case is the sole definition of the judgment** (the
# develop path, started without a flag, goes through this script; `/merge` does
# not, because it dispatches with the approved flag). It is evaluated top to
# bottom, so the order of the patterns is the priority.
#   return 0 = approval required (prints the reason to stdout)
#   return 1 = safe
#   return 2 = unclassified (falls to approval required, fail-closed)
# case's * matches / as well, so `crates/*/build.rs` catches a build.rs at any
# depth.
function classify() {
	case "$1" in
	# Agent / skill / permission definitions. Markdown under these is execution
	# logic itself, so they are evaluated before the blanket *.md safe rule.
	.claude/* | .agents/* | .codex/*)
		echo "contains the judgment script / permission allowlist / hooks / agent and skill definitions (the Markdown under them is execution logic too)"
		return 0
		;;
	# Everything under .github, not just workflows. A composite action runs
	# inside the CI workflows, and an automerge policy decides what gets merged
	# without a human.
	.github/*)
		echo "definitions of the CI pipeline, composite actions, and the automerge policy"
		return 0
		;;
	# A cargo build script runs arbitrary code at build time, on both developer
	# machines and CI. A revert does not undo it (the side effects already
	# executed, and any credential exposure, cannot be taken back).
	build.rs | crates/*/build.rs)
		echo "a cargo build script runs arbitrary code at build time, unreviewed, on developer machines and CI alike"
		return 0
		;;
	# A manifest is safe on its own: what actually decides where dependency code
	# comes from is Cargo.lock, which is approval-required below, and adding a
	# dependency always moves the lock. Treating the manifests as
	# approval-required too stopped every ordinary Rust change (a version
	# constraint, a feature flag, a new workspace member) for no added signal.
	# The build-time code execution risk is still covered: build.rs is caught
	# above, and a new dependency that ships one cannot arrive without Cargo.lock
	# moving.
	Cargo.toml) return 1 ;;
	# Release-shaping files: a release built from them reaches installed copies,
	# where a revert on main does not follow.
	release-please-config.json | .release-please-manifest.json)
		echo "decides what a release is (versions, changelog, which packages are bumped); a shipped release is not undone by a revert"
		return 0
		;;
	# Markdown is safe whatever the directory (paths under .claude / .agents /
	# .codex / .github never reach this line: the patterns above are evaluated
	# first and make them approval required).
	*.md) return 1 ;;
	# Rust source, and the per-crate manifests. Merging rebuilds it, but if it is
	# broken a revert takes the same path back. build.rs is the exception and is
	# evaluated above.
	crates/*) return 1 ;;
	# The dependency supply chain itself. A rewrite changes where dependencies
	# come from, so it is fail-closed.
	Cargo.lock)
		echo "the dependency supply chain itself; changes where dependencies are resolved from"
		return 0
		;;
	# Cargo's registry and source-replacement configuration. Rewriting it can
	# redirect where crates are fetched from, which is an entry point for a
	# supply chain attack.
	.cargo/*)
		echo "decides the registry crates are fetched from (a rewrite is an entry point for a supply chain attack)"
		return 0
		;;
	# The toolchain versions the CI and local builds resolve. A rewrite silently
	# swaps the compiler and tools that build and test everything.
	mise.toml | mise.lock | rust-toolchain.toml | rust-toolchain)
		echo "decides the toolchain versions (the Rust compiler and tools) that CI and local builds use"
		return 0
		;;
	# Scripts the permission allowlist, git hooks, and mise tasks point at.
	# Rewriting them unreviewed silently changes what runs on every developer
	# machine.
	tools/*)
		echo "may contain execution scripts referenced by the permission allowlist, git hooks, or mise tasks (an unreviewed rewrite silently changes what runs on developer machines)"
		return 0
		;;
	CODEOWNERS)
		echo "decides who review is requested from on later PRs (control over the approval gate itself)"
		return 0
		;;
	lefthook.yml)
		echo "the definition of the git hooks that run automatically on every developer machine"
		return 0
		;;
	# VS Code auto-applies workspace settings in a trusted workspace.
	# Restricted settings such as terminal.integrated.profiles.* can swap out
	# what process the editor actually launches on a developer machine.
	.vscode/settings.json)
		echo "can swap out the process-launch settings VS Code auto-applies in a trusted workspace"
		return 0
		;;
	# Docs and plans. Nothing executes them (todo.md is already covered by the
	# *.md pattern above).
	docs/*) return 1 ;;
	# Lint / format / local development settings. Only the check workflows and
	# local tooling read them, and no deploy path executes them.
	_typos.toml | .shellcheckrc | .gitattributes | .gitignore | .editorconfig | scratch/*)
		return 1
		;;
	*) return 2 ;;
	esac
}

paths_from_stdin=false
if [[ "${1:-}" == "--paths-from-stdin" ]]; then
	if [[ "${CHECK_MERGE_APPROVAL_STDIN:-}" != "1" ]]; then
		echo "Error: --paths-from-stdin requires CHECK_MERGE_APPROVAL_STDIN=1 (tests only)" >&2
		exit 2
	fi
	paths_from_stdin=true
	shift
fi

if [[ $# -ne 1 ]]; then
	usage >&2
	exit 2
fi

pr=$1

if [[ ! "${pr}" =~ ^[1-9][0-9]*$ ]]; then
	echo "Error: pr-number must be a positive integer, got: ${pr}" >&2
	exit 2
fi

# `gh pr view --json files` truncates at 100, so it is not used.
if [[ "${paths_from_stdin}" == true ]]; then
	files=$(cat)
elif ! files=$(gh api "repos/{owner}/{repo}/pulls/${pr}/files" --paginate -q '.[] | .filename, (.previous_filename // empty)'); then
	echo "Error: failed to list changed files for PR ${pr} (see the line above for gh's error output)" >&2
	exit 2
fi

if [[ -z "${files}" ]]; then
	echo "unclassified (no files): PR ${pr} has zero changed files"
	echo "TOTAL=0 APPROVAL=0 UNCLASSIFIED=1 SAFE=0"
	exit 1
fi

total=0
approval=0
unclassified=0
safe=0

while IFS= read -r path; do
	total=$((total + 1))
	verdict=0
	reason=$(classify "${path}") || verdict=$?
	case "${verdict}" in
	0)
		echo "approval ${path}: ${reason}"
		approval=$((approval + 1))
		;;
	1)
		safe=$((safe + 1))
		;;
	*)
		echo "unclassified ${path}"
		unclassified=$((unclassified + 1))
		;;
	esac
done <<<"${files}"

echo "TOTAL=${total} APPROVAL=${approval} UNCLASSIFIED=${unclassified} SAFE=${safe}"
[[ "${approval}" -eq 0 && "${unclassified}" -eq 0 ]]
