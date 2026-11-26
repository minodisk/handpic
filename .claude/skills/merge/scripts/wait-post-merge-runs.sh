#!/usr/bin/env bash

set -o errexit
set -o pipefail
set -o nounset

# Wait for GitHub Actions workflow runs triggered by a merge commit to complete.
#
# Usage:
#   wait-post-merge-runs.sh [--max-wait=<seconds>] <merge-commit-sha>
#
# Options:
#   --max-wait=<seconds>  Maximum wait. Takes precedence over MAX_WAIT
#
# Options via env:
#   POLL_INTERVAL   Seconds. Default 30
#   MAX_WAIT        Seconds. Default 1800 (30 minutes). Only used when --max-wait is absent.
#                   Agent callers pass --max-wait=120 and the Bash tool's
#                   `timeout: 600000` (ms), because the default foreground
#                   timeout can be as low as 120 s and the harness moves a
#                   foreground command to the background after about 180 s
#   INITIAL_GRACE   Seconds. Default 60 (waiting for runs to appear in the list),
#                   capped at the max wait
#
# Behavior:
#   - The max wait is a deadline on real elapsed time (`date +%s`), the time
#     spent inside gh included. After each poll it times out if the deadline
#     has passed, else sleeps POLL_INTERVAL or the time left, whichever is
#     shorter, and times out with the last observed state when that sleep
#     reaches the deadline, making no further call. So one call of this script
#     lasts the max wait, plus at most the poll in flight when the deadline
#     passes
#   - Lists every run whose head is that commit with `gh run list --commit=<sha>`
#   - Polls until zero runs are queued / in_progress / waiting / requested /
#     pending (waiting includes waiting on an environment approval)
#   - Once all are completed, prints the results as a table, ending with
#     `STATUS=success|failed|timeout|no_runs`
#   - Exits 1 if even one run concluded failure / timed_out / startup_failure /
#     action_required / stale
#   - A run concluded canceled is treated as collateral from concurrency
#     (cancel-in-progress) when a newer successful run of the same workflow
#     exists on the default branch: it is not counted as a failure and shows as
#     `superseded` in the table. A canceled run with no later success is still
#     treated as a failure
#   - When the re-run is not yet complete, it withholds the verdict and keeps
#     polling (it starts seconds after the cancellation, so it is usually
#     incomplete at this point). But if there is a failure other than what it is
#     waiting on, STATUS will not change, so it exits 1 without waiting
#   - On a timeout while still awaiting a verdict, it still prints the table and
#     counts at that moment (with the pending canceled counted as a failure)
#   - If the gh call used for the superseded check fails, it warns on stderr and
#     leaves that run as canceled (i.e. a failure). The results table is always
#     printed
#   - Right after the table it prints TOTAL_COUNT / FAILED_COUNT (failures
#     excluding superseded; includes a canceled with no later success and a
#     canceled still awaiting a verdict) / SUPERSEDED_COUNT
#   - A timeout is exit 2
#   - no_runs (still zero after the initial grace, counted on the same clock
#     and ending by the max wait) is exit 0 with STATUS=no_runs
#   - A failure of `gh run list` itself is exit 3. gh's error output is not
#     captured but passed through to this script's stderr, followed by one line
#     containing the SHA
#   - Bad arguments or options are exit 4
#
# Call it by its relative path (the allow rules in `.claude/settings.json`
# assume `bash .claude/...`, so an absolute path produces a consent prompt).

sha=""

while [[ $# -gt 0 ]]; do
	case $1 in
	--max-wait=*)
		max_wait_flag="${1#*=}"
		shift
		;;
	-*)
		echo "error: unknown option: $1" >&2
		exit 4
		;;
	*)
		if [[ -n "$sha" ]]; then
			echo "error: unexpected argument: $1" >&2
			exit 4
		fi
		sha=$1
		shift
		;;
	esac
done

if [[ -z "$sha" ]]; then
	echo "error: merge commit SHA required" >&2
	exit 4
fi

poll_interval="${POLL_INTERVAL:-30}"
# The flag wins. MAX_WAIT is kept for backward compatibility.
# Using `-` (unset test) lets an empty --max-wait= reach the validation below,
# so it fails fast with exit 4 rather than silently taking the default.
max_wait="${max_wait_flag-${MAX_WAIT:-1800}}"
initial_grace="${INITIAL_GRACE:-60}"
# How many runs on the default branch the superseded check looks back through.
# The same workflow typically re-runs right after a cancellation, so 20 is
# enough.
supersede_limit=20

# Numeric validation (to avoid obscure errors from sleep / arithmetic
# expansion). Only poll_interval must be 1 or more (0 would busy-loop on
# sleep 0); the others allow 0 or more.
if [[ ! "$poll_interval" =~ ^[1-9][0-9]*$ ]]; then
	echo "error: POLL_INTERVAL must be a positive integer, got: $poll_interval" >&2
	exit 4
fi
if [[ ! "$max_wait" =~ ^[0-9]+$ ]]; then
	echo "error: --max-wait / MAX_WAIT must be a non-negative integer, got: $max_wait" >&2
	exit 4
fi
if [[ ! "$initial_grace" =~ ^[0-9]+$ ]]; then
	echo "error: INITIAL_GRACE must be a non-negative integer, got: $initial_grace" >&2
	exit 4
fi

# Print the table of completed runs and the counts. runs_json /
# superseded_json / total / failed / superseded must be set before calling.
function print_result() {
	echo "$runs_json" | jq -r --argjson sup "$superseded_json" '.[] | .workflowName as $name | (if .conclusion == "cancelled" and (($sup | index($name)) != null) then "superseded" else (.conclusion // "?") end) as $verdict | "\($verdict)\t\($name)\t\(.url)"' | sort
	echo "TOTAL_COUNT=$total"
	echo "FAILED_COUNT=$failed"
	echo "SUPERSEDED_COUNT=$superseded"
}

function clock() {
	date +%s
}

function no_runs() {
	echo "no workflow runs found for commit $sha after $((now - start))s (typical for chore-only merges)"
	echo "STATUS=no_runs"
	exit 0
}

# End the slice with the last observed state. pending / awaiting / total /
# runs_json and print_result's inputs must be set before calling.
function time_out() {
	echo "timeout after $((now - start))s; pending=$pending awaiting_supersede=$awaiting total=$total" >&2
	if [[ "$pending" -gt 0 ]]; then
		echo "$runs_json" | jq -r '.[] | select(.status == "queued" or .status == "in_progress" or .status == "waiting" or .status == "requested" or .status == "pending") | "\(.status)\t\(.workflowName)\t\(.url)"' | sort >&2
	else
		# Every run was complete and only the superseded check was pending.
		# Print the results with the pending canceled counted as a
		# failure.
		print_result
	fi
	echo "STATUS=timeout"
	exit 2
}

start=$(clock)
deadline=$((start + max_wait))
if [[ "$initial_grace" -lt "$max_wait" ]]; then
	grace_end=$((start + initial_grace))
else
	grace_end=$deadline
fi

while true; do
	# gh's stderr is not captured to a temp file but passed straight through to
	# this script's stderr (it survives in the caller's log, so nothing is lost).
	if ! raw_runs_json=$(gh run list --commit="$sha" --limit=1000 --json workflowName,status,conclusion,url,createdAt); then
		echo "failed to list workflow runs for commit $sha (see the line above for gh's error output)" >&2
		exit 3
	fi
	# A rerun leaves several runs per workflowName, so only the newest run per
	# workflowName (by createdAt descending) is counted. An older failed run
	# lingering does not stop STATUS=success when the newest succeeded.
	runs_json=$(echo "$raw_runs_json" | jq '[group_by(.workflowName)[] | max_by(.createdAt)]')
	total=$(echo "$runs_json" | jq 'length')

	if [[ "$total" -eq 0 ]]; then
		# GitHub can be slow to create the runs. Past the initial grace, treat it as no_runs.
		now=$(clock)
		if [[ "$now" -ge "$grace_end" ]]; then
			no_runs
		fi
		nap=$((grace_end - now))
		if [[ "$nap" -gt "$poll_interval" ]]; then
			nap=$poll_interval
		fi
		echo "[elapsed $((now - start))s] no runs yet; retrying in ${nap}s" >&2
		sleep "$nap"
		# The grace ends by the deadline, so a slice that reaches it has no runs.
		now=$(clock)
		if [[ "$now" -ge "$deadline" ]]; then
			no_runs
		fi
		continue
	fi

	pending=$(echo "$runs_json" | jq '[.[] | select(.status == "queued" or .status == "in_progress" or .status == "waiting" or .status == "requested" or .status == "pending")] | length')

	awaiting=0
	if [[ "$pending" -eq 0 ]]; then
		# Every run is complete. A canceled run is usually collateral from
		# concurrency (cancel-in-progress), so those with a later successful run
		# are separated out as superseded.
		superseded_json='[]'
		canceled_tsv=$(echo "$runs_json" | jq -r '.[] | select(.conclusion == "cancelled") | "\(.workflowName)\t\(.createdAt)"')
		# The results table is printed even if the gh call for the check fails. A
		# canceled run whose check was abandoned stays a failure. See the line
		# above for gh's error output.
		if [[ -n "$canceled_tsv" ]] && ! default_branch=$(gh repo view --json defaultBranchRef -q .defaultBranchRef.name); then
			echo "warning: failed to resolve default branch; treating canceled runs as failures" >&2
			canceled_tsv=""
		fi
		if [[ -n "$canceled_tsv" ]]; then
			while IFS=$'\t' read -r workflow_name run_created_at; do
				# Among the recent runs of the same workflow on the default
				# branch, look at those created after this one. Ancestry is not
				# checked (it loosens the judgment, and keeps this to gh
				# alone).
				if ! recent_json=$(gh run list --workflow="$workflow_name" --branch="$default_branch" --limit="$supersede_limit" --json status,conclusion,createdAt </dev/null); then
					echo "warning: failed to list recent runs of workflow '$workflow_name' on $default_branch; treating it as a failure" >&2
					continue
				fi
				newer_json=$(echo "$recent_json" | jq --arg t "$run_created_at" '[.[] | select(.createdAt > $t)]')
				newer_success=$(echo "$newer_json" | jq '[.[] | select(.conclusion == "success")] | length')
				if [[ "$newer_success" -gt 0 ]]; then
					superseded_json=$(echo "$superseded_json" | jq --arg n "$workflow_name" '. + [$n]')
					continue
				fi
				# The re-run starts seconds after the cancellation, so it is
				# usually incomplete at this point. Withhold the verdict and
				# wait for the next poll.
				newer_running=$(echo "$newer_json" | jq '[.[] | select(.status != "completed")] | length')
				if [[ "$newer_running" -gt 0 ]]; then
					awaiting=$((awaiting + 1))
				fi
			done <<<"$canceled_tsv"
		fi

		# A canceled run still awaiting a verdict is conservatively counted as a
		# failure (STATUS does not change even if the later run turns out
		# successful, so with a confirmed failure it finishes without waiting).
		superseded=$(echo "$superseded_json" | jq 'length')
		failed=$(echo "$runs_json" | jq --argjson sup "$superseded_json" '[.[] | .workflowName as $name | select(.conclusion == "failure" or .conclusion == "timed_out" or .conclusion == "startup_failure" or .conclusion == "action_required" or .conclusion == "stale" or (.conclusion == "cancelled" and (($sup | index($name)) == null)))] | length')
		if [[ "$awaiting" -eq 0 || "$failed" -gt "$awaiting" ]]; then
			print_result
			if [[ "$failed" -gt 0 ]]; then
				echo "STATUS=failed"
				exit 1
			fi
			echo "STATUS=success"
			exit 0
		fi
	fi

	now=$(clock)
	if [[ "$now" -ge "$deadline" ]]; then
		time_out
	fi

	nap=$((deadline - now))
	if [[ "$nap" -gt "$poll_interval" ]]; then
		nap=$poll_interval
	fi
	if [[ "$pending" -gt 0 ]]; then
		echo "[elapsed $((now - start))s] $pending/$total runs still in progress; retrying in ${nap}s" >&2
	else
		echo "[elapsed $((now - start))s] $awaiting canceled run(s) waiting for a newer run to finish; retrying in ${nap}s" >&2
	fi
	sleep "$nap"
	now=$(clock)
	if [[ "$now" -ge "$deadline" ]]; then
		time_out
	fi
done
