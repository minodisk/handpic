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

# todo.md cleanup after the 2026-11-19 Windows real-device run

## Purpose

The root `todo.md` holds items that are obsolete, done, duplicated or
superseded, and checkboxes that a real-device run on Windows on 2026-11-19
(a screenshot-feature debug build on test copies, driven over MCP on
`--mcp-port 41931`) verified. Removing them leaves only open work in the
file. The same run found one new bug (the first launch on a fresh index
shows the startup folder empty), which gets its own item. This is a
docs-only change: one step, one PR, `todo.md` only.

## Steps

- [x] Step 1: Apply the deletions, trims, merges and the one new item to `todo.md`
  - Done when:
    - Every edit in the lists A, B and C below is applied exactly as written, the notes of the merged items (A3, A4) are in the kept items, and the new item (C) exists.
    - `git diff origin/main -- todo.md` shows no change to any other item, and the PR changes no file other than `todo.md` and this plan's files.
    - No two `###` items are left without a blank line between them and no doubled blank lines are left where an item was removed.
    - `mise run ci:local` passes locally (a `.md` change runs lychee) and the PR's GitHub checks pass.
    - The PR description records why each item or box went (the "Reason" notes below), since `todo.md` itself no longer holds them.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. Each heading below occurred exactly once in `todo.md` at `091ea31c`. An item runs from its `###` line to the line before the next `###`.
    - "Box N" counts the top-level `- [ ]` lines under the item's `#### TODO`, from 1. A box's indented sub-bullets belong to it and go with it.
    - Do not reword the Background of an item whose boxes are trimmed, even where it now describes more than the remaining boxes (see "Trade-offs and risks").
    - **A. Delete the whole item (heading, Background, Files, TODO):**
      1. `### Native: real-device check that the app runs as before after the move to gpui-pre and gpui-component`. Reason: obsolete. Its premise that `gpui_component::init` is not called is false (`main.rs` calls `gpui_kit::init`); the gpui-component adoption and gpui-kit umbrella items cover it.
      2. `### Native: read the native CI job's build time per leg against the 30-minute timeout after the move to gpui-pre`. Reason: done. CI run 43548232955 native legs took 8m54s / 5m47s / 8m56s; the recent main run 44317498801 Windows leg took 17m23s; all under 30 minutes, no change needed.
      3. `### Docs: the heading "gpui-component is initialized; the settings modal and the tree's inline rename draw from it" in native-app.md is stale`. Reason: duplicate of `### Native app docs: correct the stale "only the settings modal and the tree's inline rename draw from gpui-component" text`. Before deleting, merge its unique notes into the kept item:
         - Append to the kept item's Background paragraph, before its `Files:` sentence: `The heading is at about line 71 of docs/agents/native-app.md. Other places refer to the section by its exact heading name, for example the mention "table in "gpui-component is initialized"" near line 3227 of the same file. The native-progress-bar-reason feature fixed the same stale claim in the progress bar section's sentence and left the heading alone on purpose. Plan: docs/plans/_archived/20261115-native-progress-bar-reason/plan.md.` (write the file and plan paths in backticks, as the rest of `todo.md` does; the deleted item's "about line 48" and "near line 1900" are stale, the current lines are 71 and 3227).
         - Replace the kept item's one box with: ``- [ ] Reword the "gpui-component is initialized; ..." heading so it names the dialogs, tooltips and scrollbars as well, or make it generic. Update every quoted-title reference to the old heading name, found with `rg -n 'gpui-component is initialized' docs CLAUDE.md` (one is near `docs/agents/native-app.md:3227`), and check that `mise run lint` (lychee) still passes.``
      4. `### Native: real-device check of the Sort / Filter buttons drawn with gpui-component's `Button` (native-component-controls)`. Reason: superseded by `### Native: real-device checks of the strip's header row with Sort and Filter (native-strip-header)`. Before deleting, merge its two unique details into box 1 of that item by replacing the box's last sentence ("The filter button is lit while a filter is active, and the sort button is dimmed in a view-only (JPEG-only) folder.") with: ``The filter button is lit while a filter is active (its border goes to `input` while hovered, a known `Button` hover limit), and the sort button is dimmed in a view-only (JPEG-only) folder, where it does not hover and does not open.`` Do not delete the neighbouring `### Native: real-device check of the settings controls drawn with gpui-component (native-component-controls)`, whose heading is similar.
      5. `### Native: real-device checks of the MCP companion (read, navigate and set_judgment tools)`. Reason: verified on 2026-11-19 on Windows with a screenshot-feature debug build on test copies (an HTTP MCP client on `--mcp-port 41931`): `tools/list` returned 36 tools; `show_photo` moved the preview and the strip; `set_view` `zoom` / `normal` entered and left 1:1; `set_judgment` rating 3 on two paths and pick on the current file showed in the strip cells without advancing; one undo reverted both ("Undid 2 files") and the `.xmp` followed; a `WM_CLOSE` left `quit: judgment stores drained=true sidecar writer drained=true` and `quit: MCP server stopped=true`. The meta pane expectation was stale (the native meta pane has no judgment rows).
    - **B. Remove only these boxes or clauses:**
      6. `### Native: real-device checks of the strip, folder tree and meta pane scrollbars`: delete box 1 (bars fade after about 2 s; obsolete since `ScrollbarMode::Always`), box 2 (duplicates box 2 of the always-visible scrollbars item) and box 6 (duplicates its box 3). In box 4 delete the sentence "With the bar faded out, press in its place and expect the cell to be selected as before.", leaving: `- [ ] On Windows, on the strip, press on the shown bar. Expect the current file and the selection not to change. Expect the cells' tooltips to still show on hover.` Keep boxes 3, 5, 7 and 8 unchanged.
      7. `### Native: real-device checks of the gpui-component adoption (theme, Input, controls, menus, Root, the three Dialogs)`: delete the box starting "On Windows (Step 3: settings controls)" with its sub-bullets, and the box starting "On Windows (Step 8: settings modal as a `Dialog`)" with its sub-bullets (the last lines of the item). Reason: covered by the settings-body, settings-controls and full-window-layer items. Keep the Step 1, 2, 2 catch-up, 4, 5 (Windows, macOS, Linux), 6 and 7 boxes.
      8. `### Native: real-device checks of the MCP settings page and a live theme change over MCP`: in its one box delete part (2) ("With the settings closed, call `set_settings {"theme":...}`: expect the window to repaint in Ayu Light at once.") and renumber "(3)" to "(2)". Reason: verified, `set_settings` theme light / Ayu Light repainted at once.
      9. `### Native: real-device check of the performance overlay on `d``: delete box 3 (press `z` with the overlay on). Reason: verified, the band sits over the fit and 1:1 views without moving them.
      10. `### Native: real-device checks of the pane toggles and Open Folder (gpui-pane-toggles)`: delete box 2 (the four toggle keys, each twice) and box 4 (hide the right pane and the strip, quit, relaunch). Reason: box 2 verified via MCP `set_panes`, and its Sort / Filter-with-the-strip-hidden part is obsolete; box 4 verified (right pane and strip hidden, closed via `WM_CLOSE`, the relaunch restored them hidden with the last folder and file).
      11. `### Native: real-device checks of the empty state in the preview area`: delete box 3 (a filter that hides every file; verified). Trim box 2 to: `- [ ] On Windows, open a folder without RAW or JPEG files. Expect a click on the "This folder has no RAW or JPEG files." message to do nothing.` (the message itself was verified).
      12. `### Native: real-device check of the strip position in the status bar`: delete box 1 and box 3. Reason: box 1 verified (`6 / 12`, `1 / 1 (12)` under a filter, nothing with no shown file; its Sort / Filter placement is obsolete); box 3 verified. Keep box 2.
      13. `### Native: real-device checks of the burst bands, badges, burst keys and reject the rest`: in both boxes change the leading "On Windows and macOS" to "On macOS" (Windows verified). Leave the rest of each box, the `Cmd+Z` notes included.
    - **C. Add one new item** directly after `### Native: real-device checks of the strip, the scan progress, the resume and the resync` (before `### Native: measure the scan pause in `load` with presses on files pass 1 has not read yet`), in the file's item format:

      ```markdown
      ### Native: the first launch on a fresh index shows the startup folder empty

      #### Background

      On 2026-11-19 a debug build launched as `handpic.exe <folder> --index <new file> --settings <new file> --import-settings <seed> --log <file> --mcp-port 41931` on a folder of 12 ARW files logged `scan list: files=12`, a reconcile with todo=12, and pass 1 and pass 2 with done=12, and the tree showed 12. But the MCP `get_view` kept `count: 0`, the preview area said "This folder has no RAW or JPEG files." and the status line said `no RAW files in <folder>`, while `get_view`'s `filter_offers` did list the files' EXIF values. Opening another folder and coming back listed all 12, and a relaunch on the now-filled index restored all 12, so the failure seems limited to a fresh index.

      Files: `crates/native/src/main.rs` (the startup open and the listing path), `crates/native/src/scan.rs`, `crates/native/src/uitest.rs`.

      #### TODO

      - [ ] Reproduce with a fresh `--index` and a folder on the command line. Find why the first listing's entries do not reach `all` / `files`, and fix it.
      - [ ] Add a UI test in `uitest.rs` that launches on a fresh index with the folder given at startup (`Ui::open` launches with no folder and opens one afterwards, the path that worked) and expects the files listed.
      ```

## Trade-offs and risks

- **Renumbering in B8.** "(3)" becomes "(2)" after part (2) goes, so the box does not read as if a part were missing (decided with the user).
- **Wording of the trimmed empty-state box (B11).** The box names the message inside the click expectation so it still reads on its own.
- **Backgrounds left stale.** Only boxes are removed, so some Backgrounds now say more than their TODO (the scrollbars item's fade-out text, the adoption heading's "the three Dialogs", the status position item's old Sort / Filter placement). Rewording them is a possible follow-up.
- **The verification evidence leaves the repository.** The deleted items' and boxes' results live only in the PR description after this change.
- **origin/main moved** from `091ea31c` to `7619ce6d` before the branch was cut; locate items by heading, and if a named heading is gone or changed, report it rather than guessing.

## Progress

- (2026-11-19) Step 1 complete
