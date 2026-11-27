# Status report (tmp/agent-status.json)

The contract of the file the develop skill's main agent writes its state to,
and how to write it. The short version of when to write each state, and the
rules that only the main agent writes it and that a failed write never stops
develop, are in SKILL.md's
[`## Status report`](../SKILL.md#status-report) section. The design reasons
are in README.md's
[`### The status report (tmp/agent-status.json)`](../README.md#the-status-report-tmpagent-statusjson).

**This contract is shared with the reader (the `wrangle` skill, which lives
outside this repository). Do not change the path, the field names or the
state values.**

## Contract

- Path: `tmp/agent-status.json` at the worktree root (`/tmp` is in
  `.gitignore`)
- Overwrite it whole with the `Write` tool. No append, no partial update

### Fields

| Field        | Type             | When it is written           | Content                                                                    |
| ------------ | ---------------- | ---------------------------- | -------------------------------------------------------------------------- |
| `state`      | string           | always                       | `"waiting"`, `"working"` or `"done"`                                       |
| `summary`    | string           | always                       | the current state in 1–2 sentences                                         |
| `waiting`    | object           | only when `state` is waiting | `{ "until": ISO datetime or who / what is awaited, "resume": the condition to resume }` |
| `plan`       | string           | when there is one            | the plan.md path                                                           |
| `pr`         | number           | when there is one            | the PR number                                                              |
| `leftovers`  | array of strings | only when `state` is done    | the work left on the user's side, `[]` when there is none                  |
| `updated_at` | string           | always                       | ISO 8601, UTC                                                              |

### Examples

`waiting`:

```json
{
  "state": "waiting",
  "summary": "PR #1234 of Step 4 is waiting to be merged. The user asked for the merge on or after 11-21.",
  "waiting": {
    "until": "2026-11-21T00:00:00Z",
    "resume": "Start merger on or after 11-21, on the user's word"
  },
  "plan": "docs/plans/20261119-foo/plan.md",
  "pr": 1234,
  "updated_at": "2026-11-19T03:04:05Z"
}
```

`done`:

```json
{
  "state": "done",
  "summary": "Every step of the plan is merged and the wrap-up PR #1240 is merged.",
  "plan": "docs/plans/_archived/20261119-foo/plan.md",
  "pr": 1240,
  "leftovers": [
    "Check the strip's drag on macOS; it could not be run on this Windows host",
    "Delete the test folder C:/Users/me/Pictures/handpic-test by hand"
  ],
  "updated_at": "2026-11-22T08:09:10Z"
}
```

## When to write

- **Stopping to wait on a date or an external condition**: when the user stops
  the run until a date or an external condition ("merge on or after 11-21",
  "wait for the import to finish"). Write `"state": "waiting"` before stopping
  - Put the date (an ISO datetime) or who / what is awaited in
    `waiting.until`, and the condition to resume in `waiting.resume`
- **Resuming after a `waiting` was written**: overwrite the file whole with
  `"state": "working"` and `summary` (with `updated_at`, and `plan` / `pr`
  when there are any). A stale `waiting` would make the reader take the
  session for one still waiting. An ordinary resume with no `waiting` written
  (after an `AskUserQuestion` answer, say) writes nothing
- **Ending without resuming after a `waiting` was written** (an abort, a PR
  merged by another session's `/merge`): before ending, overwrite with
  `"state": "working"` and a `summary` saying it was aborted or finished. A
  `waiting` left behind would, once its date passes, make the reader take the
  session for one to resume. When everything asked in the session is done,
  the `done` below replaces it instead
- **Everything asked in the session is done**: write `"state": "done"` with
  `leftovers`, under the same condition as the session-ending step of SKILL.md's
  `## Finishing the run` (everything the user asked for in this session is
  done; not when the run stopped on a failure state or another request of the
  session is still pending), before `/exit` is sent. Write it with or without
  `HERDR_ENV=1`
  - `leftovers` holds what the run itself reported as left for the user:
    manual checks of 3.3 / S.5 that could not be run on this OS, a
    `POST_MERGE_TIMEOUT` whose CI result was not seen, files or folders the
    run created outside the repository and did not remove. `[]` when there
    is none
- **A new develop starting in the same session after a `done`**: overwrite
  with `"state": "working"` at the start of Phase 0, so the reader does not
  reap a session that is working again
- **Anything else**: stopped on a failure or an abort with no earlier
  `waiting`, waiting on a decision, or simply working: do not write. The
  reader can take those from the screen

## Writing rules

- Overwrite the file whole with the `Write` tool. Do not write it through a
  Bash redirect or the herdr CLI (the herdr CLI fails under the sandbox with
  `Operation not permitted` and brings up a permission prompt; reflecting the
  state into herdr is the reader's job)
- Take `updated_at` from the output of `date -u +%Y-%m-%dT%H:%M:%SZ` and type
  it in literally (`Bash(date *)` is allowed; do not put it in a shell
  variable)
- When `Write` refuses because the existing file was not read, `Read` it and
  write once more. If that fails too, carry on without reporting it
- A failed write never stops develop and is not treated as an error. No
  workaround such as re-running with `dangerouslyDisableSandbox` or anything
  that raises a permission prompt
- Only the main agent writes the file. Subagents never do, and no agent's
  prompt mentions it
- After a compact, or whenever unsure what the session is waiting for, `Read`
  `tmp/agent-status.json` first
