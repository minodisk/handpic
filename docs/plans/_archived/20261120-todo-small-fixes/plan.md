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

# Small todo.md fixes (blank Lens item, selected count, issue template, merger wording, DMG size)

## Purpose

Five `todo.md` entries are each settled by a small code or documentation fix.
This plan fixes them and removes each entry from `todo.md` in the same change,
so the list stops carrying work that takes minutes:

1. "Native: an empty lens string lists a blank `Lens` item in the filter's EXIF
   groups".
2. "Native: the status line omits the Tauri ` · N selected` suffix for a
   multi-selection".
3. "Native: the issue template says `Help > Open Log Folder`, which the native
   app does not have".
4. "Agents: `merger.md`'s post-merge paragraph calls backgrounding
   "impossible"".
5. "Package: the macOS DMG step fails with
   `hdiutil: create failed - No space left on device`".

All five were confirmed open on main at 701653bc. For item 3,
`docs/agents/app-log.md` no longer carries the sentence the entry names, so
only the template is left to fix.

## Decisions

- Item 1 is fixed in the backend's `exif()` (an empty or whitespace-only lens
  becomes `None`), so the filter menu, `filter_offers` and the meta pane's
  `EXIF` group all drop the blank `Lens`.
- Item 2's suffix goes after the optional `(listed)` part:
  `3 / 80 (120) · 4 selected`.
- Item 5 is proven by dispatching `Package` on the PR's branch
  (`gh workflow run package.yml --ref <branch>`) with a warm cache, both macOS
  legs green. If no green run can be had before merge, keep the `todo.md`
  entry with its TODO narrowed to "confirm on a warm-cache `Package` run"
  instead of removing it.
- One PR for all five. Item 3 touches `.github/**`, so the merge waits for the
  user's approval.
- Item 3's alternative (adding an "open log folder" feature) is not taken.

## Steps

- [x] Step 1: Fix the five items and remove their `todo.md` entries
  - Done when:
    - An empty (or whitespace-only) lens string produces no `Lens` item in the
      filter menu and none in `get_view`'s `filter_offers`, and a unit test
      covers it.
    - With more than one file selected, the status row's position reads
      `n / N · K selected` (`n / N (listed) · K selected` while a filter is
      on); with one or none selected it is unchanged. Unit tests of the pure
      function cover both.
    - `.github/ISSUE_TEMPLATE/os.yml` no longer mentions
      `Help > Open Log Folder` and still lists the three log folders.
    - `.claude/agents/merger.md`'s post-merge paragraph no longer calls
      backgrounding "impossible" and agrees with its step 1 and with
      `pr-runner.md` §4.
    - `tools/package/macos.sh` creates the DMG with explicit headroom, and a
      `Package` run dispatched on the PR's branch with a warm cache produces
      both DMGs (arm64 and x86_64), its "Check the DMG" step passing.
    - The five entries are gone from `todo.md` (item 5 narrowed instead, per
      "Decisions", if its proof run is not green).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Item 1. `crates/backend/src/exif.rs`'s `exif()` builds
      `lens: shot.lens_model.clone()`; map an empty or whitespace-only string
      to `None` there (a name with real text is left untouched, no trimming),
      with a test next to the existing ones. The `Exif` is built from the
      indexed `Shot` at read time, so no `EXTRACTOR_VERSION` bump. Do not touch
      `crates/core`'s readers. `crates/native/src/exif.rs`'s `exif_key` and
      `groups_of` and the companion's `filter_offers` then see `None`.
    - Item 2. The removed Tauri `renderMeta` appended
      `` ` · ${selection.selected.size} selected` `` to the position text
      when `selection.selected.size > 1`. The native position is
      `pager::position(current, shown, listed)` in `crates/native/src/pager.rs`,
      called only from `status_row` in `crates/native/src/main.rs`. Give the
      pure function the selected count (`self.selection.selected.len()`) and
      append ` · K selected` after the optional `(listed)` part when it is
      over 1; add tests beside `the_position_is_one_based` and
      `an_active_filter_adds_the_listed_count`. The selection is pruned to the
      shown files on a refilter (`selection::prune`), so the count needs no
      extra filtering.
    - Item 2, documentation. `docs/humans/usage.md` describes the status row's
      position; add the selected count there and in `docs/humans/usage.ja.md`
      in the same PR.
    - Item 3. In `.github/ISSUE_TEMPLATE/os.yml`'s `log` field, drop the
      `Help > Open Log Folder` sentence and keep the three paths, which match
      `docs/agents/app-log.md` and `crates/native/src/logger.rs`'s `log_path`.
    - Item 4. `.claude/agents/merger.md` lines 215-221: replace "(backgrounding
      is impossible for the same reason as step 1)" with wording that rules it
      out for step 1's reason (lines 91-99): the foreground run is what lets the
      counters tick and keeps the exit code observable to branch on. Change that
      clause only.
    - Item 5. `tools/package/macos.sh` line 39: add an explicit `-size` (from
      `du` of `${stage}` plus a fixed and a proportional margin) to the existing
      `hdiutil create -srcfolder ... -format UDZO`. If the dispatched run still
      fails, switch to a read-write image converted with `hdiutil convert` to
      UDZO and dispatch again. Keep the script clean under shellcheck.
      `.github/workflows/package.yml` needs no change.
    - `todo.md`. Remove each of the five entries whole. The item 3 entry sits
      between two `---` rules (around lines 2323-2333): remove one of them with
      it so no doubled rule is left.
    - `CLAUDE.md`: re-check once the code is written and change it only if a
      sentence became wrong.

## Progress

- (2026-11-21) Step 1 complete
