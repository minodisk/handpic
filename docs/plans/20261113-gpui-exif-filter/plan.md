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

# GPUI EXIF filter sections (closes port Step 7)

## Purpose

The GPUI app (`crates/native`) has every filter section of the Tauri app
except the per-folder EXIF ones (camera, lens, aperture, shutter speed, ISO,
focal length ranges), which
[the filter-sort plan](../_archived/20261113-gpui-filter-sort/plan.md) left to
a follow-up slice (the user's decision, 2026-11-13). This plan ports them and
fixes the two refilter bugs that plan's reviews found, which closes Step 7 of
[the GPUI port plan](../20261109-gpui-port/plan.md).

Once done, the native filter menu narrows a folder by camera, lens, aperture,
shutter speed, ISO and focal length range as the Tauri one does, the pane's
`n / total` follows a refilter at once, and a menu change made while the
filter hides every file comes back to the file shown before the list emptied.

### Wrap-up notes (not a step)

- When the PR merges, the wrap-up ticks Step 7 in
  `docs/plans/20261109-gpui-port/plan.md` and adds one Progress line there
  naming this plan's archived path
  (`../_archived/20261113-gpui-exif-filter/plan.md`). It belongs to the
  wrap-up because that path exists only after the archive move; in single-PR
  mode it lands in the same PR.
- The wrap-up's todo curation removes the three `todo.md` items this plan
  closes ("Native: the per-folder EXIF filter sections in the filter menu
  (closes port Step 7)", "Native: the pane's `n / total` lags a refilter that
  hides the shown file", "Native: a menu change while the filter hides every
  file lands on the first file") and carries the pending GUI checks of
  `learnings.md` into `todo.md`. The implementation does not edit `todo.md`.

### Decisions (the user, 2026-11-13)

The user approved the plan with the defaults of every trade-off below: one
step; `Camera` / `Lens` ordered by a case-insensitive compare with a plain
compare as the tie-break; the kept anchor used by `menu_action` only; the
pane showing the pager's current index (option (a)); the status line's
`n/total` left alone; no A/B.

### What the investigation found (2026-11-13, `main` at `cc6db501`)

- Tauri source of the sections: `crates/app/ui/src/exif.ts` (`exifKey`,
  `focalRanges`, `focalRange`), `crates/app/ui/src/filter.ts` (`passes`'s
  `exif` clause, `applyStored` leaving `exif`), and `main.ts`'s
  `rebuildExifMenu` (untested there). The rules:
  - Groups in this order with these headings: `Camera`, `Lens`, `Aperture`,
    `Shutter speed`, `ISO`, `Focal length`. They come after every other
    section (`#filter-exif` is the last child of `#filter-menu`, after
    `#filter-output` and `#filter-genre`), each as a separator, a heading and
    its items. A group with no label in the folder is left out entirely.
  - One item per distinct label among the folder's index entries
    (`IndexedFile.exif`, all entries of the folder, not the shown list). The
    key is the label, not the value. Focal length is keyed by its range's
    label (`<24 mm`, `24–35 mm`, `35–50 mm`, `50–85 mm`, `85–135 mm`,
    `135–200 mm`, `>200 mm`, half-open `[lower, upper)`).
  - Order within a group: numeric groups by value, ties by label; camera and
    lens by text (`localeCompare`). When two entries share a label with
    different values, the last entry in the index's order gives the value.
  - A checked label no longer present after a rebuild is dropped, so a stale
    check cannot hide everything. The rebuild runs when the entries are read
    (the open, pass 1's refresh, a re-listing's refresh).
  - A folder open (not a re-listing) clears every EXIF check. `Reset` clears
    them too. They count toward the filter button's active state. They are
    never stored: `StoredFilter` has no `exif`, and the native app writes
    nothing to `settings.json` anyway.
  - `passes`: per group with a non-empty set, the file's key must exist and
    its label be in the set; a file without EXIF fails any non-empty group.
- `crates/backend/src/exif.rs` already gives `Exif` and `Labeled`
  (`value` to sort by, `label` to show) on `IndexedFile.exif`; that is the
  shared display formatting to reuse. The focal ranges exist only in
  `exif.ts`.
- Native side today: `filter::Facts` holds no EXIF; `menu::Row`, `Item`,
  `Label`, `Toggle` and `Action` are all `Copy` over `&'static str`, which a
  per-folder label cannot be; `FilterState::is_active` and `menu::apply`'s
  `Reset` (`FilterState::default()`) list the sections by hand.
  `Riffle::set_entries` is the one place `facts` is replaced (the open, a
  re-listing, pass 1's `Done` through `read_entries`), and `entries_of`
  already walks the `IndexedFile`s on a background task.
- The pane bug: `meta_pane` prints `shown.0 + 1` over `files.len()`.
  `refilter` remaps `shown.0` only when the shown file is still in the new
  list; when it is hidden the index stays stale until the new preview's
  `loaded` replaces `shown`. The Tauri pane prints the current index
  (`positionEl`: `index + 1` over `files.length`), not the shown file's.
- The empty-list bug: `menu_action` passes `current_path()`, which is `None`
  while `files` is empty, so `refilter` falls to index 0. **The Tauri
  `refilter` does the same** (its anchor defaults to `files[index]`,
  `undefined` on an empty list), contrary to what the `todo.md` item says, so
  this fix is a native improvement rather than a port.
- Tests to port: `exif.test.ts` (the `focalRange` table of 10 rows and the 5
  `exifKey` cases) and `filter.test.ts`'s "an EXIF selection matches by label
  and fails a file without EXIF" and "applyStored leaves the EXIF groups
  alone". `rebuildExifMenu` has no Tauri test; its rules above get new native
  tests.

### A/B latency measurement: not needed

Per [native-app.md](../../agents/native-app.md)'s "A same-session A/B gates
only work on the page path", this plan is on the off-page-path side:

- The EXIF groups are built only when the entries are read (the open, a
  re-listing, pass 1's `Done`), never per page, and add no read: they reuse
  the `Index::entries` rows already fetched.
- The menu's rows are built and laid out only while the menu is open.
- `passes` gains a clause that is skipped while no EXIF item is checked, and
  `refilter` never runs per page.
- The pane fix changes which number the existing position line formats; it
  adds no element. The filter-sort plan's A/B already showed the filter, the
  sort and the two buttons cost the pager nothing measurable.

If the implementation ends up adding per-page work or a new drawn element
after all, stop and report: an A/B needs the user's explicit permission (the
machine idle for 30 to 60 minutes).

## Steps

- [x] Step 1: The EXIF filter sections, the pane's position after a refilter, and the anchor kept across an empty list
  - Done when:
    - The native filter menu shows the EXIF sections after `Output` with the
      Tauri headings, grouping, keys, order and check semantics listed above;
      a checked item narrows the strip through `refilter`; `Reset` and
      another folder's open clear the checks; a rebuild drops a check whose
      label is gone; the filter button is lit while any EXIF item is checked;
      nothing is written to `settings.json`.
    - Unit tests: the ported `exif.test.ts` cases, the two ported
      `filter.test.ts` cases, and new cases for the group build (order per
      group, a shared label taking the last entry's value, an empty group
      left out, pruning), the menu rows (sections last, headings, checked
      state, a toggle flipping its item, `Reset` clearing, `is_active`).
      Update the test modules' doc comments that name these cases as left
      out.
    - The pane's `n / total` shows the new current file's position and the
      new total at once after a refilter that hides the shown file (`p` under
      the `untagged` filter), with a unit test on the pure rule that fails
      before the fix.
    - A menu change (`Reset` included) made while the filter hides every file
      lands on the file that was current before the list emptied when it
      passes again, else on its `anchor_after_filter` neighbour, with a unit
      test on the pure rule that fails before the fix.
    - `mise run native:check` (in bash, with
      `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH"`) and root
      `mise run ci` pass.
    - `docs/agents/native-app.md` records the new rules (in "The listing and
      the shown list are two lists…" and "The sort and filter menus…"), and
      `CLAUDE.md`'s `crates/native` paragraph no longer says the EXIF
      sections are left out.
    - `learnings.md` lists the GUI checks below as pending for the user, and
      states that no A/B was run and why.
  - Implementation approach (as far as it is known; omit if unknown):
    - `crates/native` must not depend on `crates/app`. No Tauri-side change,
      smoke check or compat shim.
    - `crates/native/src/main.rs` and `CLAUDE.md`'s `crates/native` paragraph
      are being edited by parallel sessions (folder tree, MCP / log, settings
      screen, folder-op dialogs). Keep the diff there to the call sites named
      below and put the logic in `exif.rs`, `filter.rs` and `menu.rs`. Bring
      `main` in with a merge commit (never a rebase or force push) before the
      PR if it conflicts.
    - Port `exif.ts` as `crates/native/src/exif.rs` (the port convention: one
      `.rs` per `.ts`; one `mod exif;` line in `main.rs`): the group enum in
      menu order with its heading, the focal ranges and `focal_range`, and
      `exif_key` over `riffle_backend::exif::{Exif, Labeled}` giving the
      label and an order that is either a number or text. If a new module is
      too much churn against the parallel sessions, the same items may live
      in `filter.rs` instead; say which in `learnings.md`.
    - `filter.rs`: `Facts` gains what `passes` needs of the file's EXIF (the
      `Exif` itself, or only the six keys; `Facts::from(&IndexedFile)` fills
      it). `FilterState` gains the per-group label sets. `passes` gains the
      clause, reading the file's EXIF through the `facts` it already takes.
      `is_active` counts the sets. `apply_stored` leaves them. Add a method
      that clears them for another folder's open and one that prunes them
      against the folder's groups.
      - `FilterState` derives `PartialEq`, and `menu.rs`'s
        `every_toggle_flips_its_item_and_leaves_the_sort` asserts a toggled
        on-then-off state equals `FilterState::default()`. A map from group
        to set must therefore not keep an emptied set behind (remove the
        entry, or use one fixed set per group).
    - The folder's groups (per group, its labels in display order) come from
      a pure function over the entries in the index's order, so the "last
      entry's value" rule matches Tauri and does not depend on `HashMap`
      iteration. `entries_of` already walks the `IndexedFile`s on the
      background task, so build them there and hand them to `set_entries`
      with the facts (the `Entries` tuple grows by one member). `set_entries`
      stores them and prunes the checks, which keeps the prune ahead of every
      `refilter` that follows it (`opened`'s carry branch, `read_entries`).
      Numeric order is by `f64::total_cmp`, ties by label; text order (camera,
      lens) by a case-insensitive compare with a plain compare as the
      tie-break (the user's decision).
    - `opened`'s non-carry branch (another folder) clears the EXIF checks
      before `set_entries`, as the Tauri `openDirectory` does; a re-listing
      (carry) only prunes.
    - `menu.rs`: `filter_rows` takes the folder's groups and appends, after
      the `Output` section, a separator, the heading and the items of each
      non-empty group. Key a toggle by its group and label, as the Tauri
      `data-group` / `data-value`, not by a row index: the groups can be
      rebuilt between the frame that drew the menu and the click. That makes
      `Label`, `Toggle`, `Action`, `Item` and `Row` `Clone` instead of `Copy`
      (an owned label variant next to `Label::Text(&'static str)`); adjust
      `apply`, `is_checked`, `Menus::acted` and the tests' copies. `Reset`
      already clears through `FilterState::default()`. Keep `menu.rs` free of
      GPUI types.
    - `main.rs`: `menu_row` renders the owned label and clones the action
      into its click listener; `menu_button` passes the stored groups to
      `filter_rows`. Nothing else in the popover changes (it already scrolls
      under the window-height cap, which the longer menu now relies on).
    - The pane's position: the pane reads the pager's current index over
      `files.len()`, as the Tauri `positionEl` (the user's decision, option
      (a)). Put the rule in a pure function with the test. Check during
      implementation what else reads `shown.0` after a refilter hid the shown
      file: the next `refilter` takes `old.get(at)` as the shown file's path,
      and with a stale index that can name the current file and skip the
      `shown != current_path()` re-request after the `order` bump dropped the
      read still out. This was read from the code and not reproduced. If it
      is real, fix it with the same change (for example by keeping the shown
      file's path rather than trusting the index) and add a test; if the fix
      grows beyond a few lines in `main.rs`, record it in `learnings.md` for
      `todo.md` instead. The status line's `n/total` is left alone.
    - The kept anchor: a small pure type in `filter.rs` that remembers the
      anchor when a refilter leaves `files` empty and gives it back while
      `current_path()` is `None`. `menu_action` uses it (only it, the user's
      decision); it is cleared by another folder's open and once a refilter
      leaves the list non-empty. A kept path that is no longer listed must
      still fall back to index 0 (`anchor_after_filter` returns a path `next`
      does not hold; the existing `unwrap_or(0)` covers it; add the case to
      the test).
    - Verification beyond unit tests: `--auto` cannot click a menu or press a
      judgment key, and the EXIF checks are session-only, so no headless run
      can exercise them. Do not add a flag for it. One debug `--auto` run on
      a copied folder with an existing `--settings` filter file is enough to
      show the open, pass 1's `Done` and the paging are unchanged. Use only
      copies under `D:\Photos\tests\20261113-exif-filter\` (or the existing
      copies under `D:\Photos\tests\20261113-filter-sort\`), always pass a
      fresh `--index` and a test `--settings`, never open another folder
      under `D:\Photos\`, never modify files outside `D:\Photos\tests\`, and
      never kill a process.
    - Optional and cheap, as the filter-sort plan did for `order_files`: time
      the group build over a few thousand synthetic entries in a throwaway
      test and record the number in `learnings.md`.
    - Pending GUI checks to record in `learnings.md` (Windows, the debug
      build, a folder with several cameras such as the filter-sort plan's
      `sort\raw` copies):
      - The filter menu ends with `Camera`, `Lens`, `Aperture`, `Shutter
        speed`, `ISO`, `Focal length`, each listing the folder's values in
        order; it scrolls when taller than the window.
      - Checking one camera narrows the strip and lights the button; a second
        camera widens it (OR); an ISO item on top narrows it (AND); `Reset`
        clears all.
      - On a fresh index the sections appear once pass 1 ends.
      - Copying in a RAW from another camera adds its item after the re-scan;
        removing the only file of a checked camera drops that check instead
        of leaving an empty strip.
      - Under the `untagged` filter, `p` on the current file updates the
        pane's `n / total` at once.
      - With a filter that hides every file, `Reset` or unchecking the item
        comes back to the file shown before, not the first one.
      - The settings file is untouched.

## Trade-offs and risks

- **One step or two.** The plan puts the EXIF sections and the two bug fixes
  in one PR: they share `refilter`, `menu_action` and the menu, and `main.rs`
  is under parallel edits, so one pass over it is cheaper. The alternative was
  two steps (the two fixes first, then the sections).
- **Text order of `Camera` and `Lens`.** Tauri sorts them with
  `localeCompare` (ICU collation); `crates/native` has no collator. The chosen
  case-insensitive compare matches ICU for the ASCII model names seen so far
  but is not guaranteed for punctuation or non-ASCII. `natord` would order
  `ILCE-9` before `ILCE-10` unlike Tauri; `icu_collator` would add a
  dependency.
- **The kept anchor is not Tauri's behaviour.** The Tauri `refilter` also
  lands on the first file after an empty list, so `todo.md`'s wording is
  inaccurate and the native app differs on purpose. Applying it to every
  `refilter` anchored on `current_path()` would also fix a fresh-index open
  under a `Face focus` filter landing on the first file, but changes more
  paths; left out.
- **Which number the pane shows.** The pager's current index (as Tauri) fixes
  every lag of this kind, including the re-listing one, but while a preview is
  loading the position can lead the file name by one load during paging.
- **The status line's `n/total`.** It has the same lag and is not in the todo
  item; left alone.
- **No A/B.** Stated above. The risk is a wrong classification; the guard is
  that the group build sits on the existing background entries read and the
  optional timing lands in `learnings.md`.
- **A failed entries read drops the EXIF checks.** `opened` falls back to
  empty entries when the index read fails, which empties the groups and so
  prunes every check. Accepted as the existing failure mode.
- **Parallel sessions.** `main.rs` and `CLAUDE.md` conflicts are likely;
  keeping the call-site diff small is the mitigation, not a guarantee.
- **GUI behaviour is not verified by the implementation.** Clicks cannot be
  driven headlessly; the checks stay pending for the user in `learnings.md`
  and then `todo.md`.

## Progress
