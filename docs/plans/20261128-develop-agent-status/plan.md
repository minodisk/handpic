<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Status report of the develop skill (tmp/agent-status.json)

## Purpose

Many develop sessions run in parallel under herdr, and the `wrangle` skill (a
herdr session supervisor that lives outside this repository) has to tell which
of them is waiting on a date or an external condition, which is working again,
and which is done. The screen alone does not show a session that stopped on
purpose and needs no answer, nor what a finished session left for the user to
do by hand, and a pane left with only a shell after develop's own `/exit` does
not tell a finished session from a crashed one.

This work has the develop skill's main agent write its state to
`tmp/agent-status.json` at the worktree root, under a contract shared with
`wrangle`. Once it is done, `wrangle` can read that file to leave a waiting
session alone until its resume condition holds, and to report a finished
session's leftovers before reaping its worktree.

## Steps

- [x] Step 1: Add the status report to the develop skill
  - Done when:
    - `.claude/skills/develop/references/agent-status.md` exists and holds the
      contract exactly as below: the path, every field with its type and when
      it is written, one JSON example per state that carries extra fields
      (`waiting` and `done`), when to write, and the writing rules
    - `.claude/skills/develop/SKILL.md` has a short status report section that
      says when to write each state, that only the main agent writes the file,
      that a failed write never stops develop, and that links
      `references/agent-status.md` as required reading before a write
    - The head of "## Phase 0" in SKILL.md has a one-line pointer: when this
      session already wrote `done`, overwrite it with `working` first
    - "## Finishing the run" in SKILL.md has a step that writes
      `"state": "done"` with `leftovers`, under the same condition as the step
      that ends the session, placed before the step that sends `/exit`
    - "## Notes" in SKILL.md lists writing the status file among the things
      the main agent does itself
    - `.claude/skills/develop/README.md` has a design note on why the file
      exists and why its contract must not change
    - `mise run ci:local` passes locally (it runs lychee because the branch
      changes Markdown) and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Docs only. Files: `.claude/skills/develop/references/agent-status.md`
      (new), `.claude/skills/develop/SKILL.md`,
      `.claude/skills/develop/README.md`. `.gitignore` already ignores `/tmp`
      (line 5), so it is not changed; `.claude/settings.json` already allows
      `Bash(date *)`, so it is not changed either. `CLAUDE.md` does not
      describe the develop skill's internals and is not changed
    - Write everything in English
    - **The contract is shared with the reader (`wrangle`). Keep the path, the
      field names and the state values exactly**, and say so in the reference
      file:
      - Path: `tmp/agent-status.json` at the worktree root, overwritten whole
        with the `Write` tool (no append, no partial update)
      - `state`: `"waiting"` | `"working"` | `"done"`, always
      - `summary`: string, 1–2 sentences, always
      - `waiting`: object, only when `state` is `waiting`:
        `{ "until": ISO datetime or who / what is awaited, "resume": the
        condition to resume }`
      - `plan`: the plan.md path, when there is one
      - `pr`: the PR number, when there is one
      - `leftovers`: array of strings, only when `state` is `done`: the work
        left on the user's side, `[]` when there is none (examples: a test
        folder to delete by hand, a post-merge CI result not yet seen, items
        that could not be verified on this OS)
      - `updated_at`: ISO 8601 UTC, always
    - When to write, to state in both files (SKILL.md in short, the reference
      in full):
      - Stopping to wait on a date or an external condition → `waiting`,
        written before stopping
      - Resuming after a `waiting` was written, or ending without resuming
        (abort, merged by another session) → overwrite with `working`, so a
        stale `waiting` does not make the reader take the session for one to
        resume
      - Everything asked in the session is done → `done`, under the same
        condition as the session-ending step of "## Finishing the run",
        written before `/exit` is sent, with or without `HERDR_ENV=1`
      - A new develop starting in the same session after a `done` → overwrite
        with `working`
      - Stopped on a failure or an abort (with no earlier `waiting`), waiting
        on a decision, or simply working → do not write
    - Rules to state: only the main agent writes the file and subagents never
      do; a failed write never stops develop and is not reported as an error;
      no workaround such as retrying with `dangerouslyDisableSandbox` or
      anything that raises a permission prompt
    - Writing mechanics worth carrying over from the kwcp reference
      (`.claude/skills/develop/references/agent-status.md` of the kwcp
      repository, Japanese, read only): take `updated_at` from the output of
      `date -u +%Y-%m-%dT%H:%M:%SZ` and type it in literally; do not write the
      file through a Bash redirect or the herdr CLI; when `Write` refuses
      because the existing file was not read, `Read` it and write once more,
      then carry on whatever the result; after a compact, `Read` the file
      first when unsure what the session is waiting for. That reference
      covers only `waiting` / `working` and says not to write on completion,
      so the `done` state, the `leftovers` field and the "new develop after
      `done`" rule are written new here, not translated
    - SKILL.md layout, following kwcp: one short section (for example
      `## Status report`) near "## Finishing the run" / "## Reporting", with
      the when-to-write bullets and a bold instruction to `Read`
      `references/agent-status.md` before writing. Link it the way SKILL.md
      links `references/pr-merge-lifecycle.md`
    - "## Phase 0": add the one-line pointer to the status report section at
      its head
    - "## Finishing the run": insert the `done` write as its own numbered step
      between the report of the decisions (step 1) and the session end (the
      current step 2), and renumber. Its condition is the session-end step's
      ("everything the user asked for in this session is done"), and the
      current step 4 (no session end on a failure state or with another
      request pending) applies to it the same way. The `/exit` step stays the
      last tool call of the turn. Check that no text in SKILL.md, README.md or
      `.claude/agents/` refers to a step of "## Finishing the run" by number
      before renumbering
    - Tell the main agent where `leftovers` come from: what the run itself
      reported as left for the user (manual checks of 3.3 / S.5 that could not
      be run on this OS, a `POST_MERGE_TIMEOUT` whose CI result was not seen,
      files or folders it created outside the repository and did not remove)
    - "## Notes": add writing the status file (`tmp/agent-status.json`) to the
      "Exceptions, all done by the main agent" bullet
    - README.md: add a `###` design note under "## Design notes", next to
      "### Decide by recommendation, report at the end", which already
      describes the session end. Say that the contract is shared with
      `wrangle`, that the file is write-only from develop's side (develop
      behaves the same with no reader), that the reader judges freshness by
      `updated_at`, and why a failed write is ignored
    - lychee runs with `--include-fragments` over `.claude/**/*.md`, so every
      new relative link and anchor has to resolve

## Trade-offs and risks

- **Writing `done` outside herdr (decided: write it).** The contract ties
  `done` to the condition of the session-ending step, which holds with or
  without `HERDR_ENV=1`. One rule, and a file nobody reads does no harm.
- **Mirroring the resume condition into auto memory (decided: no).** kwcp also
  writes the resume condition of a `waiting` into the in-flight plan entry of
  `MEMORY.md`. The contract given for this work does not include it; the
  reference file's "read the status file first after a compact" covers the
  same need.
- **Where the "new develop after `done`" rule lives (decided: also a pointer
  at the head of Phase 0).** The agent reads Phase 0 long before the status
  report section, so a pointer there keeps the overwrite with `working` from
  being missed.
- **A new numbered step versus folding into the session-end step (decided: a
  new step).** Easier to see and to check; the steps after it renumber.
- **Stale files.** A session that does not use develop in the same worktree,
  or a failed overwrite after a resume, leaves old content. The reader has to
  judge freshness by `updated_at`; develop does not clean the file up.
- **Divergence from kwcp.** kwcp's develop is being extended with `done` by
  another session; kwcp's develop and the `wrangle` skill are out of scope
  here.

## Progress
