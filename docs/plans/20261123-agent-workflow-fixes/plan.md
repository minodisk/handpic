<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Agent workflow fixes

## Purpose

Five `todo.md` items record places where the agent workflow itself failed or
wasted a run: archived plan scripts whose relative paths went stale, a plan
link that broke lint on main once its target was archived, `python` / `node`
misbehaving in Git Bash on the Windows host, foreground commands the harness
moves to the background after about 180 s, a `local-review-runner` that
returned before its reviewer had, and a `pr-runner` that aborted on a short
`api.github.com` outage. Once this is done, each of those has a written rule
or a script behavior that prevents it, and the five items are gone from
`todo.md`.

## Steps

- [x] Step 1: Close the five agent-workflow todo items
  - Done when:
    - Archived plan scripts: the archive convention states that a plan
      folder's scripts are frozen as run and how to import them after the
      move.
    - Plan links: the plan-writing instructions tell the author to link
      another plan only at an archived or otherwise stable path, to name a
      plan still in flight in backticks, and to re-run the link check after
      rebasing onto a main where plan folders moved.
    - `python` / `node`: `CLAUDE.md` Development carries the note to run them
      through `mise x --` on the Windows host.
    - Backgrounded `ci:local`: `.claude/agents/implementer.md` and
      `.claude/agents/local-review-addresser.md` both say what to do when the
      Bash tool moves the run to the background on its own.
    - PR wait scripts: each foreground wait slice that `pr-runner` and
      `merger` run returns by itself in under 180 s of wall time, and the
      slice-count limits are rescaled so the total waits stay about 30
      minutes.
    - `local-review-runner`: the definition makes the runner wait for the
      reviewer's (and the addresser's) hand-back and refuses to decide or
      report while the round's section is missing from the review file.
    - `api.github.com` outage: `wait-pr-actionable.sh` retries a failed
      `pr-status.sh` inside the slice, and
      `wait-pr-actionable.test.sh` has cases for "fails, then succeeds"
      (exit 0 with the ACTION) and "fails through the whole slice" (exit 1).
    - The five headings are gone from `todo.md` and nothing else in it
      changed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Compatibility: other sessions run `/develop` and `/pr` from main while
      this lands. Rename no agent, script, terminal state, exit code or
      `ACTION=` value. Keep the script defaults (`wait-pr-actionable.sh`
      `max_wait_sec=240`, `wait-post-merge-runs.sh` `MAX_WAIT`) as they are,
      so a run holding an older definition still gets the slices its
      counters assume; shorten the slice by the arguments the definitions
      pass.
    - `todo.md`: delete exactly these five items, heading through the last
      line of the item, and nothing else (the file is edited concurrently):
      "Agents: the archived plan scripts' import paths broke when their
      folders moved", "Agents: a plan linking a sibling plan in flight breaks
      lint on main once the sibling is archived", "Agents: bare `python` /
      `node` hang in Git Bash on the Windows host", "Agents: a cold Rust
      `ci:local` is moved to the background by the Bash tool after 180 s",
      "Agents: `local-review-runner` returns before its reviewer finishes,
      and `pr-runner` aborts on a transient `api.github.com` outage". The
      `python` / `node` item is followed by a `---` line; check what it
      separates before deciding whether it goes with the item. Re-read the
      file right before editing, since line numbers move.
    - Item "archived plan scripts" (the convention, not a path fix).
      Facts found at planning time:
      `folder_gate.py`, `ballgate.py` and `genre_offsets.py` insert
      `<folder>/../_archived/20261012-image-embeddings`, which is now
      `_archived/_archived/...`; `folder_gate.py`'s `GENRES` points at
      `crates/core/genres/siglip2.json`, which no longer exists anywhere
      (removed in #1205), so a path fix cannot make it runnable alone;
      `docs/plans/_archived/20261015-folder-wide-ranking-eval/folderwide.py`
      has the same stale `HERE/../_archived` hop since its own archive move,
      so the convention must describe the mechanism (put the needed plan
      folders on `sys.path` yourself, import `gate` first, override
      `GENRES`) and must not cite `folderwide.py` as a working example of
      the paths. Place: `.claude/skills/develop/SKILL.md`, "3.5 Archiving
      the plan" (the archive move), a short paragraph: scripts in a plan
      folder are records of a run, the move rewrites none of their relative
      paths, they are not maintained afterwards. Check whether S.5 (the
      single-PR archive) needs a pointer to it.
    - Item "plan links": `.claude/agents/planner.md`, in the plan.md
      template's `<plan-guide>` next to the existing lychee line (every new
      plan.md then carries it to the implementer), plus one sentence in the
      planner's own writing guidance. Content: a plan folder moves under
      `_archived/` when it completes, so link only
      `../_archived/<folder>/...` or a stable non-plan path; write a plan
      still in flight as a backticked path; after rebasing onto a main where
      plan folders moved, re-run the link check (`mise run ci:local` runs
      lychee when the branch changed Markdown; `mise run lint` always does).
      Do not edit the copies of the guide inside existing plan.md files.
    - Item "`python` / `node`": one short line under `CLAUDE.md`
      Development. Checked at planning time: neither the user-level
      `CLAUDE.md` nor the auto memory index carries the note. Measured in
      the planning session's Git Bash on this host (2026-11-23):
      `mise x -- node --version` and `mise x -- python --version` return at
      once; a bare `node` is not on `PATH` (exit 127), a bare `python`
      resolves to a per-user install and ran. The hang was not reproduced,
      so word the note as "run them through `mise x --`; a bare invocation
      can hang or be missing in Git Bash", not as a guaranteed hang.
    - Item "backgrounded `ci:local`", first checkbox:
      `.claude/agents/implementer.md` step 7 and
      `.claude/agents/local-review-addresser.md` step 5, right after the
      "Do not set `run_in_background: true`" sentence. Content: the rule is
      about not choosing `run_in_background` yourself; if the harness
      reports it moved the command to the background, do not re-run it, do
      not poll with `sleep` / `until`, wait for its completion notification
      and take the exit code and output from that; the run counts once
      toward the 3 attempts. No cold reproduction is run (the user approved
      writing the conditional text without one); record in `learnings.md`
      what the harness's message says and how the result of a backgrounded
      command reaches a leaf subagent if it is observed during this work.
    - Item "backgrounded `ci:local`", second checkbox (wait scripts):
      `.claude/agents/pr-runner.md` and `.claude/agents/merger.md`. Pass an
      explicit shorter slice in the command blocks
      (`wait-pr-actionable.sh <PR> <interval> <max_wait>`, e.g. `30 120`;
      `wait-post-merge-runs.sh --max-wait=<n>`), which the existing
      `Bash(bash .claude/skills/pr/scripts/*)` /
      `.../merge/scripts/*` allow rules already cover. `max_wait_sec` is the
      sum of the sleeps and `pr-status.sh`'s runtime adds on top, so measure
      one slice's wall time and pick a value that lands clearly under
      180 s. Rescale every limit counted in slices so wall-clock totals
      stay: `wait_timeouts < 7`, `review_required_timeouts < 2` / `>= 2`
      ("roughly 8-10 minutes stalled") in `pr-runner.md`;
      `pr_wait_timeouts < 7` and `post_merge_timeouts < 7` in `merger.md`;
      and the prose that says "4 minutes", "4-minute", "240-second",
      "about 30 minutes" (also the header comments of
      `wait-pr-actionable.sh` and `wait-post-merge-runs.sh` where they
      describe what agent callers pass, and
      `.claude/skills/develop/README.md` if it repeats the numbers). Keep
      the existing "if the harness nonetheless reports ... moved to the
      background" paragraphs as the fallback.
    - Item "`local-review-runner`", first checkbox:
      `.claude/agents/local-review-runner.md`. Mirror `pr-runner.md`'s
      "Dispatching a child and waiting for its hand-back" (added in #666):
      an `Agent` launch can return at once as an async launch whatever
      `run_in_background` says; after dispatching the reviewer or the
      addresser, end the turn and run nothing until the hand-back arrives;
      ending the turn is not a report. Add the detection to "2. Decide":
      decide only when the review file's latest section is
      `## Round {round_n}` with a `STATUS:` line; a missing section means
      the reviewer has not finished, so neither decide, commit the history
      nor report. Keep the terminal states (`APPROVED` /
      `MAX_ROUNDS_REACHED` / `LOCAL_CHECK_FAILED`) and the input contract
      unchanged; the `develop` skill's watchdog
      (`watch-local-review.sh`) depends on the push of the review history
      and is not touched.
    - Item "`api.github.com` outage", second checkbox:
      `.claude/skills/pr/scripts/wait-pr-actionable.sh` currently does
      `out=$(bash pr-status.sh) || exit 1`, and `pr-runner` re-runs at once
      up to 5 times, so an outage of a few seconds exhausts
      `pr_status_failures`. Treat a failed `pr-status.sh` inside the loop
      like a waiting state: report it on stderr, sleep the interval, count
      it toward the elapsed time, and exit 1 only when the slice ends with
      the last call still failing (exit codes 0 / 1 / 2 keep their
      meaning; argument errors still exit 1 before the loop). One exit 1
      then stands for a slice-long outage, and `pr_status_failures > 5` for
      several slices. Decide what a failed call does to the `blocked`
      grace (`blocked_since`) and pin it in a test. Extend the `gh` stub of
      `wait-pr-actionable.test.sh` with a failing fixture. Update the
      exit-1 bullets of `pr-runner.md` and `merger.md` to say that the
      script has already retried within the slice. `merger` shares the
      script and gets the same tolerance; `wait-post-merge-runs.sh`'s
      exit 3 is not part of this item.
    - `tools/ci/local.sh` selects lychee for the Markdown, shellcheck for
      the scripts and `wait-pr-actionable.test.sh` for a change to
      `wait-pr-actionable.sh` / `pr-status.sh`, so `mise run ci:local`
      covers every file this step changes.

## Trade-offs and risks

- One step, approved by the user. The riskier half is the wait loop (slice
  length, rescaled counters in `pr-runner.md` / `merger.md`, the in-script
  retry); the rest is additive text.
- Archived scripts: the convention is stated instead of fixing the paths,
  because `folder_gate.py` stays unrunnable after a path fix (its genre file
  was removed from the repository), the scripts need a private dataset
  anyway, and the next archive move would break the next script the same
  way.
- The 180 s backgrounding is not reproduced with a cold `ci:local`; the rule
  is conditional on the harness's report, so it is correct either way.
- The backgrounding rule goes into the two definitions the todo names.
  `pr-check-fixer.md`, `pr-conflict-resolver.md`, `pr-review-addresser.md`,
  `pr-runner.md` and `local-review-runner.md` carry the same "do not set
  `run_in_background`" sentence and are left as they are.
- Shorter wait slices are passed as arguments, keeping the script defaults,
  so runs in flight with an older definition are unaffected.
- In-script retry makes a persistent error (expired auth, a missing PR) take
  several slices to end in `pr_status_failure` instead of seconds.
- The wait-for-hand-back rule in `local-review-runner.md` relies on the same
  mechanism as `pr-runner.md`'s, whose confirmation on a real `/pr` run is
  still an open todo item (out of scope). The review-file check in
  "2. Decide" is what keeps the runner from reporting early if it does not
  hold.
- Seen while investigating, not part of this plan:
  `.claude/skills/develop/scripts/commit-settings.sh`,
  `.claude/skills/merge-settings/SKILL.md` and `merge.test.sh` call a bare
  `node`, which was not on `PATH` in the planning session's Git Bash.

## Progress
