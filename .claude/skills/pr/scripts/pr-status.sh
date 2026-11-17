#!/usr/bin/env bash
# Fetch the PR's current state in one command and decide the next action.
# Output format: KEY=VALUE lines
#   ACTION: wait | conflict | changes_requested | check_failed | review_required | ready | draft | behind | blocked | merged | closed
#   others: mergeable, mergeStateStatus, reviewDecision, checksOverall, failedCount, pendingCount, failedRunIds, headSha
#   unresolvedThreads is counted and printed only when reviewDecision=CHANGES_REQUESTED.
#     An empty value means "the fetch failed" (not "zero threads")
#   state is printed additionally only when merged/closed
#
# `changes_requested` is returned only when there is genuinely something for the
# agent to act on. When all that remains is a CHANGES_REQUESTED from the
# automated approver and there are zero unresolved threads, there is nothing to
# act on and starting review handling would be a no-op, so it returns
# `review_required` — waiting for the approver to re-evaluate.
set -euo pipefail

PR="${1:?usage: pr-status.sh <PR_number_or_url>}"
# The automated approver's login. A human CHANGES_REQUESTED is always returned
# as changes_requested (it is never re-evaluated, so the agent has to handle it).
# Leave it empty in a repository with no automated approver (in which case no
# approval is ever attributed to an approver).
APPROVER_LOGIN="${PR_APPROVER_LOGIN:-}"

stderr_file=$(mktemp "${TMPDIR:-/tmp}/pr-status.XXXXXX")
trap 'rm -f "$stderr_file"' EXIT

# Get the CHANGES_REQUESTED breakdown in a single GraphQL call. Prints
# "<number of human CHANGES_REQUESTED> <number of unresolved threads>" to stdout
# and returns non-zero on failure.
#
# owner/repo comes from the target PR's URL (deriving it from the current
# repository would count a different PR of the same number when given a PR URL
# from another repository).
#
# Human CHANGES_REQUESTED is judged from latestOpinionatedReviews (each author's
# latest opinionated review; COMMENTED and DISMISSED excluded). `gh pr view
# --json` truncates reviews at 100 with no pagination, so on a PR with more than
# 100 reviews it drops human feedback (erring dangerously). GraphQL has no such
# limit.
fetch_review_signals() {
	local owner="$1" repo="$2" pr_number="$3" approver="$4"
	local cursor="" unresolved_total=0 human_cr="" page unresolved has_next

	# shellcheck disable=SC2016
	local query='
	  query($owner:String!,$repo:String!,$number:Int!,$cursor:String) {
	    repository(owner:$owner, name:$repo) {
	      pullRequest(number:$number) {
	        latestOpinionatedReviews(first:100) {
	          nodes { state author { login } }
	        }
	        reviewThreads(first:100, after:$cursor) {
	          pageInfo { hasNextPage endCursor }
	          nodes { isResolved }
	        }
	      }
	    }
	  }'

	while :; do
		local args=(-f query="$query" -F owner="$owner" -F repo="$repo" -F number="$pr_number")
		if [[ -n "$cursor" ]]; then
			args+=(-F cursor="$cursor")
		fi
		page=$(gh api graphql "${args[@]}") || return 1

		# gh can exit 0 with a response in an unexpected shape (data:null on a
		# permission error, say). jq does not error when keying into null, so
		# the aggregation below with `// []` would silently become "zero".
		# Reject it explicitly.
		jq -e '.data.repository.pullRequest != null' >/dev/null <<<"$page" || return 1

		# The review list is not paginated, so count it on the first page only.
		# A login can pick up a [bot] suffix depending on the API route, so
		# strip it before comparing.
		if [[ -z "$human_cr" ]]; then
			human_cr=$(jq -r --arg approver "$approver" '
			  [(.data.repository.pullRequest.latestOpinionatedReviews.nodes // [])[]
			   | select(.state == "CHANGES_REQUESTED")
			   | select(((.author.login // "") | sub("\\[bot\\]$"; "")) != $approver)
			  ] | length
			' <<<"$page") || return 1
		fi

		unresolved=$(jq '[(.data.repository.pullRequest.reviewThreads.nodes // [])[] | select(.isResolved == false)] | length' <<<"$page") || return 1
		unresolved_total=$((unresolved_total + unresolved))

		has_next=$(jq -r '.data.repository.pullRequest.reviewThreads.pageInfo.hasNextPage' <<<"$page") || return 1
		if [[ "$has_next" != "true" ]]; then
			break
		fi
		cursor=$(jq -r '.data.repository.pullRequest.reviewThreads.pageInfo.endCursor' <<<"$page") || return 1
	done

	printf '%s %s' "$human_cr" "$unresolved_total"
}

if ! view_json=$(gh pr view "$PR" --json state,mergeable,mergeStateStatus,reviewDecision,headRefOid,statusCheckRollup,isDraft,number,url 2>"$stderr_file"); then
	echo "pr-status.sh: failed to fetch PR '$PR':" >&2
	cat "$stderr_file" >&2
	exit 1
fi
if [[ -s "$stderr_file" ]]; then
	cat "$stderr_file" >&2
fi

state=$(jq -r '.state // ""' <<<"$view_json")
if [[ "$state" == "MERGED" ]]; then
	echo "ACTION=merged"
	echo "state=$state"
	exit 0
fi
if [[ "$state" == "CLOSED" ]]; then
	echo "ACTION=closed"
	echo "state=$state"
	exit 0
fi

mergeable=$(jq -r '.mergeable // ""' <<<"$view_json")
mergeStateStatus=$(jq -r '.mergeStateStatus // ""' <<<"$view_json")
reviewDecision=$(jq -r '.reviewDecision // ""' <<<"$view_json")
headSha=$(jq -r '.headRefOid // ""' <<<"$view_json")
isDraft=$(jq -r '.isDraft // false' <<<"$view_json")

# Aggregate the failed and in-progress counts from statusCheckRollup
normalize='
  (.statusCheckRollup // [])
  | map(
      if .__typename == "CheckRun" then
        (if .status == "COMPLETED" then .conclusion else .status end)
      else
        .state
      end
    )
'
failed_selector='. == "FAILURE" or . == "TIMED_OUT" or . == "CANCELLED" or . == "ACTION_REQUIRED" or . == "ERROR" or . == "STARTUP_FAILURE" or . == "STALE"'
pending_selector='. == "QUEUED" or . == "IN_PROGRESS" or . == "PENDING" or . == "WAITING" or . == "REQUESTED" or . == "EXPECTED"'
failed_count=$(jq -r "$normalize | map(select($failed_selector)) | length" <<<"$view_json")
pending_count=$(jq -r "$normalize | map(select($pending_selector)) | length" <<<"$view_json")

failed_run_ids=$(jq -r '
  [(.statusCheckRollup // [])[]
   | select(.__typename == "CheckRun")
   | select(.conclusion == "FAILURE" or .conclusion == "TIMED_OUT" or .conclusion == "CANCELLED" or .conclusion == "ACTION_REQUIRED" or .conclusion == "ERROR" or .conclusion == "STARTUP_FAILURE" or .conclusion == "STALE")
   | (.detailsUrl // "")
   | select(test("/runs/[0-9]+"))
   | capture("/runs/(?<id>[0-9]+)")
   | .id
  ] | join(",")
' <<<"$view_json")

# "name<TAB>url" for every failed check (non-GitHub-Actions included), newline separated.
failed_checks=$(jq -r '
  [(.statusCheckRollup // [])[]
   | if .__typename == "CheckRun" then
       select(.conclusion == "FAILURE" or .conclusion == "TIMED_OUT" or .conclusion == "CANCELLED" or .conclusion == "ACTION_REQUIRED" or .conclusion == "ERROR" or .conclusion == "STARTUP_FAILURE" or .conclusion == "STALE")
       | "\(.name // "?")\t\(.detailsUrl // "")"
     else
       select(.state == "FAILURE" or .state == "ERROR")
       | "\(.context // "?")\t\(.targetUrl // "")"
     end
  ] | join("\n")
' <<<"$view_json")

# Only on CHANGES_REQUESTED, look up the unresolved thread count and whether any
# CHANGES_REQUESTED came from a human (querying unconditionally would hit
# GraphQL on every poll, so it is narrowed to when it is needed).
unresolved_threads=""
human_changes_requested=0
if [[ "$reviewDecision" == "CHANGES_REQUESTED" ]]; then
	pr_number=$(jq -r '.number' <<<"$view_json")
	# url is of the form https://github.com/<owner>/<repo>/pull/<number>
	pr_owner=$(jq -r '.url | split("/")[3] // ""' <<<"$view_json")
	pr_repo=$(jq -r '.url | split("/")[4] // ""' <<<"$view_json")

	# On a fetch failure, leave unresolved_threads empty. The check below is
	# `== "0"`, so empty = nothing to judge on = changes_requested as before.
	if signals=$(fetch_review_signals "$pr_owner" "$pr_repo" "$pr_number" "$APPROVER_LOGIN"); then
		human_changes_requested="${signals%% *}"
		unresolved_threads="${signals##* }"
	fi
fi

if [[ "$failed_count" -gt 0 ]]; then
	checksOverall=failure
elif [[ "$pending_count" -gt 0 ]]; then
	checksOverall=pending
else
	checksOverall=success
fi

# Decide the action (in priority order)
if [[ "$mergeable" == "CONFLICTING" || "$mergeStateStatus" == "DIRTY" ]]; then
	action=conflict
elif [[ "$checksOverall" == "failure" ]]; then
	action=check_failed
elif [[ "$reviewDecision" == "CHANGES_REQUESTED" ]]; then
	# If all that remains is the automated approver's verdict and there are zero
	# unresolved threads, there is nothing to act on. Starting review handling
	# would be a no-op, so treat it as waiting for the approver to re-evaluate.
	# A human CHANGES_REQUESTED, or a failure to fetch the thread count
	# (unresolved_threads empty), still returns changes_requested as before.
	if [[ "$human_changes_requested" -eq 0 && "$unresolved_threads" == "0" ]]; then
		action=review_required
	else
		action=changes_requested
	fi
elif [[ "$checksOverall" == "pending" ]]; then
	action="wait"
elif [[ "$reviewDecision" == "REVIEW_REQUIRED" ]]; then
	action=review_required
elif [[ "$isDraft" == "true" || "$mergeStateStatus" == "DRAFT" ]]; then
	# A draft PR is a terminal state, never counted as ready even with clean checks/conflicts
	action=draft
elif [[ "$mergeStateStatus" == "CLEAN" || "$mergeStateStatus" == "UNSTABLE" || "$mergeStateStatus" == "HAS_HOOKS" ]]; then
	action=ready
elif [[ "$mergeStateStatus" == "BEHIND" ]]; then
	# BEHIND means it is waiting on a merge from main; the caller directs a merge/rebase as needed
	action=behind
elif [[ "$mergeStateStatus" == "BLOCKED" ]]; then
	# BLOCKED means a branch protection rule is stopping it, so manual intervention is needed
	action=blocked
else
	# Wait and see on an unknown state (though the cases above cover all the main ones)
	action="wait"
fi

cat <<EOF
ACTION=$action
mergeable=$mergeable
mergeStateStatus=$mergeStateStatus
reviewDecision=$reviewDecision
checksOverall=$checksOverall
failedCount=$failed_count
pendingCount=$pending_count
failedRunIds=$failed_run_ids
headSha=$headSha
EOF

if [[ "$reviewDecision" == "CHANGES_REQUESTED" ]]; then
	echo "unresolvedThreads=$unresolved_threads"
fi

if [[ -n "$failed_checks" ]]; then
	echo "---failedChecks (name<TAB>url; no run ID for non-Actions checks)---" >&2
	printf '%s\n' "$failed_checks" >&2
fi
