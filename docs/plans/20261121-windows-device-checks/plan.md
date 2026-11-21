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

# Record the 2026-11-21 Windows real-device checks

## Purpose

A real-device run on Windows 11 on 2026-11-21 exercised many of the pending
manual checks in the root `todo.md`: the meta pane, the strip marks and key
names, the sidecar writes, undo / redo, the chunked Move to Trash, and the
MCP paths that need the user's click. This plan removes the items the run
settled, rewords the ones it settled in part, records what was seen in
`docs/agents/app.md` where an item asks for it, and adds the four problems
the run found as new items. Docs and todo only, no code change: one step,
one PR.

## Steps

- [x] Step 1: Apply the run's results to `todo.md` and `docs/agents/app.md`
  - Done when:
    - Every edit in the lists A, B, C and D below is applied.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files, and no `todo.md` item outside the lists changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where an item went.
    - The PR description records what was seen for each removed item (the "Seen" notes below), since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally (a `.md` change runs lychee) and the PR's GitHub checks pass.
  - Implementation approach:
    - Locate every item by its `###` heading, never by line number. An item runs from its `###` line to the line before the next `###`. "Box N" counts the top-level `- [ ]` lines of the item, from 1.
    - Write everything in English, in the file's own style (`#### Background`, `Files:`, `#### TODO`, paths and identifiers in backticks). Keep the neighbouring items byte-for-byte.
    - The texts quoted below are the content to carry; their facts and quoted strings are exact, the surrounding wording may follow the file's style.
    - **A. Delete the whole item in `todo.md`:**
      1. `### App: real-device check of the chunked Move to Trash on Windows (trash-over-scan Step 4)`. Seen: box 1, 605 files moved in 2385 ms (3.9 ms a file, under `trash-bench.md`'s 7 ms), the status count in steps of 100, Undo restored every `.xmp` with its RAW, `Ctrl+Shift+Z` moved the 605 again in 1873 ms. Box 2, a file held open with no sharing was listed as a failure (`the source is in use by another process (0x80270027)`) with its `.xmp` left, the other 10 moved, Undo restored them with their sidecars.
      2. `### App: real-device checks of the meta pane, the judgment keys, the sidecar writes and undo / redo`, all four boxes; what was seen goes to `docs/agents/app.md` (list D). Seen for the meta pane box, which asks for no record: `D:\Photos\tests\20261108-gpui-compare\ARW` showed the file name, the `EXIF` group and a Sony `Maker note` group, the arrows updating them; the position shows in the status bar as `n / 20`, no longer in the pane; the JPEG folder showed `EXIF` only, no `Maker note`, and `JPEG folder: view only`. Its optional part (3), the corrupt file, was not done and is dropped.
      3. `### App: real-device check of the MCP busy guard's wiring (settings, trash dialog, folder dialog, inline rename)`. Seen: `navigate` was refused in all four states while `get_view` answered, with "Handpic's settings are open; try again once the user closes them", "a Move to Trash or Delete Permanently is under way in Handpic; try again once it ends", "a folder operation's dialog is open in Handpic; try again once the user closes it" and "a folder name is being edited in Handpic; try again once the user is done".
      4. `### App: real-device checks of the MCP settings page and a live theme change over MCP`. Seen: both parts pass.
      5. `### App: real-device check of Move to Trash over MCP with `mcpAllowDestructive` off, and the redo refusal`. Seen: all three parts pass.
      6. `### App: real-device check of a folder operation confirmed in the app's dialog after an MCP call (Rewrite Sidecars, Delete Sidecars, Sequence)`. Seen: Rewrite gave "Rewrote the sidecars of 7 files in a", Delete Sidecars "Moved 7 sidecars to the Trash", Sequence's Run wrote the `-sequenced` folder and revealed it in Explorer.
      7. `### App: real-device check of Import, Rename by Capture Date and the folder rename confirmed in the app after an MCP call`, both boxes. Seen: box 1's three parts pass; box 2, the running dialog came back with Run disabled, Cancel enabled and `importing 6 / 44`.
      8. `### App: real-device check of the Clear Cache dialog opened over MCP, and the grayscale latch on screen`. Seen: pass. Note: while the folder tree holds the keyboard, `g` is its type-ahead, so the latch stays; after Escape a press and release of `g` ends it. This note goes in the PR description only.
    - **B. Reword in `todo.md`:**
      9. `### App: real-device check of `open_folder` deferred by a run and refused during a rename over MCP`: part (1) passed, part (2) was not done. Rename the heading to `### App: real-device check of `open_folder` refused during a rename over MCP`. In the Background, replace "Neither state was reached end to end, because each needs the user's input." with a sentence saying the deferred open was seen on Windows 11 on 2026-11-21 (a Move to Trash of 605 files, `open_folder` mid-run at 100 / 605: `opening` set, `idle: false`, then `wait_for_idle` returned with the folder open) and the rename refusal was not reached. Replace the box with: ``- [ ] On Windows, run the app with MCP on a test port as `docs/agents/app.md` prescribes. Rename a folder in the tree and, while the rename is in flight (a slow share helps), call `open_folder` on it: expect `A folder is being renamed; try again in a moment.` ``
      10. `### Engine: Undo of Move to Trash is still one shell operation per item`: delete box 1 ("Time Undo of a 500+ file move on Windows."). In the Background, replace the sentence "Not timed, but if an operation costs what a move did (about 200 ms), an Undo of a large move takes far longer than the move." with: "Timed in the app on Windows 11 on 2026-11-21: an Undo of a 605-file move took 85.6 s (141 ms a file) against 2.4 s for the move." Replace box 2 with: `- [ ] Restore in chunks while keeping the RAW-before-sidecar and never-overwrite rules (the baseline to beat: 85.6 s for 605 files, 141 ms a file, against 2.4 s for the move).`
      11. `### Engine: Delete Permanently cannot remove a read-only file on some platforms`: delete box 1. In the Background, replace "Not tried." with: "On Windows 11 on 2026-11-21, `Delete Permanently…` removed a read-only (`attrib +r`) ARW without an error; macOS and Linux are not tried." Replace box 2 with: ``- [ ] On macOS and Linux, delete a read-only file with `Delete Permanently…` and see whether it fails. If it fails, decide whether to clear the read-only flag first or leave it.``
    - **C. Add four items to `todo.md`**, each a `###` section with `#### Background` (ending in a `Files:` sentence) and `#### TODO`, under `## Cross-cutting / other`. Suggested places, each next to a related App item: C1 after `### App: real-device check of Left / Right paging the strip and Space toggling from the folder tree (native-tree-left-right)`; C2 after `### App: real-device checks of Import… (gpui-folder-dialogs Step 5)`; C3 after the reworded `open_folder` item (B9); C4 after `### App: real-device checks of Rewrite Sidecars from Index… and Delete Sidecars… and the folder menu (gpui-folder-dialogs Step 3)`. Each Background says it was seen on Windows 11 on 2026-11-21.
      - C1 `### App: `ctrl+,` does not open the settings while the folder tree holds the keyboard`. Background: after a click on a tree row, or after an inline rename, `ctrl+,` does nothing; after Escape it opens the settings; the title bar's gear button works throughout. The tree's gate lets through only `TREE_PASSTHROUGH`. Files: `crates/app/src/treekeys.rs` (`TREE_PASSTHROUGH`, `tree_gate`), `crates/app/src/main.rs` (`key_down`). TODO: decide whether `ctrl+,` should pass the tree's gate; if so, let it through and pin it in `treekeys.rs`'s test.
      - C2 `### App: the Import dialog's `Source:` line shows the Windows verbatim prefix`. Background: an import started over MCP (`import_folder`) with an index-keyed path showed `Source: \\?\D:\...` in the dialog; its destination row did not carry the prefix. The line is built from `preview.source` as is; the Sequence dialog's `Source:` line is built from `dir` the same way. `context.rs`'s `shown_path` strips the prefix for display elsewhere. Files: `crates/app/src/folderops.rs` (the `Source:` lines), `crates/app/src/context.rs` (`shown_path`), `crates/engine/src/import.rs`. TODO: check whether the tree's right-click `Import…` shows the prefix too, and strip it for display with `shown_path`.
      - C3 `### App: the pane's error list keeps an earlier Move to Trash failure after later operations finish`. Background: after a Move to Trash listed a failure, `get_status`'s `errors` (the pane's notes, `pane_errors`) still held that line after later operations had finished. Files: `crates/app/src/main.rs` (`pane_errors`), `crates/app/src/companion.rs` (`Status`). TODO: decide whether a new run clears the earlier run's failure lines, and apply it.
      - C4 `### App: Rewrite Sidecars from Index reported "4 files not in the index (skipped)" in a fully scanned folder`. Background: the preview's row said `4 files not in the index (skipped)` in a folder whose scan had finished; those files had no `ratings` row. The engine counts `skipped` as the listed RAW files less the rows it will write. Files: `crates/app/src/sidecars.rs` (the row's text), `crates/engine/src/foldersidecars.rs` (`targets`, `preview`). TODO: check whether the wording matches what is counted (a scanned file without a judgment is in the index), and reword the row or change the count. Do not state a cause.
    - **D. Record in `docs/agents/app.md`** (find each section by its heading; keep the headings and their tags unchanged):
      - "Every keystroke goes through the keymap": the paragraph saying the names are read from `gpui_windows` and not yet seen in the running app is no longer true. Say the names were seen in the running app on Windows 11 on 2026-11-21 under `RUST_LOG=debug`, and list them: `shift+x` -> `rejectRest`, `ctrl+alt+1` -> `red`, `ctrl+alt+0` -> `clearlabel`, `ctrl+z` -> `undo`, `ctrl+shift+z` -> `redo`, `1` -> `rate1`, `x` -> `reject`, Shift+1 -> `!` with no action, `arrowright` -> `next`, `arrowleft` -> `previous`. Ctrl+Alt+digit reached the window; the layout has no AltGr, so the AZERTY and AltGr bullets stay as read from the source, not seen.
      - "The writer drains in `App::on_app_quit`": replace the sentence leaving the window-close path and a key inside the debounce to the user's hand check with what was seen on 2026-11-21: `5` then `q` at once logged `quit: judgment stores drained=true sidecar writer drained=true` and the sidecar held `Rating="5"`; the same line and value with the title bar's close button. Also the other sidecar-write results: `3` / `p` / `Ctrl+Alt+1` gave `xmp:Rating="3"`, `xmpDM:good="True"`, `photoshop:LabelColor="red"`, `xmp:Label="Red"`; `0` / `u` / `Ctrl+Alt+0` gave `Rating="0"`, no `good`, no label (`0` does not auto-advance); `1`, `2`, `4` within about 150 ms on the last file gave one mtime change and the final value; with `both`, `.xmp` and `.arw.dop` held `Rating = 3`, `ShouldProcess = 0`, `ColorLabel = "Red"` and random Uuids; with `rot.json`, `xmp:Label="Rot"`; `4` then a forced kill by PID within about 100 ms left no sidecar, and the reopen on the same index logged `scan sidecars: ... dirty=1` and wrote `Rating="4"`; auto-advance: `3` moves on and stays on the last file, `Ctrl+Alt+2` stays, the arrows wrap both ways; in a JPEG folder `3` does nothing and writes no file.
      - "A judgment goes view, row, writer": add that the undo / redo hand check passed on 2026-11-21 on the four-ARW `undo` folder: `Ctrl+Z` twice walked back with the notes `Undid DSC-RX0_DSC00009.ARW` and `Undid DSC-RX0M2_sony_rx0_ii_40.arw`, each `.xmp` following within a second; `Ctrl+Shift+Z` gave `Redid ...` and the stars back; a new key then `Ctrl+Shift+Z` did nothing; after `q` and a new index `Ctrl+Z` did nothing, and nothing in the JPEG folder.
      - The strip marks (L1005036 three stars, green dot, yellow band; 037-040 four to one stars; 041 green dot; 043 red dot and dimmed; 044 red band; stars readable on a bright thumbnail; with `both.json` on a copy with mtimes kept, 036 showing the newer `.xmp`'s 3 stars against its `.dop`'s 5, so newest-wins holds) and the obsolete "nothing is written" expectation go in the PR description only.

## Decisions

- Results no item asks to record go in the PR description only, as the 2026-11-19 cleanup did.
- The grayscale latch note goes in the PR description only.
- B10 / B11 also replace the one stale Background sentence each.
- "Every keystroke goes through the keymap" keeps its (Inferred) tag.
- Headings are from `main`; if one no longer matches (main moved since), find the item by its text and note it in `learnings.md` rather than skipping it.

## Progress

- (2026-11-21) Step 1 complete
