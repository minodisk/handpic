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

# Sidecar and keymap decisions

## Purpose

Four `todo.md` items were waiting on a product decision. The user has now made each one:

1. A picked PhotoLab virtual copy reads as a pick, and the flag is written to every `Items` entry of a `.dop`.
2. `Rewrite Sidecars from Index…` skips a file whose existing sidecar is larger than `MAX_SIDECAR_BYTES`, and counts it as skipped.
3. A `.xmp` shared by RAWs of one stem is copied, not carried away, when a date split or a file rename separates them.
4. A stored shortcut override wins a key over a default action the user has not overridden, and the move is logged at `warn`.

Once done, frames picked on a virtual copy show as picked, a rewrite never patches a sidecar that was never read, no RAW loses its shared sidecar, and an upgrade that adds a default key no longer takes a key the user bound.

Each step removes its own `todo.md` section and nothing else in that file, because other sessions edit `todo.md` concurrently. Do not touch `docs/plans/20261123-all-faces-check`.

## Steps

- [x] Step 1: `.dop` flag across virtual copies (`fix(core)`)
  - Done when:
    - `dop::read_flag` returns `Flag::Pick` when any `Items` entry has `ShouldProcess = 0`, else the flag of `Items[0]` as today.
    - `dop::write_rating` on an existing sidecar writes `ShouldProcess` to every `Items` entry (pick, reject and unflag alike), inserting the line in an item that lacks it.
    - `Rating`, `ColorLabel`, `Settings` and `Orientation` stay `Items[0]`-only; `read_rating`, `read_label` and `write_label` are unchanged in what they read and write.
    - `write_rating` sets `ModificationDate` on every item to `now`; `write_label` keeps updating only `Sidecar.Date` and `Items[0].ModificationDate`.
    - Rust tests in `crates/core/src/dop.rs` cover:
      - a two-item sidecar with master `2` and copy `0` reads `Pick`;
      - master `1` and copy `0` reads `Pick`;
      - master `1` and copy `2` reads `Reject`;
      - a pick, a reject and an unflag each land on both items, and everything outside the `ShouldProcess` values and the timestamps is byte-identical;
      - the second item's `Rating`, `ColorLabel`, `Settings` and `Orientation` are untouched, and none is inserted into it;
      - `write_label` leaves the second item byte-identical;
      - a second item without `ShouldProcess` gets the line;
      - the existing single-item tests still pass.
    - The `dop.rs` module doc and the doc comments of `read_flag` / `write_rating` state the new rule.
    - The "A picked virtual copy is invisible to `dop::read_flag`" section of `docs/agents/photolab.md` is rewritten to the decided behaviour.
    - `docs/humans/usage.md` and `docs/humans/usage.ja.md` say, in the PhotoLab sidecar bullet, that a picked virtual copy counts as a pick and that the flag is written to the master and its virtual copies.
    - `CLAUDE.md`'s `src/dop.rs` clause is updated if its wording no longer holds.
    - The section "Core: the `.dop` reader ignores a picked PhotoLab virtual copy" is deleted from `todo.md`, and only that section. In its place, a manual-check item is added: in PhotoLab, give a frame a virtual copy, change its flag in Handpic, and confirm PhotoLab shows the new flag on both the master and the copy (the per-item `ModificationDate` rule is unverified).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/core/src/dop.rs`, `docs/agents/photolab.md`, `docs/humans/usage.md`, `docs/humans/usage.ja.md`, `todo.md`, possibly `CLAUDE.md`.
    - `locate` finds the first depth-3 line inside `Items` that starts with `{`. Extend it to collect every such item (its close, its `ShouldProcess` range, its `ModificationDate` range); `Doc` keeps the `Items[0]` fields it has. There is still no Lua parser; keep the brace-depth scanner.
    - `ModificationDate` on every item, unconditionally, because:
      - the module doc and `docs/agents/photolab.md` say PhotoLab ignores a sidecar whose timestamps are older than the database item's `ModificationDate`, and each virtual copy is its own database item, so a copy with a stale date may have its new `ShouldProcess` ignored;
      - `Items[0].ModificationDate` is already bumped on every write whether or not a value changed, so the same rule on every item adds no "did it change" comparison.
    - Edits are applied in descending offset order; items do not overlap, so the per-item edits cannot collide with the `Items[0]` inserts whose order `docs/agents/photolab.md` documents.
    - No fixture in `crates/core/src/fixtures/dop/` has two items. Build the two-item text in the test from an existing fixture (the user chose this over waiting for a real sample), and say so in the PR.
    - Callers checked, none depends on `Items[0]`-only flag semantics:
      - `SidecarFormat::read_flag` / `write_rating` in `crates/engine/src/sidecar.rs`, reached from `reconcile_of` and the writer, pass through;
      - `crates/cli/src/main.rs` (about line 562) prints `dop::read_flag` as a column, which now shows virtual-copy picks;
      - the engine tests that call `dop::read_flag` use single-item sidecars.
    - The fresh template is single-item and does not change.

- [x] Step 2: skip oversize sidecars in `Rewrite Sidecars from Index…` (`fix(engine)`)
  - Done when:
    - A listed RAW with a row, one of whose existing sidecars of the current `SidecarFormat`'s kinds is larger than `MAX_SIDECAR_BYTES`, is neither marked dirty nor handed to the writer by `foldersidecars::rewrite`, and its sidecar bytes are unchanged afterwards.
    - `foldersidecars::preview` counts that file in `skipped`, not in `judged` or `unjudged`.
    - The rewrite's `Summary` reports the skipped count and does not count such a file as `written` or `failed`.
    - Under `Both`, a file is skipped when either existing kind is oversize. Under `Xmp` an oversize `.dop` does not skip the file, and under `Dop` an oversize `.xmp` does not.
    - Rust tests in `crates/engine/src/foldersidecars.rs` cover:
      - an oversize `.xmp` under `Xmp`;
      - an oversize `.dop` under `Dop`;
      - one oversize kind under `Both`, where neither sidecar is written;
      - an oversize sidecar of the other format, which is not skipped;
      - a sidecar of exactly `MAX_SIDECAR_BYTES`, which is not skipped (the folder open tests `size > MAX_SIDECAR_BYTES`);
      - the preview figure and the summary figure agree.
    - The dialog keeps one "skipped" row, reworded to name both reasons (for example "N files skipped (not in the index, or a sidecar larger than 4 MiB)"); the status text after the run follows the same figure. `crates/app/src/sidecars.rs` and its tests match.
    - The `Rewrite Sidecars from Index…` passage of `docs/humans/usage.md` and `docs/humans/usage.ja.md` is updated.
    - The `foldersidecars.rs` module doc and `CLAUDE.md`'s `src/foldersidecars.rs` clause are updated if their wording no longer holds.
    - The section "Engine: Rewrite Sidecars from Index… patches an oversize sidecar" is deleted from `todo.md`, and only that section.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/engine/src/foldersidecars.rs`, `crates/app/src/sidecars.rs`, `docs/humans/usage.md`, `docs/humans/usage.ja.md`, `todo.md`, possibly `CLAUDE.md`. `crates/app/src/folderops.rs` only if a signature it calls changes.
    - `targets` calls `read_listing(dir, None)`, so it sees no sidecars. It needs the format: `read_listing(dir, Some(format))`, `listing::stat_sidecars` and `trash::sidecars_of(format, raw, &stats)` already give each RAW's existing sidecars of the format's kinds with their size, as `doomed` uses them. `targets` is called only inside the module.
    - Filter before `mark_dirty`, so a skipped row is not left dirty.
    - Keep the constant and the comparison shared with `sidecar::reconcile_of` (`size > MAX_SIDECAR_BYTES`).
    - `Summary` has no skipped field today (`written`, `failed`, `flushed`); adding one is part of this step. `Preview` and `Summary` derive `Serialize`; check whether `crates/app/src/companion.rs` reports either to MCP clients before changing a field.
    - When every indexed row is oversize, `targets` still refuses only when no row exists, so the dialog offers a rewrite of 0 files with the skipped figure; keep it, and test it.

- [x] Step 3: copy a shared `.xmp` on a date split and a file rename (`fix(engine)`)
  - Done when:
    - In a split, RAWs that share one `.xmp` and go to different date folders each have the `.xmp` beside them afterwards, moved with one and copied for the others.
    - A dated RAW whose stem-mate is undated, and so stays, gets a copy, and the original `.xmp` stays with the undated RAW.
    - Stem-mates going to the same date folder end with one `.xmp` there, and neither move is refused as "already exists".
    - When every file moved, the original folder is still removed.
    - `.dop` handling is unchanged (per full file name).
    - `rename::file_plan` for a RAW whose `.xmp` is shared with a stem-mate left in the folder plans a copy of the `.xmp` to the new name and leaves the original; an unshared `.xmp` is still moved.
    - A copy never overwrites an existing file, and a planned copy onto an existing target is refused like a move.
    - `rename::file_run` rolls a failed run back by deleting the copies it made and renaming the moves back.
    - Rust tests in `crates/engine/src/datefolder.rs` cover:
      - two mates with different dates;
      - a dated and an undated mate;
      - mates with the same date;
      - three mates over two dates;
      - a failing later step that removes the copy and puts the RAW back;
      - both RAWs' index rows keeping their judgments after `split_folder`.
    - Rust tests in `crates/engine/src/rename.rs` cover:
      - a shared `.xmp` copied on rename;
      - an unshared one moved;
      - a rollback that deletes the copy;
      - a collision on the copy's target.
    - The module docs of `datefolder.rs` and `rename.rs` are updated, as is the "`photolab::lookup` keys on folder + file `Name`…" section of `docs/agents/photolab.md` (its sentence on `file_plan` and a shared `a.xmp`).
    - The `Rename by Capture Date…` passage of `docs/humans/usage.md` and `docs/humans/usage.ja.md` is updated, and `CLAUDE.md`'s `src/rename.rs` / `src/datefolder.rs` clauses if their wording no longer holds.
    - The section "Engine: a split moves a shared sidecar with whichever RAW moves first" is deleted from `todo.md`, and only that section.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/engine/src/rename.rs` (`FilePlan`, `file_plan`, `file_run`), `crates/engine/src/datefolder.rs` (`move_plan`, `split`), `docs/agents/photolab.md`, `docs/humans/usage.md`, `docs/humans/usage.ja.md`, `todo.md`, possibly `CLAUDE.md`.
    - `FilePlan` is `moves: Vec<(PathBuf, PathBuf)>`, RAW first. It needs to tell a copy from a move. `file_run` takes only a `rename` closure, which the tests use to inject failures; the copy and the delete of a copy need the same injectability.
    - Define "shares" through `sidecar::existing_sidecar(raw, SidecarFormat::Xmp)`: two RAWs of one folder (`handpic_core::scan::is_raw_file`) share when it gives the same path. That is the file the writer patches for each, and it covers the case-insensitive fallback.
    - `std::fs::copy` overwrites its target; the copy must not.
    - In a split the copy-or-move choice depends on where the stem-mates go, which `move_plan(file, target)` cannot see today. A choice made only from "a mate is still in the source folder" fails when mates share a date: the second move would find the `.xmp` already in the target and be refused. `split` has all the groups and the undated files stay, so the destinations are known up front. This step decides the rule; the test cases above pin it.
    - A file whose move fails stays in the source and must keep its `.xmp`.
    - For `file_plan`, a case-only rename on a case-insensitive file system makes the copy's target the shared sidecar itself; plan nothing for it then.
    - Index: `Index::move_file` carries each RAW's `ratings` row, with its stored sidecar stat, to the new path. A copy may have another mtime, so the next open's `reconcile_of` reads it back; the content is the same and the caller has drained the writer, so no index change is expected. Confirm with the `split_folder` test.
    - Undo: neither a split nor a file rename is recorded in `trash::Runs` or the app's history, so there is nothing to extend.
    - `rename::file_plan` has no caller in `crates/app` today (the companion refuses `rename_file` with `not_in_app`), so its change is covered by the engine tests alone.

- [ ] Step 4: a stored shortcut override wins over an un-overridden default (`fix(engine)`)
  - Done when:
    - `Keymap::from_overrides` gives a stored override every key it asks for that is held by a default action with no usable stored override of its own. That action keeps its remaining default keys, or has none, and each such move is logged at `log::warn!` naming the key and both actions.
    - Two overrides that want the same key are resolved as today (first in action order; the other applies in part or stays inactive).
    - An invalid or forbidden override still displaces nothing.
    - An action left without keys loads back the same way: `overrides()` never stores an empty key list (`parse_keys` refuses one), and `Keymap::from_overrides(Some(&keymap.overrides())) == keymap` holds for the new cases.
    - A new Rust test in `crates/engine/src/shortcuts.rs` loads an old store that binds a newer default's key to another action (for example `{"next": ["arrowright", "d"]}`, and one with `e`) and asserts `next` holds the key, `performance` / `peaking` has no key, and the round trip.
    - The existing tests whose expectations change are updated and renamed to say what they now assert:
      - `p_on_another_action_is_skipped_while_pick_holds_it`;
      - `a_collision_with_another_default_is_skipped`;
      - `a_stale_default_in_an_old_store_drops_only_that_key`;
      - `a_partly_applied_override_frees_its_default_for_another`;
      - the tests that build an inactive override from a default's key, which now need two overrides wanting one key: `an_inactive_override_survives_unrelated_edits`, `editing_or_resetting_an_action_drops_its_inactive_override`, `inactive_overrides_round_trip`, `a_conflicting_override_keeps_only_its_filtered_keys`.
    - A Rust test in `crates/app` covers an action with no keys where one is missing: `keys::Bindings`, the settings modal's `shortcut_rows`, `context.rs`'s `shortcut_of`.
    - The `from_overrides` doc comment is updated.
    - The bullet "Adding a new default key (`DEFAULTS`) wins over any stored override…" under "Resolve shortcut overrides order-independently" in `docs/agents/shared-crates.md` is rewritten to the new rule.
    - The `Keyboard Shortcuts` paragraph of `docs/humans/usage.md` and `docs/humans/usage.ja.md` says a key the user bound stays theirs when a later version makes it another action's default, leaving that action without a key.
    - `CLAUDE.md`'s `src/shortcuts.rs` clause is updated if its wording no longer holds.
    - The section "Shortcuts: a newly added default key silently displaces a user's stored override" is deleted from `todo.md`, and only that section.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Files: `crates/engine/src/shortcuts.rs`, `docs/agents/shared-crates.md`, `docs/humans/usage.md`, `docs/humans/usage.ja.md`, `todo.md`; `crates/app/src/keys.rs`, `settingsmodal.rs` or `context.rs` only for tests or a fix found there.
    - "Not overridden" is known in `from_overrides` as an action with no entry in `pending` (the valid, non-forbidden stored overrides). The yield can happen before the existing retry loops, which then run unchanged over the overrides.
    - Every default in `DEFAULTS` has one key today, so "keeps its remaining keys" cannot occur with the shipped table. A displaced action with keys left differs from its default, so `overrides()` stores the remainder as that action's override on the next save.
    - `overrides()` emits every action whose keys differ from the default, so an action with no keys would be stored as `[]`, which `parse_keys` refuses with a warning on every launch; leave it out and let the load derive it again.
    - What the app does with an action with no keys today, to confirm rather than assume: `Bindings::new` maps nothing for it; `accelerator_for` gives `None`; `empty::open_hint` handles an empty list; `context::shortcut_of` gives an empty string; `settingsview.rs`'s row draws no `Kbd` and keeps `+`; `Keymap::remove` refuses the last key, which is unaffected; `Keymap::reset` on the displaced action is refused with "`<key>` is bound to `<action>`" until the user frees the key.

## Trade-offs and risks

- Step 1, the first write spreads a virtual copy's pick to the master. A frame whose virtual copy is picked reads as `Pick`, and every `write_rating` writes the flag it holds to every item. So the first star change on such a frame, or a `Rewrite Sidecars from Index…` over the folder, makes the master picked in PhotoLab too. This follows from the two decisions together and was accepted.
- Step 1, `ModificationDate` on every item is not verified against PhotoLab; Step 1 files a manual check in `todo.md`.
- Step 1, the two-item layout is inferred. The repository has no real virtual-copy `.dop`; `locate` assumes each item opens on its own depth-3 line, as the first item does.
- Step 2, `Both` with one oversize kind skips the whole file, because the writer patches both kinds of a file together.
- Step 3, moving to the Trash still takes a shared `.xmp` (`trash::groups_of`). The todo item names only the split and the rename, so this plan leaves it.
- Step 3, a copy is a second file. Later edits to one RAW's copy no longer reach the other; this is the meaning of the "copy" decision. On a failed rollback (the delete of a copy failing) a stray `.xmp` can remain in the target folder; it is logged, as a failed rename-back is today.
- Step 4, the rule cannot tell a new default from an old one. `{"reject": ["p"]}` in a store now takes `p` from `pick` and leaves `pick` without a key, where it was ignored before. Several existing tests assert the old outcome and change.
- Step 4, the displaced action is found only on the Keyboard Shortcuts page and in the log; the decision asks for a warn log only.
- The four PRs each edit `todo.md`, and most edit `docs/humans/usage.md` and its `.ja.md`, so they are merged in order.

## Progress

- (2026-11-23) Step 1 complete
- (2026-11-23) Step 2 complete
- (2026-11-24) Step 3 complete
