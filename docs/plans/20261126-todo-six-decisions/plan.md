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

# Record six "do not change the code" decisions and close their todo items

## Purpose

Six `todo.md` items ask for a decision rather than a fix. The user decided
all six on 2026-11-26, each as "do not change the code". This plan records
each decision and its reason where an agent will find it, removes the items
from `todo.md`, and adds a short revert caution to the `develop` skill. It is
docs only, in one PR.

The decisions:

1. `### Core: decide whether a bad-photo cue shown to the human replaces the removal filter`.
   The cue shown to the human already exists: the strip's Low mark, the gray
   frowning face (`crates/app/src/mark.rs`, the judged face's
   `full_eye_sharp` under `LOW_FULL_EYE_SHARP` = 0.110). No removal filter is
   pursued. The item closes without measuring Low's precision.
2. `### App: decide the scan's read cap now that NVMe was measured`.
   `READ_SLOTS` stays 2 on every drive. On NVMe cap 2 makes pass 1 slower,
   but pass 1 is fast there to begin with, so the cost is small. What the
   user feels is the preview wait on SATA, and cap 2 protects it.
3. `### Core: mesh the faces under EYES_MIN_FACE from the full JPEG so the all-faces counts cover them (all-faces-check)`.
   Not done. The meta pane's `Faces judged` row already tells how many faces
   could be judged, and the gain is small against the labeling and the
   threshold re-fit it needs.
4. `### Core: the gate's size band lets a background row at the player's size through (all-faces-check)`.
   No further rule; the cost is accepted. What is counted is correct as the
   faces in that photo. Only pass 3's cost grows, which is not worth a
   `FACES_VERSION` bump and a pass 2 re-run.
5. In `### Core: widen camera support from public sample RAW files`, the
   Tier 3 TODO line only. No new RAW format for now. The major formats are
   supported and Pentax bodies can write DNG. Reconsidered on request. The
   section's other TODO lines stay.
6. `### Develop: check intervening commits before restoring a file from an old commit in a revert (revert-page-present-now)`.
   The caution goes into `.claude/skills/develop/SKILL.md`, briefly.

## Steps

- [x] Step 1: Record the six decisions, add the revert caution, remove the todo items
  - Done when:
    - `docs/agents/shared-crates.md`, section "The scan's reads are capped
      and yield to the preview's at the disk (Measured)": the end of the NVMe
      bullet no longer says the cap is "an open decision". It states decision
      2 with its reason and the date (user decision, 2026-11-26): one cap of
      2 on every drive, no per-drive cap. The measured numbers and both
      tables are unchanged.
    - `docs/agents/shared-crates.md`, section "The good-photo mark's rule is
      validated against the user's stars; know what it can claim (Hit)": one
      new bullet states decision 1 (user decision, 2026-11-26). It says the
      Low mark is the cue shown to the human, no removal filter is pursued,
      Low's precision against the bad-photo labels is deliberately
      unmeasured, and names
      `docs/plans/_archived/20261017-bad-photo-labels/plan.md` (Decision) as
      the source.
    - `docs/plans/_archived/20261017-bad-photo-labels/plan.md`, section
      "Decision (...)", directly after the "Left for the user to decide"
      list: a dated note answers the first bullet (removal filter or cue)
      with decision 1. The second bullet (a second labeling round) is left
      as it is.
    - `docs/plans/_archived/20261123-all-faces-check/plan.md`, section
      "Decisions": a new block at the end, "Made by the user on 2026-11-26,
      after the plan was archived:", with two bullets, decision 3 and
      decision 4, each with its reason. The existing blocks and the
      "Trade-offs and risks" bullets are not rewritten.
    - `docs/agents/raw-metadata-parsing.md`: a short paragraph in the
      introduction, after the paragraph that lists the parsers and before
      the one about the tags, states decision 5 (user decision, 2026-11-26):
      no parser for a further RAW container for now, the reason, and that it
      is reconsidered on request.
    - `.claude/skills/develop/SKILL.md`, section "2.2 Implementation": a
      short paragraph says that when a step reverts an earlier change, the
      `implementer` prompt carries these rules. Prefer
      `git show <commit> -- <paths> | git apply -R --index`. When a file must
      be restored from an old commit, first list the later changes with
      `git log <base>..origin/main -- <file>`, re-apply the unrelated later
      hunks, and confirm with `git diff`.
    - `todo.md`: the five whole sections of decisions 1, 2, 3, 4 and 6 are
      gone (heading, Background and TODO). In "Core: widen camera support
      from public sample RAW files" only the `- [ ] Tier 3: ...` line is
      gone; the Tier 1, Tier 2 and Sigma BF lines and the Background stay.
      No other item changes.
    - No file under `crates/` changes, and none of `README.md`,
      `README.ja.md`, `docs/humans/**` or `CLAUDE.md` changes.
    - `mise run ci:local` passes locally (it runs lychee because the branch
      changes Markdown) and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Everything is English. Each record gives the decision, the reason and
      the date, in the form the guides already use ("user decision,
      2026-11-23" in the same good-photo section, "Made by the user on
      2026-11-24" in the all-faces plan's Decisions).
    - Read cap: the sentence to replace is at the end of the NVMe bullet,
      "`READ_SLOTS` was not changed; which cap to ship (one cap, or one per
      drive kind) is an open decision." Keep "HDR PQ CR3 is untested." after
      it. `READ_SLOTS` is in `crates/app/src/scan.rs`; its doc comment is
      not touched.
    - Decision 1: do not claim more than was measured. The bad-photo-labels
      Decision measured the folder's bottom sharpness decile (17.7%, 3.2x
      the 5.52% base rate) and frames with no AF face (14.6%, 2.65x). The
      Low mark is a different rule (the judged face's full-size eye
      sharpness under `LOW_BELOW`), so say that its precision is unmeasured
      rather than borrowing those figures. The same section already
      explains that `mark.rs` mirrors `LOW_BELOW` as `LOW_FULL_EYE_SHARP`.
    - Archived plans: append dated text only, and do not edit what was
      written at the time. In the plan text refer to other plans by
      backticked path, not by link. Inside the two archived plans, follow
      the link style already used there.
    - Decision 3 can refer to the plan's own "Small faces" trade-off bullet
      (Option A chosen, Option B the follow-up now declined) and the first
      Decisions bullet, which calls full-size meshing "a follow-up".
      Decision 4 can refer to the trade-off bullet "The size band and the
      low-resolution input are baked into the stored gate count". The
      measured frame is `2026-11-07/_DSC1611.ARW`, 9 counted (one player,
      eight spectators); it is in the todo item, so carry it into the record
      before deleting the item.
    - `todo.md`: some neighbouring items are separated by a `---` line (one
      sits directly above the "Develop: check intervening commits" heading).
      After the removals leave no doubled or dangling `---` and no doubled
      blank lines. In the camera section the Background still describes
      tier 3 and Tier 2 mentions "(tier 3)"; leave both, see "Trade-offs and
      risks".
    - Skill: "S.3 Implementation" says "Same as 2.2", so one paragraph in
      2.2 covers single-PR mode too. `.claude/` is outside `mise run fmt`'s
      scope; match the file's existing line wrapping by hand.
    - Source for the revert rule:
      `docs/plans/_archived/20261122-revert-page-present-now/learnings.md`
      (Step 1 and "Deferred issues").

## Trade-offs and risks

- **Where decision 5 is recorded.** Chosen: the agents' guide
  `docs/agents/raw-metadata-parsing.md`, which is what an agent reads before
  adding a parser. The alternative is the support policy paragraph of
  `docs/humans/cameras.md`, which users read; that needs the same change in
  `docs/humans/cameras.ja.md` and turns an internal priority into a public
  statement.
- **The camera section's Background still lists tier 3.** Removing only the
  TODO line, as asked, leaves "3. New RAW containers" in the Background with
  no TODO line under it. Not changed, since the request limits the todo
  change to the Tier 3 line.
- **Where the revert caution sits in the skill.** Chosen: "2.2
  Implementation", as a rule for the `implementer` prompt, because the main
  agent does not run git on the code itself.
- **Decision 1 is recorded in two places.** The guide bullet keeps an agent
  from reopening the question; the archived plan's note answers the question
  where it was asked.
- **Appending to archived plans.** Two archived plans gain a dated block.
  No guide has an all-faces section today, so a guide would need a new
  section.
- **The neighbouring item "decide whether a second bad-photo labeling round
  for the cue readings is wanted" stays open.** It was not among the six and
  is out of scope.
- **The read cap item's own "Done when" names `READ_SLOTS` too.** This plan
  changes only the guide; the constant's doc comment in
  `crates/app/src/scan.rs` is not touched (a code-file change is out of
  scope).
- **Docs-only CI.** `todo.md` has an open item about `test (ubuntu-latest)`
  stalling on docs-only commits. If the PR's check hangs, a rerun has been
  enough so far.

## Progress
