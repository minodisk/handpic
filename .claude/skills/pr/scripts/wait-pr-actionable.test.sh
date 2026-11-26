#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Tests for wait-pr-actionable.sh's ACTION and exit code, through pr-status.sh.
#
# pr-runner and merger branch on the exit code and the ACTION= line, so every
# case pins both. `gh` is replaced by a stub placed first on PATH that answers
# successive `gh pr view` calls with successive fixtures, the last repeating,
# so a case can be a sequence of PR states. A fixture reading `FAIL` makes that
# call fail, as a network error or an `api.github.com` outage does.
#
# The script reads the time with `date +%s` and waits with `sleep`, so both are
# stubbed too, over a clock file: `sleep N` advances it by N without sleeping,
# and each `gh` call advances it by the case's LATENCY (default 0). Every case
# is then deterministic and none waits in real time.
#
# Run it directly, or via `mise run ci`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/wait-pr-actionable.sh"

stub_dir="$(mktemp -d)"
trap 'rm -rf "${stub_dir}"' EXIT

cat >"${stub_dir}/gh" <<'STUB'
#!/usr/bin/env bash
if [[ "$1 $2" != "pr view" ]]; then
	echo "stub gh: unexpected arguments: $*" >&2
	exit 1
fi
echo $(($(cat "${STUB_CASE}/clock") + STUB_LATENCY)) >"${STUB_CASE}/clock"
count=$(($(cat "${STUB_CASE}/count") + 1))
echo "${count}" >"${STUB_CASE}/count"
last=$(cat "${STUB_CASE}/last")
if ((count > last)); then
	count="${last}"
fi
if [[ "$(cat "${STUB_CASE}/${count}.json")" == FAIL ]]; then
	echo "stub gh: error connecting to api.github.com" >&2
	exit 1
fi
cat "${STUB_CASE}/${count}.json"
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

# [INTERVAL=n] [GRACE=n] [LATENCY=n] expect <name> <expected exit> <max_wait_sec> <fixture>... -- <expected line>...
# Runs the script against the fixtures, one per `gh pr view` call, with
# interval_sec INTERVAL (default 1), BLOCKED_GRACE GRACE (default 2) and each
# call taking LATENCY seconds (default 0), and checks the exit code and that
# each expected line is present in the output.
function expect() {
	local name="$1"
	local expected_status="$2"
	local max_wait="$3"
	shift 3
	local case_dir="${stub_dir}/case${cases}"
	local output status=0 n=0

	cases=$((cases + 1))
	mkdir "${case_dir}"
	while [[ "$1" != -- ]]; do
		n=$((n + 1))
		printf '%s\n' "$1" >"${case_dir}/${n}.json"
		shift
	done
	shift
	echo 0 >"${case_dir}/count"
	echo "${n}" >"${case_dir}/last"
	echo 1000 >"${case_dir}/clock"

	output=$(PATH="${stub_dir}:${PATH}" STUB_CASE="${case_dir}" STUB_LATENCY="${LATENCY:-0}" \
		BLOCKED_GRACE="${GRACE:-2}" PR_APPROVER_LOGIN="" \
		bash "${target}" 1 "${INTERVAL:-1}" "${max_wait}" 2>&1) || status=$?

	if [[ "${status}" -ne "${expected_status}" ]]; then
		echo "FAIL: ${name}: expected exit ${expected_status}, got ${status}" >&2
		indent "${output}" >&2
		failures=$((failures + 1))
		return
	fi

	# pr-status.sh's output is printed verbatim: a printf format regression
	# (e.g. a missing space before "$out") shows up as a blank line.
	if grep -q '^$' <<<"${output}"; then
		echo "FAIL: ${name}: output has a blank line" >&2
		indent "${output}" >&2
		failures=$((failures + 1))
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

# check <name> <status> [conclusion]
function check() {
	printf '{"__typename":"CheckRun","name":"%s","status":"%s","conclusion":"%s","detailsUrl":"https://example.com/runs/1"}' "$1" "$2" "${3:-}"
}

# context <name> <state>
function context() {
	printf '{"__typename":"StatusContext","context":"%s","state":"%s","targetUrl":"https://example.com/%s"}' "$1" "$2" "$1"
}

# pr <mergeStateStatus> <statusCheckRollup entries, comma separated>
function pr() {
	printf '{"state":"OPEN","mergeable":"MERGEABLE","mergeStateStatus":"%s","reviewDecision":"","headRefOid":"deadbeef","isDraft":false,"number":1,"url":"https://github.com/o/r/pull/1","statusCheckRollup":[%s]}' "$1" "$2"
}

jobs_running=""
jobs_done=""
for job in lint test-linux test-macos test-windows native-linux native-macos native-windows; do
	jobs_running+="${jobs_running:+,}$(check "${job}" IN_PROGRESS)"
	jobs_done+="${jobs_done:+,}$(check "${job}" COMPLETED SUCCESS)"
done
ci_done="$(check ci COMPLETED SUCCESS)"

expect "every non-completed status is pending" 2 0 \
	"$(pr BLOCKED "$(check a QUEUED),$(check b IN_PROGRESS),$(check c WAITING),$(check d PENDING),$(check e REQUESTED),$(context f PENDING),$(context g EXPECTED)")" -- \
	'ACTION=wait' 'checksOverall=pending' 'pendingCount=7' 'failedCount=0'

expect "requested alone is pending" 2 0 \
	"$(pr BLOCKED "$(check a REQUESTED)")" -- \
	'ACTION=wait' 'pendingCount=1'

expect "empty rollup, then running checks, then clean" 0 30 \
	"$(pr BLOCKED "")" \
	"$(pr BLOCKED "")" \
	"$(pr BLOCKED "${jobs_running}")" \
	"$(pr CLEAN "${jobs_done},${ci_done}")" -- \
	'ACTION=ready' 'mergeStateStatus=CLEAN' 'pendingCount=0'

expect "jobs concluded without ci, then ci reports" 0 30 \
	"$(pr BLOCKED "${jobs_done}")" \
	"$(pr CLEAN "${jobs_done},${ci_done}")" -- \
	'ACTION=ready' 'mergeStateStatus=CLEAN'

expect "blocked through the grace" 0 30 \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" -- \
	'[elapsed 1s] ACTION=blocked; retrying in 1s' \
	'ACTION=blocked' 'mergeStateStatus=BLOCKED' 'checksOverall=success' 'pendingCount=0'

expect "timeout inside the grace" 2 0 \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" -- \
	'wait-pr-actionable.sh: timed out after 0s (last ACTION=blocked)' 'ACTION=blocked'

expect "fails, then succeeds" 0 30 \
	FAIL \
	FAIL \
	"$(pr CLEAN "${jobs_done},${ci_done}")" -- \
	'[elapsed 0s] pr-status.sh failed; retrying in 1s' \
	'[elapsed 1s] pr-status.sh failed; retrying in 1s' \
	'ACTION=ready' 'mergeStateStatus=CLEAN'

expect "fails on every call of the slice" 1 2 \
	FAIL \
	FAIL \
	FAIL -- \
	'[elapsed 0s] pr-status.sh failed; retrying in 1s' \
	'[elapsed 1s] pr-status.sh failed; retrying in 1s' \
	'wait-pr-actionable.sh: pr-status.sh failed on every call for 2s'

expect "fails at the end after a success" 2 2 \
	"$(pr BLOCKED "${jobs_running}")" \
	FAIL -- \
	'[elapsed 0s] ACTION=wait; retrying in 1s' \
	'[elapsed 1s] pr-status.sh failed; retrying in 1s' \
	'wait-pr-actionable.sh: timed out after 2s (last call failed; reporting the last successful state)' \
	'ACTION=wait'

expect "a failed call keeps the blocked grace" 0 3 \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" \
	FAIL \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" -- \
	'[elapsed 1s] pr-status.sh failed; retrying in 1s' \
	'ACTION=blocked' 'mergeStateStatus=BLOCKED'

LATENCY=5 expect "call latency alone passes the deadline" 2 3 \
	"$(pr BLOCKED "${jobs_running}")" \
	"$(pr CLEAN "${jobs_done},${ci_done}")" -- \
	'wait-pr-actionable.sh: timed out after 3s (last ACTION=wait)' \
	'ACTION=wait'

INTERVAL=60 GRACE=60 LATENCY=46 expect "a 60 120 slice meets the blocked grace" 0 120 \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" -- \
	'[elapsed 46s] ACTION=blocked; retrying in 60s' \
	'ACTION=blocked' 'mergeStateStatus=BLOCKED'

INTERVAL=60 GRACE=60 LATENCY=46 expect "a 60 60 slice holds one observation" 2 60 \
	"$(pr BLOCKED "${jobs_done},${ci_done}")" -- \
	'[elapsed 46s] ACTION=blocked; retrying in 14s' \
	'wait-pr-actionable.sh: timed out after 60s (last ACTION=blocked)' \
	'ACTION=blocked'

if [[ "${failures}" -gt 0 ]]; then
	echo "wait-pr-actionable: ${failures} failure(s) in ${cases} cases" >&2
	exit 1
fi

echo "wait-pr-actionable: ${cases} cases passed"
