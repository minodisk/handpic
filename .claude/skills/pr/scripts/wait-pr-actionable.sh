#!/usr/bin/env bash
# Poll until the PR reaches a state that needs the agent's judgment, then
# return pr-status.sh's output at that moment verbatim.
#
# The waiting ACTIONs in the pr skill's step 5 (`wait` = checks in progress /
# `review_required` = waiting on the approver) give the agent nothing to decide
# on. Driving them agent-side with `until ...; do sleep 30; done` makes a
# compound command that prompts every time, so the wait loop is shut inside this
# script as one atomic call.
#
# The ACTIONs that end the wait and return:
#   ready / merged / closed / conflict / check_failed / changes_requested /
#   draft / behind / blocked
#
# `blocked` ends the wait only once it has been observed continuously for
# BLOCKED_GRACE seconds (default 60) of real time. A required check that has
# not reported yet makes the PR BLOCKED with nothing pending: right after the
# PR is created, before any check run is registered, and between the last job
# finishing and an aggregate check (`needs:`) appearing. Any other ACTION
# observed meanwhile resets the grace. The grace restarts on every call of the
# script, so it is met only within one slice, and a timeout during it exits 2
# with the last output (ACTION=blocked), as for `wait`.
#
# A failed pr-status.sh (a network error, an `api.github.com` outage) is
# treated like a waiting state: reported on stderr and slept on. It neither
# resets nor ends the `blocked` grace. Only a slice in which every call failed
# exits 1, so one exit 1 stands for an outage as long as the slice. A slice
# that ends on a failed call after at least one success exits 2 with the last
# successful output, as ordinary waiting.
#
# max_wait_sec is a deadline on real elapsed time (`date +%s`), the time spent
# inside pr-status.sh included. After each call the script times out if the
# deadline has passed, else sleeps interval_sec or the time left, whichever is
# shorter, and times out with the last observed state when that sleep reaches
# the deadline, making no further call (the caller's re-run starts with one).
# So a slice lasts max_wait_sec, plus at most the one call in flight when the
# deadline passes. max_wait_sec=0 makes exactly one call.
#
# The default max_wait_sec is 240 (4 minutes). The Bash tool's default
# foreground timeout can be as low as 120 s, so callers must pass the Bash
# tool's `timeout: 600000` (ms) explicitly; 600 s is the maximum. Keep the wait
# shorter than that to always land on "the script returns exit 2 itself" (killed
# by the tool, the exit code is unobservable and cannot be branched on).
# The harness also moves a foreground command to the background after about
# 180 s, so the agent callers (pr-runner, merger) pass a shorter slice,
# `60 120`: calls at 0 s and one call plus 60 s later, so two observations at
# least 60 s apart meet the default BLOCKED_GRACE whenever the first call takes
# under 60 s. The slice is 120 s, or up to the second call's end when that call
# straddles the deadline (about 152 s with the 46 s calls measured on a slow
# api.github.com), and returns by itself before 180 s. A slice no longer than
# interval_sec plus one call (`60 60`) holds one observation and never meets
# the grace. Their slice-count limits assume the 120 s slice.
# To wait longer, callers extend the wait by re-running and counting exit 2, not
# by backgrounding.
#
# usage: wait-pr-actionable.sh <PR_number_or_url> [interval_sec=30] [max_wait_sec=240]
# exit code:
#   0 = reached a non-waiting ACTION (branch on the ACTION= line in the output)
#   1 = error (bad arguments / pr-status.sh failed on every call of the slice)
#   2 = timeout (prints the last observed state to stdout; calling again is fine)
set -euo pipefail

PR="${1:?usage: wait-pr-actionable.sh <PR_number_or_url> [interval_sec] [max_wait_sec]}"
INTERVAL="${2:-30}"
MAX_WAIT="${3:-240}"
GRACE="${BLOCKED_GRACE:-60}"

if ! [[ "$INTERVAL" =~ ^[1-9][0-9]*$ ]] || ! [[ "$MAX_WAIT" =~ ^[0-9]+$ ]]; then
	echo "wait-pr-actionable.sh: interval_sec must be a positive integer and max_wait_sec a non-negative integer (got '$INTERVAL' / '$MAX_WAIT')" >&2
	exit 1
fi
if ! [[ "$GRACE" =~ ^[0-9]+$ ]]; then
	echo "wait-pr-actionable.sh: BLOCKED_GRACE must be a non-negative integer (got '$GRACE')" >&2
	exit 1
fi

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

function clock() {
	date +%s
}

# End the slice with the last observed state.
function time_out() {
	if [[ "$failed" == true ]]; then
		if [[ -n "$last_out" ]]; then
			echo "wait-pr-actionable.sh: timed out after ${MAX_WAIT}s (last call failed; reporting the last successful state)" >&2
			printf '%s\n' "$last_out"
			exit 2
		fi
		echo "wait-pr-actionable.sh: pr-status.sh failed on every call for ${MAX_WAIT}s" >&2
		exit 1
	fi
	echo "wait-pr-actionable.sh: timed out after ${MAX_WAIT}s (last ACTION=${action})" >&2
	printf '%s\n' "$last_out"
	exit 2
}

start=$(clock)
deadline=$((start + MAX_WAIT))
blocked_since=""
last_out=""
action=""
failed=false
while true; do
	if out=$(bash "$script_dir/pr-status.sh" "$PR"); then
		failed=false
		last_out="$out"
		action=$(grep -E '^ACTION=' <<<"$out" | cut -d= -f2)
		now=$(clock)
		case "$action" in
		wait | review_required) blocked_since="" ;;
		blocked)
			blocked_since="${blocked_since:-$now}"
			if ((now - blocked_since >= GRACE)); then
				printf '%s\n' "$out"
				exit 0
			fi
			;;
		*)
			printf '%s\n' "$out"
			exit 0
			;;
		esac
		note="ACTION=${action}"
	else
		failed=true
		now=$(clock)
		note="pr-status.sh failed"
	fi
	if ((now >= deadline)); then
		time_out
	fi
	nap=$((deadline - now))
	if ((nap > INTERVAL)); then
		nap=$INTERVAL
	fi
	echo "[elapsed $((now - start))s] ${note}; retrying in ${nap}s" >&2
	sleep "$nap"
	if (($(clock) >= deadline)); then
		time_out
	fi
done
