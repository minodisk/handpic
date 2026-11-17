#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Tests for wait-pr-actionable.sh's ACTION and exit code, through pr-status.sh.
#
# pr-runner and merger branch on the exit code and the ACTION= line, so every
# case pins both. `gh` is replaced by a stub placed first on PATH that answers
# successive `gh pr view` calls with successive fixtures, the last repeating,
# so a case can be a sequence of PR states.
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
count=$(($(cat "${STUB_CASE}/count") + 1))
echo "${count}" >"${STUB_CASE}/count"
last=$(cat "${STUB_CASE}/last")
if ((count > last)); then
	count="${last}"
fi
cat "${STUB_CASE}/${count}.json"
STUB
chmod +x "${stub_dir}/gh"

failures=0
cases=0

# Indent a multi-line block for the failure output.
function indent() {
	local text="$1"
	printf '  %s\n' "${text//$'\n'/$'\n'  }"
}

# expect <name> <expected exit> <max_wait_sec> <fixture>... -- <expected line>...
# Runs the script against the fixtures, one per `gh pr view` call, and checks
# the exit code and that each expected line is present in the output.
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

	output=$(PATH="${stub_dir}:${PATH}" STUB_CASE="${case_dir}" BLOCKED_GRACE=2 PR_APPROVER_LOGIN="" \
		bash "${target}" 1 1 "${max_wait}" 2>&1) || status=$?

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

if [[ "${failures}" -gt 0 ]]; then
	echo "wait-pr-actionable: ${failures} failure(s) in ${cases} cases" >&2
	exit 1
fi

echo "wait-pr-actionable: ${cases} cases passed"
