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

# Close out ten todo items (todo-ten-closeouts)

## Purpose

Ten items of `todo.md` have been decided by the user and only need carrying
out: eight are small code changes in `crates/app` (some also in
`crates/engine`), two are decisions that need no code and are recorded in
`docs/agents/app.md`. Once done, the MCP `get_view` reports the eyes'
openness, a new trash run no longer shows an earlier run's failures, a folder
switch no longer loses the previous folder's last viewed file, every listing
waits for the launch eviction, the prefetch ring covers the pager's wrap, the
dead `compare` key and the MCP `compare` view mode are gone, `ctrl+,` opens
the settings from the folder tree, and the default window is centered on the
visible area.

Rules for every step:

- Each step deletes from `todo.md` only the items it names, by their exact
  `### ...` heading (the heading and its body up to the next `### ` heading).
  Other sessions edit `todo.md` concurrently: make a targeted edit of that
  one item, never rewrite or reorder the file, and re-read it right before
  editing.
- Do not touch `docs/plans/20261123-all-faces-check`.
- The decisions below are the user's and are not reopened.
- Where a change makes a sentence of `CLAUDE.md`, `docs/agents/app.md` or
  `docs/humans/usage.md` (and its `usage.ja.md` pair) wrong, fix that
  sentence in the same step.

## User decisions (2026-11-23)

- `compare`: remove it from the keymap **and from the MCP surface** (the
  `set_view` tool's `compare` mode, `ViewMode::Compare`, the server
  instructions, `get_view`'s `compare_active`, the usage docs). The app then
  names compare nowhere.
- `openness` in `get_view`: the rounded integer the meta pane shows.
- Trash failure lines: cleared when the next trash-family run starts;
  judgment write-failure lines stay.
- `Latest`: pending value per folder.
- Eviction: every listing waits for it.
- Prefetch: `neighbours` wraps; a kept ring entry keeps its GPU texture (not
  released on `keep`).
- Strip image box: stays 3:2 (144x96 default).
- `ctrl+,`: passes the tree's gate.
- Default window: centered on the visible area.

## Steps

- [x] Step 1: Keyboard and MCP: remove `compare` everywhere, and let the settings key through the tree's gate
  - Done when:
    - The keymap has no `compare` action: it is gone from
      `crates/engine/src/shortcuts.rs`'s `DEFAULTS` (and its
      `the_defaults_are_the_full_table` test), from
      `crates/app/src/settingsmodal.rs` (`shortcut_label`'s `Compare frames`
      and `known_group`), and from `companion.rs`'s `coverage::KEYMAP`; the
      Keyboard Shortcuts page no longer lists it and `v` is free to bind.
    - A stored `shortcuts` override that still names `compare` loads: a new
      test next to `a_store_with_the_retired_photolab_action_still_loads`
      shows the other overrides apply, the rest equals the defaults, and
      `overrides()` no longer carries `compare`.
    - The MCP surface no longer has compare: the `set_view` tool's `compare`
      mode (`ViewMode::Compare` in `crates/engine/src/mcp.rs`), its mention in
      the server instructions and tool descriptions, `get_view`'s
      `compare_active`, the coverage table's `NotImplemented` entry for it,
      and `docs/humans/usage.md` / `usage.ja.md`'s "Asking for compare ... is
      refused" wording (keep the file-rename part). A `set_view` call with
      `mode: "compare"` is now refused as an unknown mode by the schema /
      argument parsing, pinned by a test. The coverage tests pass.
    - With the tree holding the keyboard (`tree_keys`), `settingsmodal::OPEN_KEY`
      opens the settings; while the inline rename owns the keys it still does
      not. A test that fails before the change pins it (a UI test in
      `crates/app/src/uitest.rs` after
      `the_settings_modal_drops_the_paging_keys_until_escape_closes_it`, or a
      pure test in `treekeys.rs` if the gate is given the key).
    - `docs/agents/app.md` is accurate ("Every keystroke goes through the
      keymap", "The tree takes the keyboard through one flag in the view",
      the coverage table paragraph, the MCP section), as are
      `docs/humans/usage.md` / `usage.ja.md` and `CLAUDE.md`'s `treekeys.rs`
      / Coverage sentences (the "compare" in "refused with `not_in_app` until
      the feature lands (compare, `rename_file`)").
    - The `todo.md` items "App: the keymap's `compare` action is bound to `v`
      and listed, but does nothing" and "App: `ctrl+,` does not open the
      settings while the folder tree holds the keyboard" are deleted.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `Keymap::from_overrides` already warns about and skips an unknown
      action (`an_unknown_action_does_not_block_the_others`), so loading
      needs no new code, only the test.
    - `main.rs`'s `key_down` returns on `rename_owns_keys()` before the tree
      gate, so the rename exception already holds; the gate block
      (`self.tree_keys && (tree_key_down(..) || tree_gate(action) == Swallow)`)
      swallows `ctrl+,` because it is no keymap action. Either test
      `name == settingsmodal::OPEN_KEY` before that block, or make
      `treekeys::tree_gate` take the key name; the step decides by which is
      testable with less code. `keys::typed_char` returns `None` with Ctrl /
      Meta held, so the type-ahead does not take the key.
    - `open_settings` keeps its own refusals (a trash or folder dialog or a
      folder flow under way); do not change them.
    - `the test every_feature_not_in_the_app_is_refused_with_the_shared_message`
      requires every refused feature to have a `NotImplemented` entry; with
      compare gone only the photo rename remains.
    - `crates/engine` changes, so `ci:local` also checks the root workspace's
      `handpic-engine`.

- [x] Step 2: MCP surface: `get_view`'s eyes carry `openness`, and a new trash run clears the earlier run's failure lines
  - Done when:
    - `get_view`'s `eyes` object carries `openness`, the integer 0 to 100 the
      meta pane's `Eyes` row shows (`js_round(openness(ear))` from
      `EyesJudgment.ear`), the formula shared and not duplicated;
      `companion.rs`'s `eyes` tests cover it (a value between the ends and
      the clamps at both ends).
    - The tool's description in `crates/engine/src/mcp.rs` (`get_view`) and
      the `get_view` row of `docs/humans/usage.md` and `usage.ja.md` name
      `openness`.
    - When a Move to Trash, a Delete Permanently, or an undo or redo of a
      trash run starts, the failure lines of the previous trash-family run
      (the trash summary's failures pushed from `trashed`, and the
      "could not restore from the Trash" lines pushed from `restored`) leave
      the pane's errors and `get_status`'s `errors`; the judgment
      write-failure lines (the judgment store's and `Event::SidecarError`'s)
      stay.
    - A test covers that rule, on a pure model if possible: a trash failure
      then a new run start leaves only the judgment lines; a failure of the
      new run shows; a folder open still clears all of them as today.
    - `docs/agents/app.md` says when the trash failure lines go, and
      `CLAUDE.md` is still accurate.
    - The `todo.md` items "App: the MCP `get_view` eyes summary carries no
      openness" and "App: the pane's error list keeps an earlier Move to
      Trash failure after later operations finish" are deleted.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `openness` and `EYES_WIDE_OPEN_EAR` are private to
      `crates/app/src/meta.rs` today (`focus.ts`'s `eyesOpenness`;
      `EYES_CLOSED_EAR` is `handpic_core::eyes`'s). Share them by widening
      their visibility or by moving them to `mark.rs`, the port of
      `focus.ts`; the step picks.
    - `write_errors: Vec<(String, String)>` in `main.rs` is keyed by path, the
      newest message replacing the older, filled by `write_failed` from four
      places (the judgment store, `Event::SidecarError`, `trashed`,
      `restored`) and cleared in `opened`. The run starts are `run_trash`,
      `undo_trash` and `redo_trash` (after their early-return guards, where
      the run really starts; not at the dialog or while the gate holds it).
    - The trash lines need their own origin: a second list, or a tag on each
      entry. With one tagged list a trash failure replaces the same path's
      judgment line and the next run's clear then drops both, so check which
      shape keeps "judgment lines stay" true for a path that has both.
    - `trashed` also ends a redo of Delete Sidecars… (`What::Sidecars`), which
      runs through `redo_trash`; it is part of the same family.

- [x] Step 3: Folder open bookkeeping: `Latest` keeps a pending value per folder, and every listing waits for the launch eviction
  - Done when:
    - `viewed::Latest` keeps one pending value per folder: a position pushed
      in folder A while a write is out is still written after B's first push,
      and both folders' latest positions reach `set_last_viewed`. Unit tests
      in `viewed.rs`: A then B pushed during a write are both handed out, in
      first-push order; pushes to one folder still collapse into its latest;
      one write is out at a time; the existing cases still hold.
    - Every listing waits for the launch eviction, also when the first
      listing's task was dropped before it joined the eviction: the eviction
      is a shared handle each listing's background task waits on, not an
      `Option<JoinHandle>` only the first listing takes. A test covers the
      shared wait where feasible.
    - `docs/agents/app.md` and `CLAUDE.md` (`viewed.rs`'s `Latest` sentence)
      are accurate.
    - The `todo.md` items "App: the coalescing `last_viewed` writer can lose
      folder A's position when B is opened and paged first" and "App: a
      second open started before the first listing joins the launch eviction
      can scan before the eviction runs" are deleted.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `Latest<T>` has one user: `main.rs`'s `last_viewed: Latest<(String, String)>`
      (`push((dir, path))`, `settled()`, `write_last_viewed`). `Batches` in
      the same file already keeps its pending per folder in first-push order;
      follow its shape.
    - In `main.rs`'s `list`, the first listing takes `self.eviction`
      (`JoinHandle<()>`) and joins it inside its background task; a later
      listing waits only on `settled`, a oneshot that also resolves when its
      sender is dropped, which is the hole. A handle each listing's
      background task locks, takes and joins under the lock (for example
      `Arc<Mutex<Option<JoinHandle<()>>>>`) closes it. Keep the `settled`
      chain as it is for the previous scan's stop.

- [ ] Step 4: Prefetch: `neighbours` wraps like the pager; record that kept entries keep their texture
  - Done when:
    - `prefetch::neighbours` wraps as `Pager::go` does on `Next` / `Previous`:
      at the last file the next neighbour is the first file, at the first
      file the previous one is the last; never the current file, and no
      path twice (no neighbour with 1 file, one with 2 files). Unit tests in
      `prefetch.rs` replace `stops_at_the_ends_of_the_list` and cover both
      ends in both directions and the 1-, 2- and 3-file lists; the ring's
      other tests still pass.
    - If the measurement is feasible on the host (the 20-file compare
      folders, a release build, a warm index): `--auto 60` is re-run and the
      hit count and p50 / p95 `keypressToPixels` are recorded under "A
      prefetch hit leaves only the frame" in `docs/agents/app.md`. If it is
      not feasible, the PR says so and the section's earlier numbers are
      marked as from before the wrap.
    - `docs/agents/app.md`'s "The prefetch ring holds the decoded neighbours"
      no longer says "none across the pager's wrap", and records in one
      sentence the decision that a kept ring entry keeps its GPU texture (no
      release on `keep`: paging speed over memory).
    - `CLAUDE.md`'s `prefetch.rs` sentence is still accurate.
    - The `todo.md` items "App: the prefetch ring never covers the wrap from
      the last file to the first" and "App: kept prefetch-ring entries keep
      their GPU texture" are deleted.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The pager's wrap is `(index + 1) % len` / `(index + len - 1) % len`.
    - `Move::NextClamped` (auto-advance) does not wrap; the ring then holds a
      neighbour the next press may not use, which costs only its place under
      `PREFETCH_PIXEL_CAP`.
    - Check that `Ring`'s `crowded` set still behaves with a wrapped
      neighbour.

- [ ] Step 5: Window and strip defaults: center the default window on the primary display's visible area; record that the strip's image box stays 3:2
  - Done when:
    - `main.rs`'s `window_placement` no longer calls
      `Bounds::centered(None, size(px(1600.), px(1000.)), cx)`: the default
      bounds come from a pure function in `crates/app/src/session.rs` that
      centers 1600x1000 in the primary display's visible area, with unit
      tests (no taskbar, a taskbar at the bottom, a taskbar docked left, the
      Windows workspace-coordinate case).
    - On the Windows host, `--auto` with the taskbar in its usual place
      still opens the window centered in the visible area, checked from the
      window's rectangle and noted in `learnings.md`.
    - `docs/agents/app.md`'s "A launch restores the last folder and the
      window" describes the new default and no longer calls the question
      pending in `todo.md`; `CLAUDE.md`'s `session.rs` sentence is accurate.
    - `docs/agents/app.md`'s "The strip's height is one number, stored under
      `stripHeight`" records in one sentence the decision that the image box
      stays 3:2 and the default 144x96 (a taller strip enlarges portrait
      thumbnails).
    - The `todo.md` items "App: the `--auto` default window is not centered
      when the taskbar is docked left" and "App: decide whether the strip's
      image box grows from 144x96 to the Tauri strip's 144x144" are deleted.
    - The hand check with the taskbar docked left is listed under
      `## Deferred issues (todo candidates)` in `learnings.md`, so the
      wrap-up adds it to `todo.md` as a pending manual check.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `session.rs` is free of GPUI types (`Rect`, `Screen { uuid, bounds, visible }`);
      keep the function pure over those types.
    - The recorded measurement (one 5120x1440 display, a 62 px left
      taskbar): the window was 93 px right of the display's center; 93 = 31
      (visible-area center offset) + 62 (taskbar), consistent with Windows
      reading the position as workspace coordinates. Inferred from one
      measurement: verify on the host, and take the platform as a parameter
      of the pure function so both cases are unit-tested on every platform.
    - Do not change `session::place` (the restore path) in this step.

## Trade-offs and risks

- **Removing `compare` from the MCP contract (Step 1).** A client sending
  `mode: "compare"` now gets an unknown-mode error instead of `not_in_app`.
  Decided by the user.
- **One list or two for the trash failure lines (Step 2).** Left to the step,
  bound by "judgment lines stay".
- **The Windows centering formula rests on one measurement (Step 5).** The
  left-docked check needs the user's taskbar setup and stays manual.
- **The `--auto 60` re-run (Step 4)** is "if feasible".
- **Deleting todo headings inside the implementation PRs.** The wrap-up is
  told these headings are already gone and only adds the Step 5 manual check.
  Keep each deletion a minimal hunk; on a conflict re-apply only that
  deletion.
- **A stored key on `v` (Step 1).** A user override that put another action
  on `v` now applies. Intended.

## Progress


- (2026-11-24) Step 1 complete
- (2026-11-24) Step 2 complete
