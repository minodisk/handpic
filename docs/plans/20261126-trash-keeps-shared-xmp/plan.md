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

# Move to Trash keeps a `.xmp` shared with a RAW that stays

## Purpose

Two RAWs of one stem in a folder (`a.ARW` and `a.NEF`) share `a.xmp`. Moving
one of them to the Trash takes the shared `.xmp` along today
(`trash::groups_of`), so the RAW that stays loses its sidecar and its
judgment. The
[sidecar-and-keymap-decisions plan](../_archived/20261123-sidecar-and-keymap-decisions/plan.md)
fixed this for the date split and the file rename and left the Trash out of
scope; `todo.md` carries it as "Engine: Move to Trash takes a `.xmp` shared
with a RAW that stays".

The user decided the rule: a shared `.xmp` stays in place when any RAW sharing
it is not moved to the Trash in the same run. Only the target RAW and its
`.dop` go, and the undo of that run restores only them. When every RAW sharing
it moves in one run, the `.xmp` moves too. `Delete Permanently…` already works
this way (`trash::delete_for_good`); after this plan both follow one rule.

## Steps

- [x] Step 1: Keep a shared `.xmp` out of a Move to Trash run unless every RAW sharing it goes
  - Done when:
    - A test in `crates/engine/src/trash.rs` moves only one RAW of a
      shared-stem pair (`a.ARW`, `a.NEF`, `a.xmp`, `a.ARW.dop`) through the
      pure seam (the grouping, then `run` with the tests' `mover`) and finds
      `a.xmp` still in the folder, only the RAW and its `.dop` in the recorded
      `Trashed` list, and a restore of that run (`restore_with`) bringing
      back only the RAW and its `.dop`.
    - A test moves both RAWs of the pair in one run and finds `a.xmp` moved,
      recorded exactly once, with no failure reported.
    - A test covers both RAWs selected with one failing to move: the shared
      `.xmp` stays (see Trade-offs and risks, item 1).
    - A test covers the chunked path (`run_chunked`, the Windows mover) for
      the both-selected case: the shared `.xmp` is handed to `move_many` at
      most once.
    - The existing `delete_for_good` tests pass unchanged in what they
      assert.
    - `README.md` and `README.ja.md`, `docs/humans/usage.md` and
      `docs/humans/usage.ja.md`, `docs/agents/photolab.md` and
      `docs/agents/shared-crates.md` describe the new rule, and no text still
      says a Move to Trash takes a shared `.xmp` from a RAW that stays.
    - The `todo.md` item "### Engine: Move to Trash takes a `.xmp` shared
      with a RAW that stays" is deleted, and no other item is touched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - All code changes are in `crates/engine/src/trash.rs`. `trash_listed`
      calls `groups_of` bare today; `delete_listed` calls `delete_for_good`,
      which calls `groups_of` and then already applies the rule: a `.xmp`
      whose lower-cased stem is that of a RAW of the listing not among the
      chosen paths is dropped from the group, and a `.xmp` listed by several
      chosen groups is taken out of all of them and removed once after
      `run`, only when every RAW that listed it went.
    - Extract that filtering from `delete_for_good` into a helper both paths
      use, rather than writing a second copy. `trash_listed` already reads
      the listing (`read_listing(.., Some(SidecarFormat::Both))`), so
      `listing.files` is at hand as it is for `delete_listed`; no extra
      `read_dir` is needed. Judge sharing as delete does, by lower-cased
      stem over the listing, which matches how `sidecars_of` finds a sidecar
      (by lower-cased name); `rename::shared_xmps` is the reference for the
      concept, not the code to call (Trade-offs and risks, item 2).
    - A foreign path (another folder) must not count as chosen: the existing
      test `a_refused_foreign_raw_does_not_stand_for_the_kept_one_of_its_name`
      holds this for delete, and the shared helper must keep it for the
      Trash.
    - Only the `.xmp` is ever shared. A `.dop` name embeds the RAW's whole
      name (`a.ARW.dop`), so it always goes with its RAW.
    - Both RAWs selected: today the shared `.xmp` sits in both groups. By
      reading the code, `run` then reports a failure for the second move of
      a file already gone, and `run_chunked` passes the same path twice to
      one `move_many`. List it once. For the Trash the run is recorded, and
      `restore`, `redo_groups`, `items` and `item_starts` read `moved` as
      "each RAW followed by its sidecars", so the shared `.xmp`'s `Trashed`
      record must sit after the records of one of its RAWs (the last sharing
      RAW that went), not at the end of the run behind an unrelated RAW.
    - `move_groups` is also used by `foldersidecars::delete_run`, and
      `run` / `run_chunked` by the redo; keep their behavior for groups
      without a shared `.xmp` unchanged.
    - The undo needs no code change: a `.xmp` that was not moved is not in
      `Runs`. The redo moves exactly what the undo brought back, so it does
      not take the `.xmp` either; assert that in the test if it is cheap.
    - `trash_listed` itself touches the real Trash, so test through a pure
      function with the mover as a closure, as the existing tests do
      (`mover(trash)`, `delete_in`, `move_many`, `restore_with`).
    - Update the doc comments that state the grouping: the module header,
      `groups_of`, `delete_for_good` ("grouped and ordered as a move to the
      Trash is") and `trash_listed`.
    - Docs to change:
      - `README.md`, the paragraph "Two RAWs of the same stem with different
        extensions..." (the Trash sentence and "Handpic moves an existing
        `FOO.xmp` along with a file moved to the Trash whichever format is
        chosen"), with `README.ja.md` kept in sync in the same PR.
      - `docs/humans/usage.md`, **Move to Trash…** ("Each RAW goes with the
        sidecars sitting next to it") and **Delete Permanently…** ("each RAW
        with the sidecars next to it"): add the shared-`.xmp` exception, and
        that the undo then restores only the RAW and its `.dop`; the same in
        `docs/humans/usage.ja.md`.
      - `docs/agents/photolab.md`, the sentence ending "`trash::groups_of`
        still takes it along when `a.ARW` alone goes to the Trash".
      - `docs/agents/shared-crates.md`, the trash sections: record the rule
        and where it lives.
    - `CLAUDE.md` does not have to change: its sentence on
      `src/trash.rs` ("each RAW with the `.xmp` and `.dop` sidecars that
      exist for it") is a summary that already leaves out delete's same
      exception. Add a clause only if the implementation makes that sentence
      wrong.

## Trade-offs and risks

1. Both RAWs selected, one fails to move. The decision says the `.xmp` stays
   when any RAW sharing it is not moved in the same run, so the plan moves
   the shared `.xmp` only after all its RAWs went, as `delete_for_good`
   does. That costs one extra move per shared `.xmp` (on Windows one more
   shell operation, for a rare layout) and an insertion into `moved` at the
   right place. The cheaper option, attaching the `.xmp` to the last
   selected sharing RAW's group, keeps `run` untouched but sends the `.xmp`
   to the Trash when an earlier sharing RAW failed and stays. Not taken.
2. Sharing is judged by lower-cased stem over the listing (delete's rule),
   not by `rename::shared_xmps` (its own `read_dir`, entries compared after
   canonicalizing). The two differ only for stems that differ in case on a
   case-sensitive file system (`a.ARW`, `A.NEF`, with `a.xmp` and `A.xmp`
   both present): the stem rule treats them as sharing and keeps the `.xmp`,
   which errs toward leaving a file in place. Using `shared_xmps` instead
   would make the Trash and delete paths disagree unless delete is changed
   too.
3. Two single moves in a row: `a.ARW` alone goes (the `.xmp` stays), then
   `a.NEF` alone goes (nothing shares the `.xmp` any more, so it goes with
   it). Undoing only the first run brings `a.ARW` back with its `.dop` and
   no `.xmp` until the second run is undone too. This follows from the
   decision; the plan documents it in the usage guide and adds no handling.
4. `README.md` calls same-stem RAWs "not supported". They still share one
   judgment through the one `.xmp`, so the plan keeps that wording and
   corrects only the sentences about the Trash. Dropping "not supported" is
   a separate decision.

## Progress
