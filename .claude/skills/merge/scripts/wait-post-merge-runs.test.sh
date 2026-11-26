#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Tests for wait-post-merge-runs.sh's verdict paths.
#
# The merger branches on the exit code and the STATUS= line, so every case pins
# both, plus the counts and the results table rows. `gh` is replaced by a stub
# placed first on PATH that prints fixture JSON from env vars and counts its
# `gh run list --commit` calls.
#
# The script reads the time with `date +%s` and waits with `sleep`, so both are
# stubbed too, over a clock file: `sleep N` advances it by N without sleeping,
# and each `gh` call advances it by the case's LATENCY (default 0). Every case
# is then deterministic and none waits in real time.
#
# Run it directly, or via `mise run ci`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/wait-post-merge-runs.sh"

stub_dir="$(mktemp -d)"
trap 'rm -rf "${stub_dir}"' EXIT

cat >"${stub_dir}/gh" <<'STUB'
#!/usr/bin/env bash
echo $(($(cat "${STUB_CASE}/clock") + STUB_LATENCY)) >"${STUB_CASE}/clock"
case "$1 $2 ${3:-}" in
"run list --commit="*)
	echo $(($(cat "${STUB_CASE}/count") + 1)) >"${STUB_CASE}/count"
	printf '%s\n' "${STUB_COMMIT_RUNS}"
	;;
"repo view --json"*) echo main ;;
"run list --workflow="*) printf '%s\n' "${STUB_RECENT_RUNS}" ;;
*)
	echo "stub gh: unexpected arguments: $*" >&2
	exit 1
	;;
esac
STUB

cat >"${stub_dir}/date" <<'STUB'
#!/usr/bin/env bash
if [[ "$*" != "+%s" ]]; then
	echo "stub date: unexpected arguments: $*" >&2
	exit 1
fi
cat "${STUB_CASE}/clock"
STUB

cat >"${stub_dir}/sleep" <<'STUB'
#!/usr/bin/env bash
echo $(($(cat "${STUB_CASE}/clock") + $1)) >"${STUB_CASE}/clock"
STUB
chmod +x "${stub_dir}/gh" "${stub_dir}/date" "${stub_dir}/sleep"

failures=0
cases=0

# Indent a multi-line block for the failure output.
function indent() {
	local text="$1"
	printf '  %s\n' "${text//$'\n'/$'\n'  }"
}

# [MAX=n] [GRACE=n] [POLL=n] [LATENCY=n] expect <name> <expected exit> <commit runs json> <recent runs json> <expected line>...
# Runs the script against the fixtures with --max-wait=MAX (default 0),
# INITIAL_GRACE GRACE (default 0), POLL_INTERVAL POLL (default 1) and each gh
# call taking LATENCY seconds (default 0), and checks the exit code and that
# each expected line is present in the output, followed by a
# `gh run list --commit calls: N` line.
function expect() {
	local name="$1"
	local expected_status="$2"
	local commit_runs="$3"
	local recent_runs="$4"
	shift 4
	local case_dir="${stub_dir}/case${cases}"
	local output status=0

	cases=$((cases + 1))
	mkdir "${case_dir}"
	echo 0 >"${case_dir}/count"
	echo 1000 >"${case_dir}/clock"

	output=$(PATH="${stub_dir}:${PATH}" STUB_CASE="${case_dir}" STUB_LATENCY="${LATENCY:-0}" \
		STUB_COMMIT_RUNS="${commit_runs}" STUB_RECENT_RUNS="${recent_runs}" \
		POLL_INTERVAL="${POLL:-1}" INITIAL_GRACE="${GRACE:-0}" \
		bash "${target}" --max-wait="${MAX:-0}" deadbeef 2>&1) || status=$?
	output+=$'\n'"gh run list --commit calls: $(cat "${case_dir}/count")"

	if [[ "${status}" -ne "${expected_status}" ]]; then
		echo "FAIL: ${name}: expected exit ${expected_status}, got ${status}" >&2
		indent "${output}" >&2
		failures=$((failures + 1))
		return
	fi

	local line
	for line in "$@"; do
		if ! grep -qxF "${line}" <<<"${output}"; then
			echo "FAIL: ${name}: missing line: ${line}" >&2
			indent "${output}" >&2
			failures=$((failures + 1))
		fi
	done
}

function run() {
	printf '{"workflowName":"%s","status":"completed","conclusion":"%s","url":"https://example.com/%s","createdAt":"%s"}' "$1" "$2" "$1" "$3"
}

ci_ok="$(run CI success 2026-01-01T00:00:00Z)"
release_ok="$(run Release success 2026-01-01T00:00:00Z)"

expect "all success" 0 "[${ci_ok},${release_ok}]" '[]' \
	$'success\tCI\thttps://example.com/CI' \
	$'success\tRelease\thttps://example.com/Release' \
	'TOTAL_COUNT=2' 'FAILED_COUNT=0' 'SUPERSEDED_COUNT=0' 'STATUS=success'

expect "one failure" 1 "[${ci_ok},$(run Release failure 2026-01-01T00:00:00Z)]" '[]' \
	$'failure\tRelease\thttps://example.com/Release' \
	'TOTAL_COUNT=2' 'FAILED_COUNT=1' 'SUPERSEDED_COUNT=0' 'STATUS=failed'

expect "canceled superseded" 0 "[${ci_ok},$(run Release cancelled 2026-01-01T00:00:00Z)]" \
	'[{"status":"completed","conclusion":"success","createdAt":"2026-01-01T00:05:00Z"}]' \
	$'superseded\tRelease\thttps://example.com/Release' \
	'TOTAL_COUNT=2' 'FAILED_COUNT=0' 'SUPERSEDED_COUNT=1' 'STATUS=success'

expect "no runs" 0 '[]' '[]' 'STATUS=no_runs'

MAX=3 GRACE=100 expect "no runs ends within the max wait" 0 '[]' '[]' \
	'no workflow runs found for commit deadbeef after 3s (typical for chore-only merges)' \
	'STATUS=no_runs' 'gh run list --commit calls: 3'

MAX=120 GRACE=60 POLL=30 LATENCY=36 expect "no runs after the grace, short of the max wait" 0 '[]' '[]' \
	'[elapsed 36s] no runs yet; retrying in 24s' \
	'no workflow runs found for commit deadbeef after 96s (typical for chore-only merges)' \
	'STATUS=no_runs' 'gh run list --commit calls: 2'

MAX=60 POLL=30 LATENCY=20 expect "call latency counts toward the max wait" 2 \
	'[{"workflowName":"CI","status":"in_progress","conclusion":"","url":"https://example.com/CI","createdAt":"2026-01-01T00:00:00Z"}]' '[]' \
	'[elapsed 20s] 1/1 runs still in progress; retrying in 30s' \
	'timeout after 70s; pending=1 awaiting_supersede=0 total=1' \
	'STATUS=timeout' 'gh run list --commit calls: 2'

if [[ "${failures}" -gt 0 ]]; then
	echo "wait-post-merge-runs: ${failures} failure(s) in ${cases} cases" >&2
	exit 1
fi

echo "wait-post-merge-runs: ${cases} cases passed"
