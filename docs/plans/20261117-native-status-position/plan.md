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

# Native Status Position

## Purpose

In the GPUI app (`crates/native`), the strip position (`3 / 120`) is drawn
under the file name in the right-hand meta pane. The user's rule is that the
meta pane holds only the shown photo's details and the status bar holds
everything else. This work moves the position into the status bar's right
region, left of the progress circle and the `Sort ▾` / `Filter ▾` buttons,
and, while the strip's filter is active, adds the total listed count in
parentheses (`3 / 80 (120)`), so the user sees how many files the filter
hides. The position then stays visible when the meta pane is hidden.

## Steps

- [x] Step 1: Move the strip position from the meta pane to the status bar, with the listed count while a filter is active
  - Done when:
    - The meta pane draws the file name (and the error line of a failed
      preview) and no position.
    - The status bar's right region shows the position as its first item,
      left of the progress circle (while shown), `Sort ▾` and `Filter ▾`:
      `3 / 120` without an active filter, `3 / 80 (120)` while
      `FilterState::is_active` (80 = the shown `files`, 120 = the listing
      `all`), including when the filter hides nothing (`3 / 120 (120)`).
    - With no shown file (`files` empty) the status bar shows no position,
      in every case (no folder, an empty folder, a filter that hides
      everything).
    - The position shows whenever `files` is non-empty, from the listing
      on, not only once a preview has arrived.
    - Unit tests cover the unfiltered format, the filtered format, the
      one-based index and the empty list; `the_position_is_one_based` no
      longer lives in `meta.rs`.
    - `docs/agents/native-app.md` describes the new placement and format
      and no longer says the position lives in the meta pane.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Touch `crates/native` and `docs/agents/native-app.md` only. The Tauri
      app (`crates/app`) is frozen. `docs/humans/usage.md`'s `N / M`
      counter text describes the Tauri strip header and stays as is.
      `CLAUDE.md` does not mention the position's placement and needs no
      change, unless the helper moves to a new module (see below).
    - The helper is `meta::position(current, total)` at
      `crates/native/src/meta.rs:30`, with its test
      `the_position_is_one_based` at `meta.rs:205`. Its only call site is
      `crates/native/src/main.rs:4110`, inside the `div` that wraps the
      file name. Remove that child; whether the now single-child wrapper
      `div` stays is the implementer's call (match the surrounding style).
    - Keep the formatting a pure, unit-tested function, and move it out of
      `meta.rs` since it is no longer the pane's. Suggested shape:
      `position(current: usize, shown: usize, listed: Option<usize>) -> Option<String>`,
      `None` when `shown == 0`, `listed` being
      `self.filter.is_active().then(|| self.all.len())` at the call site.
      The step decides the home: `pager.rs` (the index is the pager's;
      `CLAUDE.md` has no `pager.rs` entry to maintain) or another existing
      module; a new module would need a `CLAUDE.md` Layout mention.
    - Keep the existing rule: the index is the pager's current index over
      `files` (`self.pager.current()`, `self.files.len()`), not the shown
      file's index, so a refilter that hides the file on screen moves the
      number at once (`docs/agents/native-app.md`, "Rule: the pane's
      `n / total` ...").
    - `status_row` (`main.rs:4347`) builds a gpui-component `StatusBar`;
      add the position as one more `.right(..)` before the progress
      circle's. Follow the rules of the `StatusBar` section of
      `docs/agents/native-app.md`: nothing in `left` (the center status
      text would jump to the middle), each right item its own `.right(..)`
      (the region's `gap_2` spaces them), no override of the bar's root
      style, so the text takes the bar's `text_xs` / `muted_foreground`.
    - Verify during implementation, in a debug build: the position sits
      left of the circle and the buttons, it is not clipped by the right
      region's `overflow_hidden` with a long status text, and the sort and
      filter popovers still anchor to their buttons.
    - Update these places in `docs/agents/native-app.md` (line numbers as
      of this plan): line 862 ("the pane's `n / total` index into"), the
      rule at 945-949 (`meta::position`, "the pane's"), the status row
      description at 1039-1047 ("the current file's position, name and
      shooting settings live in the meta pane only"), the progress
      circle's "the first item of the `StatusBar`'s right region" near
      2196, the `StatusBar` section's rule listing the right items near
      2258, and the meta pane section near 3466 ("the file name,
      `n / total`, ...").

## Trade-offs and risks

- What to show with no files: decided by the user, nothing in every case
  (the pager's index is 0 on an empty list, so the old helper would print
  `1 / 0`). The status text already shows the filtered note when a filter
  hides everything.
- When to show the parenthesized count: decided by the user, whenever
  `FilterState::is_active`, mirroring the lit `Filter ▾` button, so a filter
  that lets every file through reads `3 / 120 (120)`.
- When the position appears: decided by the user, whenever `files` is
  non-empty, so it appears as soon as the folder is listed (the meta pane
  drew it only once a preview had arrived).
- `all` is the listing after `--size` / `--max` narrowing, so the
  parenthesized count is the listed count under those test flags, not the
  folder's file count.
- The circle appears and disappears to the right of the position, so the
  position shifts sideways by the circle's width when a scan starts or
  ends. Putting the position right of the circle would avoid it but
  contradicts the requested order.
- `todo.md` holds an open hand check that names "the pane's `n / total`"
  (the `gpui-exif-filter` section, around line 2612). It goes stale with
  this change; rewording it belongs to the wrap-up's todo curation, not to
  the step.

## Progress
