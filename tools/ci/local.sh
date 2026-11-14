#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# local.sh — the light local check behind `mise run ci:local`.
#
# Selects each check from the branch's changed paths and runs only those. The
# Tauri crate (riffle-app) is never built and the frontend tests never run; a
# crate that depends on a changed one is not tested. GitHub Actions runs all of
# it on the PR (`lint`, `test` on three platforms, `native`), and
# `mise run ci` stays the full local run.
#
# Changed paths: the working tree against `git merge-base HEAD origin/main`
# (committed and uncommitted, renames as both paths) plus the untracked,
# unignored files. It never fetches: a stale origin/main only moves the merge
# base back, which can only add checks.
#
# Usage:
#   local.sh                    select from the changed paths and run
#   local.sh --select <mode>    print the checks for the paths on stdin, one
#                               per line, and run nothing; <mode> is `none`,
#                               `moved` (a path was deleted or renamed) or
#                               `all` (as when the merge base is missing)

# The pinned toolchain of crates/native, as .github/workflows/ci.yml's
# `native` job and `[tasks."native:check"]` use it.
native_rust="rust@1.98.1"

# The root packages checked one by one (crates/core, crates/backend,
# crates/cli); riffle-app, the Tauri crate, is never built.
root_packages=(riffle-core riffle-backend riffle-cli)

# Each bash test and the paths whose change runs it.
bash_tests=(
	.claude/skills/merge/scripts/check-merge-approval.test.sh
	.claude/skills/merge/scripts/wait-post-merge-runs.test.sh
	.claude/skills/merge-settings/scripts/merge.test.sh
	tools/ci/local.test.sh
)

# The extensions vp's formatter handles: the main extensions of each language
# oxfmt 0.68.0 (the one vite-plus 0.3.3 ships) formats, probed by running it
# over one file of each, except `xhtml` and `pcss`, which come from its
# linguist table. `.md` is left out (vite.config.ts ignores it); the
# formatter's other ignored paths still select `vp check`, harmlessly.
vp_extensions=(
	js jsx mjs cjs ts tsx mts cts
	json jsonc json5
	yaml yml
	toml
	html htm xhtml vue
	css scss less pcss
	graphql gql
	hbs handlebars
	mdx markdown
)

# The globs `[tasks.lint]` passes to shellcheck, as regular expressions.
shellcheck_patterns=(
	'^\.claude/skills/[^/]+/scripts/[^/]+\.sh$'
	'^tools/git/[^/]+\.sh$'
	'^tools/stats/[^/]+\.sh$'
	'^tools/ci/[^/]+\.sh$'
)

# select_checks <none|moved|all>
# Reads the changed paths on stdin and prints the selected checks, one per line,
# in the order they run. A pure function of its input.
function select_checks() {
	local mode="$1"
	local all=0 moved=0
	case "${mode}" in
	all) all=1 ;;
	moved) moved=1 ;;
	none) ;;
	*)
		echo "unknown mode: ${mode}" >&2
		return 2
		;;
	esac

	local fmt_root=0 fmt_native=0 vp=0 actionlint=0 lychee=0 native=0
	local packages=' ' tests=' ' scripts=''
	local path ext pattern any=0
	while IFS= read -r path; do
		[[ -n "${path}" ]] || continue
		any=1
		case "${path}" in
		mise.toml | .shellcheckrc) all=1 ;;
		esac
		if [[ "${path}" == *.rs ]]; then
			case "${path}" in
			crates/app/* | crates/backend/* | crates/cli/* | crates/core/*) fmt_root=1 ;;
			crates/native/*) fmt_native=1 ;;
			esac
		fi
		case "${path}" in
		Cargo.toml | Cargo.lock)
			packages+="riffle-core "
			packages+="riffle-backend "
			packages+="riffle-cli "
			;;
		crates/core/*) packages+="riffle-core " ;;
		crates/backend/*) packages+="riffle-backend " ;;
		crates/cli/*) packages+="riffle-cli " ;;
		crates/native/*) native=1 ;;
		esac
		case "${path}" in
		crates/app/ui/* | package.json | pnpm-lock.yaml | vite.config.ts | tsconfig.json) vp=1 ;;
		.claude/* | *.md) ;;
		*.*)
			ext="${path##*.}"
			if [[ "${ext}" != */* && " ${vp_extensions[*]} " == *" ${ext} "* ]]; then
				vp=1
			fi
			;;
		esac
		for pattern in "${shellcheck_patterns[@]}"; do
			if [[ "${path}" =~ ${pattern} ]]; then
				scripts+="${path}"$'\n'
			fi
		done
		case "${path}" in
		.github/workflows/*) actionlint=1 ;;
		esac
		case "${path}" in
		.claude/skills/merge/scripts/check-merge-approval.sh | .claude/skills/merge/scripts/check-merge-approval.test.sh)
			tests+=".claude/skills/merge/scripts/check-merge-approval.test.sh "
			;;
		.claude/skills/merge/scripts/wait-post-merge-runs.sh | .claude/skills/merge/scripts/wait-post-merge-runs.test.sh)
			tests+=".claude/skills/merge/scripts/wait-post-merge-runs.test.sh "
			;;
		.claude/skills/merge-settings/scripts/merge.js | .claude/skills/merge-settings/scripts/merge.test.sh)
			tests+=".claude/skills/merge-settings/scripts/merge.test.sh "
			;;
		tools/ci/*) tests+="tools/ci/local.test.sh " ;;
		esac
		if [[ "${path}" == *.md ]]; then
			lychee=1
		fi
	done

	if [[ "${all}" -eq 0 && "${any}" -eq 0 ]]; then
		return 0
	fi
	if [[ "${moved}" -eq 1 ]]; then
		lychee=1
	fi

	local package test
	if [[ "${all}" -eq 1 ]]; then
		fmt_root=1 fmt_native=1 vp=1 actionlint=1 lychee=1 native=1
		for package in "${root_packages[@]}"; do
			packages+="${package} "
		done
		for test in "${bash_tests[@]}"; do
			tests+="${test} "
		done
	fi
	local native_clippy="${native}"
	if [[ "${packages}" == *" riffle-core "* || "${packages}" == *" riffle-backend "* ]]; then
		native_clippy=1
	fi

	[[ "${fmt_root}" -eq 0 ]] || echo "fmt root"
	[[ "${fmt_native}" -eq 0 ]] || echo "fmt native"
	[[ "${vp}" -eq 0 ]] || echo "vp-check"
	if [[ "${all}" -eq 1 ]]; then
		echo "shellcheck-all"
	elif [[ -n "${scripts}" ]]; then
		printf '%s' "${scripts}" | LC_ALL=C sort -u | while IFS= read -r path; do echo "shellcheck ${path}"; done
	fi
	[[ "${actionlint}" -eq 0 ]] || echo "actionlint"
	[[ "${lychee}" -eq 0 ]] || echo "lychee"
	for test in "${bash_tests[@]}"; do
		[[ "${tests}" != *" ${test} "* ]] || echo "bash-test ${test}"
	done
	for package in "${root_packages[@]}"; do
		[[ "${packages}" != *" ${package} "* ]] || echo "clippy ${package}"
	done
	for package in "${root_packages[@]}"; do
		[[ "${packages}" != *" ${package} "* ]] || echo "test ${package}"
	done
	[[ "${native_clippy}" -eq 0 ]] || echo "clippy native"
	[[ "${native}" -eq 0 ]] || echo "test native"
}

# Every check select_checks can print, shellcheck counted once.
function every_check() {
	echo "fmt root"
	echo "fmt native"
	echo "vp-check"
	echo "shellcheck"
	echo "actionlint"
	echo "lychee"
	printf 'bash-test %s\n' "${bash_tests[@]}"
	printf 'clippy %s\n' "${root_packages[@]}"
	printf 'test %s\n' "${root_packages[@]}"
	echo "clippy native"
	echo "test native"
}

# changed_paths <base>
# Prints the changed paths, one per line, and on the last line `moved=0` or
# `moved=1`.
function changed_paths() {
	local base="$1"
	local status path old moved=0
	while IFS= read -r -d '' status; do
		case "${status}" in
		R* | C*)
			IFS= read -r -d '' old
			IFS= read -r -d '' path
			printf '%s\n%s\n' "${old}" "${path}"
			[[ "${status}" != R* ]] || moved=1
			;;
		*)
			IFS= read -r -d '' path
			printf '%s\n' "${path}"
			[[ "${status}" != D ]] || moved=1
			;;
		esac
	done < <(git diff -z --name-status -M "${base}")
	git ls-files -z --others --exclude-standard | tr '\0' '\n'
	echo "moved=${moved}"
}

# Puts NASM on PATH for crates/native's build script (it refuses to build
# without it), trying where `winget install NASM.NASM` puts it on Windows.
function ensure_nasm() {
	if ! command -v nasm >/dev/null 2>&1 && [[ -n "${LOCALAPPDATA:-}" ]]; then
		local dir
		dir="$(cygpath -u "${LOCALAPPDATA}" 2>/dev/null || echo "${LOCALAPPDATA}")/bin/NASM"
		if [[ -x "${dir}/nasm.exe" || -x "${dir}/nasm" ]]; then
			export PATH="${PATH}:${dir}"
		fi
	fi
	if ! mise exec "${native_rust}" -- nasm -v >/dev/null 2>&1; then
		echo "ci:local: NASM is not on PATH; crates/native's build script needs it." >&2
		echo "  Install it (winget install NASM.NASM, apt-get install nasm, brew install nasm) and put it on PATH." >&2
		return 1
	fi
}

function run_check() {
	local check="$1"
	echo "==> ${check}"
	case "${check}" in
	"fmt root") cargo fmt --all -- --check ;;
	"fmt native") cargo fmt --manifest-path crates/native/Cargo.toml --all -- --check ;;
	vp-check)
		pnpm install --frozen-lockfile
		pnpm exec vp check
		;;
	shellcheck-all) shellcheck -x .claude/skills/*/scripts/*.sh tools/git/*.sh tools/stats/*.sh tools/ci/*.sh ;;
	"shellcheck "*)
		local script="${check#shellcheck }"
		if [[ -f "${script}" ]]; then
			shellcheck -x "${script}"
		else
			echo "(deleted, nothing to check)"
		fi
		;;
	actionlint) actionlint ;;
	# The same arguments as `[tasks.lint]` in mise.toml; keep the two in step.
	lychee) lychee --offline --include-fragments --no-progress --exclude-path 'docs.plans.review-history' '*.md' 'docs/**/*.md' '.claude/**/*.md' ;;
	"bash-test "*)
		local test="${check#bash-test }"
		if [[ -f "${test}" ]]; then
			bash "${test}"
		else
			echo "(deleted, nothing to run)"
		fi
		;;
	"clippy native")
		ensure_nasm
		(cd crates/native && mise exec "${native_rust}" -- cargo clippy --all-targets -- -D warnings)
		;;
	"test native")
		ensure_nasm
		(cd crates/native && mise exec "${native_rust}" -- cargo test)
		;;
	"clippy "*)
		local -a packages
		read -r -a packages <<<"${check#clippy }"
		cargo clippy "${packages[@]/#/--package=}" --all-targets -- -D warnings
		;;
	"test "*)
		local -a packages
		read -r -a packages <<<"${check#test }"
		cargo test "${packages[@]/#/--package=}"
		;;
	*)
		echo "unknown check: ${check}" >&2
		return 2
		;;
	esac
}

function main() {
	if [[ "${1:-}" == --select ]]; then
		select_checks "${2:?--select needs a mode}"
		return
	fi

	cd "$(dirname "${BASH_SOURCE[0]}")/../.."

	local base mode lines check selected=()
	if base="$(git merge-base HEAD origin/main 2>/dev/null)"; then
		lines="$(changed_paths "${base}")"
		if [[ "${lines##*$'\n'}" == moved=1 ]]; then
			mode=moved
		else
			mode=none
		fi
		lines="${lines%moved=?}"
		while IFS= read -r check; do selected+=("${check}"); done < <(select_checks "${mode}" <<<"${lines%$'\n'}")
	else
		echo "ci:local: no merge base with origin/main; running every check."
		while IFS= read -r check; do selected+=("${check}"); done < <(select_checks all </dev/null)
	fi

	if [[ "${#selected[@]}" -eq 0 ]]; then
		echo "ci:local: no changed path against origin/main; nothing to check."
		return 0
	fi

	local skipped=() joined
	joined="$(printf '%s\n' "${selected[@]}" | sed 's/^shellcheck.*/shellcheck/')"
	while IFS= read -r check; do
		grep -qxF "${check}" <<<"${joined}" || skipped+=("${check}")
	done < <(every_check)
	if grep -q '^shellcheck ' <<<"$(printf '%s\n' "${selected[@]}")"; then
		skipped+=("shellcheck of the unchanged scripts")
	fi
	skipped+=("riffle-app (never built locally)" "the frontend tests (vp test, never run locally)")

	echo "ci:local selected:"
	printf '  %s\n' "${selected[@]}"
	echo "ci:local skipped (left to GitHub Actions):"
	printf '  %s\n' "${skipped[@]}"

	# The root packages' clippy runs and test runs go to cargo as one call each,
	# so a run over several packages builds them in one parallel graph.
	local clippy_packages=() test_packages=()
	for check in "${selected[@]}"; do
		case "${check}" in
		"clippy native" | "test native") ;;
		"clippy "*) clippy_packages+=("${check#clippy }") ;;
		"test "*) test_packages+=("${check#test }") ;;
		esac
	done
	for check in "${selected[@]}"; do
		case "${check}" in
		"clippy native" | "test native") run_check "${check}" ;;
		"clippy "*)
			if [[ "${#clippy_packages[@]}" -gt 0 ]]; then
				run_check "clippy ${clippy_packages[*]}"
				clippy_packages=()
			fi
			;;
		"test "*)
			if [[ "${#test_packages[@]}" -gt 0 ]]; then
				run_check "test ${test_packages[*]}"
				test_packages=()
			fi
			;;
		*) run_check "${check}" ;;
		esac
	done
	echo "ci:local: every selected check passed."
}

main "$@"
