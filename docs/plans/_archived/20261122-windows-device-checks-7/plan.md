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

# Record the seventh batch of Windows real-device checks (2026-11-22, empty state, status row, scrollbars and the strip header)

## Purpose

A seventh real-device run on Windows 11 exercises the open Windows boxes of
six `todo.md` items (the file is at the repository root): the empty state in
the preview area, the status row's `StatusBar` and progress circle, the status
row with no current-file info, the strip position in the status bar, the
always-visible scrollbars, and the strip's header row with `Sort ▾` and
`Filter ▾`. This plan runs them, removes the boxes the run settles, narrows
the rest to what stays open and why, adds whatever the run finds as new
items, and replaces the "inferred / not re-measured" notes of
`docs/agents/app.md` with measured values. Docs and todo only, no code
change: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-6/plan.md`.

Like batch 6, this plan is written before the run. The run's results go to
`results.md` in this folder. They are the source of truth for every edit:
lists A to G below give the rule per box, and are tightened to the exact
edits once `results.md` exists.

## Rule for sub-points the run did not exercise

A sub-point of a box that `results.md` does not mention, or lists as not run,
too fast to see or not comparable, was **not** run and must stay open: carry
it into the reworded box with a short note of why (or keep the box). A box is
deleted only when every one of its sub-points was exercised and passed. A
sub-point that failed stays open or becomes a new item (list H). A box that
asks for a judgment ("reads well", "acceptable", "not visibly slower") is
closed on the session's own judgment from captures, and `results.md` and the
item's Background say it was judged that way, on a debug build. The macOS /
Linux boxes of every item are left untouched. Cases to expect (check for
others while editing):

- Pass 1's circle and the scan suffix need files not in the OS cache; a set
  of cached files ends pass 1 before the window takes keys.
- `DROP_TEXT` needs a real OS drag onto the window; no earlier batch made
  one.
- The resize cursor is not in a window capture; it counts only if read
  another way (e.g. `GetCursorInfo`) and `results.md` says how.
- "On an OS appearance change" in System mode (see Trade-offs).
- "In every pane layout": counts only for the layouts whose canvas size was
  recorded with the circle shown and gone.
- Hovered and dragged thumb states need a capture taken while the pointer is
  there or the button is down.

## Steps

- [x] Step 1: Run the six items' Windows checks, write `results.md`, and apply the results to `todo.md` and `docs/agents/app.md`
  - Done when:
    - `results.md` in this folder holds the setup, the incidents and one section per `todo.md` item with one entry per box, each sub-point marked PASS, FAIL / FINDING, judged (with the judgment), or not run with the reason, and its evidence (capture file names, MCP answers, measurements).
    - Every in-scope Windows box is deleted (confirmed), narrowed to what is left with the reason, or left with a recorded reason, under the rule above, and this plan's lists name the exact boxes deleted, narrowed and added.
    - The measured preview canvas size, status row height and header height replace the "inferred / not re-measured" notes in `docs/agents/app.md` (list G), or the header box stays open with the reason.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md` and this plan's files; no `todo.md` item outside lists A to F and H changed, and no macOS / Linux box changed.
    - No two `###` items are left without a blank line between them, and no doubled blank lines are left where a box or an item went (same counts as `origin/main`).
    - The PR description records what was seen for each removed box, since `todo.md` no longer holds them.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **The run.**
      - Build: Windows 11, debug build of `crates/app` with `--features screenshot` (`mise run app:dev:screenshot` builds it), at current `origin/main`; record the commit in `results.md`.
      - MCP: on a test port (`--mcp-port`, e.g. 41951), `"mcpEnabled": true` seeded through `--import-settings` into a fresh `--settings` store. Leave `mcpAllowDestructive` off, so `trash_photos` opens the app's own dialog.
      - Every run gets a fresh `--index`, `--settings` and `--log`, kept under the scratchpad or the batch folder, never the app's default store. Read the `page` timing lines from redirected stdout (they do not go to `Handpic.log`).
      - Quit each instance by its own PID (or `q` in its window); never kill by image name.
      - Input: SendInput synthetic keys and mouse. A held key is repeated key-downs with one key-up. Captures: window captures, the MCP `screenshot`, and burst captures of a screen region (one every 80 to 200 ms) for anything that moves.
      - Material only under `D:\Photos\tests\20261122-batch7\`; open no other folder under `D:\Photos\` in the app. Suggested sets (adjust and record what was used):
        - `nef\`: copies of `D:\Photos\samples\NEF\NIKON_D70_Nikon.nef` and two normal NEFs.
        - `cold-a\`, `cold-b\`: about 600 ARWs each, made with `robocopy /J` and opened once each (a hardlink or `cp` copy of a file already read is cached; see `docs/agents/app.md`, "During pass 1 the preview reads what the scan already read"). The same ARW copied under several prefixes is fine. `cold-b\` is the spare.
        - `many\`: a few hundred hardlinked ARWs (strip overflow, the Move to Trash runs; a trashed hardlink leaves the original).
        - `few\`: three ARWs (content that fits).
        - `mixed\`: about 20 copies from several makers under `D:\Photos\samples\` (a long `Filter ▾` menu with EXIF sections).
        - `jpeg-only\`: a few JPEGs (view-only).
        - `deep\`: enough nested empty folders to overflow the tree, some holding files so count badges show.
        - `drop\`: two small files to drag.
        - `import-dst\` as the Import root, if `Import…` is the overflowing dialog.
      - Measure at display scale 100 % (check the window's DPI, or compare the MCP `screenshot`'s reported size with the client size) in the default 1600 x 1000 window (no `--size`, no stored `windowBounds`). Use `--size` for the short-window checks.
      - If an incident touches anything outside the batch folder, stop and record it under Incidents.
    - **Editing.**
      - Rebase on `origin/main` right before editing: other sessions are changing `todo.md` and `docs/agents/app.md`.
      - Locate each item by its exact `###` line, never by line number.
      - Write a Python script to the scratchpad with the Write tool (not a Bash heredoc) and run it with `python -I`, asserting one match per `###` anchor; drop or replace boxes by their index among the item's top-level `- [ ] ` lines.
      - Every "seen" sentence added to a Background says "on Windows 11 on 2026-11-22" and names the results as `docs/plans/_archived/20261122-windows-device-checks-7/results.md` (in backticks), and ends with what is left open.
      - Do not touch `### App: real-device check of the Sort / Filter menus with no folder, the status row's reserved slot and shortened strip names (todo-three-decisions)` or the three Inferred rules it names in `docs/agents/app.md`, nor the folder-rename, folder-dialog-list and display-text items other sessions hold. What the run happens to see of them goes to `results.md` only.
      - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of the empty state, the status row, the scrollbars and the strip header`.
    - **A. `### App: real-device checks of the empty state in the preview area`** (2 Windows boxes):
      - Box 1 (rebinding `open`): launch with no folder and no `lastFolder`. In the Keyboard Shortcuts page rebind `open` to `space`, close the settings and read the message's key clause (`Space`); unbind it and expect the clause gone. The settings layer is opaque, so "at once" is judged on the first capture after it closes. Click the settings layer over where the message is and expect no folder picker (no new top-level window of the process). Deleted when all three passed.
      - Box 2 (second visit): on `nef\`, page from a normal NEF onto `NIKON_D70_Nikon.nef`, on, and back. Expect `The preview of this file could not be shown.` on the second visit too. Deleted when seen.
      - The item is deleted whole if both boxes go and no macOS / Linux box is in it (it has none today).
    - **B. `### App: real-device checks of the status row's StatusBar and progress circle`** (3 Windows boxes; the macOS / Linux box stays):
      - Box 1 (pass 1's circle): open `cold-a\` on a fresh index. Burst-capture the status row's right end from the first frame through the pass 1 to pass 2 change. Sub-points: the fill through pass 1; the restart at empty for pass 2; the judgment of the full-to-empty drop (180 ms); paging during the scan (repeated `→` key-downs, the stdout `page` lines compared with the same paging after the scan). The box also asks for a note in `docs/agents/app.md` (list G).
      - Box 2 (Move to Trash, layouts): on `many\`, select all, hide the strip and the left pane, start `Move to Trash…` (MCP `trash_photos` on the targets opens the app's dialog; confirm it in the window). Expect the circle to show the run and go. For "every pane layout", record the canvas size with the circle shown and gone in each layout reached (a trash redo and pass 2 also show the circle; the trash undo does not). Narrow to the layouts not recorded.
      - Box 3 (long text, error color): a long status text (e.g. the `error: <file>: libjpeg fatal error…` line on `NIKON_D70_Nikon.nef` in a narrow window, `--size`). Expect the circle's slot and the position to keep their place, and judge the error text's readability in the muted color.
    - **C. `### App: real-device check of the status row with no current-file info`** (1 box):
      - The scan suffix (`scanning N/total`) at the row's left edge without leading spaces: read from B box 1's captures on `cold-a\`.
      - `DROP_TEXT` (`Drop a single folder or RAW file.`) at the left edge: needs two files dropped on the window by a real OS drag (an Explorer window on `drop\`, driven by SendInput). If the drag cannot be made, the box is narrowed to the `DROP_TEXT` sub-point with that reason.
      - The item is deleted whole only when both passed.
    - **D. `### App: real-device check of the strip position in the status bar`** (1 box):
      - With the long status text of B box 3: the position not clipped by the right region. With a file shown: `Sort ▾` and `Filter ▾` popovers anchored to their buttons (right edges equal, measured). The item is deleted whole when both passed.
    - **E. `### App: real-device checks of the always-visible scrollbars`** (8 Windows boxes; the macOS / Linux box stays):
      - Box 1 (four bars at rest): `many\` (strip), `deep\` expanded (tree), a short window (meta pane), an overflowing dialog body (`Import…` or `Move to Trash…` on many files; open and Escape, nothing run).
      - Box 2 (thumb drag, track click on each, without scrolling first; the strip's thumbnails loading).
      - Box 3 (`few\`, collapsed tree: no bar, presses reach cells and rows).
      - Box 4 (tree: nothing under the thumb at rest, hovered, dragged; the measured gap between a badge's right edge and the thumb; a press on a badge's right edge selects the row; a non-overflowing tree).
      - Box 5 (meta pane: no value under the bar, notes and errors 12 px from the right edge, measured; a long dialog: no body text under its bar).
      - Box 6 (strip: the bar below the names, nothing under the thumb in the three states, a burst's band included; a press on a name band selects, a right press opens the menu).
      - Box 7 (short folder: the empty 9 px band, no jump in the preview's height between folders, measured; the top-edge drag to `MIN`, `MAX` and the preview's minimum; a 1:1 crop re-cut after a drag; a short window's menus fitting above the header). The drag sub-points overlap F box 3: run once, cite in both.
      - Box 8 (Neutral dark, Neutral light, one vendored theme such as Catppuccin Mocha, switched on the Appearance page: judged from captures; the OS appearance change in System mode: see Trade-offs).
      - The dialog bodies' list layout belongs to another session's item: judge only the bar here, and put anything else seen there in `results.md` without a new item.
    - **F. `### App: real-device checks of the strip's header row with Sort and Filter (native-strip-header)`** (7 Windows boxes; the macOS / Linux box stays):
      - Box 1: the header and its buttons; open upward, second press, press outside, `escape`; the lit filter button and its hovered border; the dimmed, inert sort button on `jpeg-only\`.
      - Box 2: `mixed\`, the strip dragged to its tallest, a short window (`--size`): the menu inside the window, scrolling, its button still toggling it, not clipped by the header.
      - Box 3: the handle: the resize cursor (not in a capture; read another way or left open), the 1 px line (a zoomed crop), the drag to `MIN` (96) and `MAX` (302), the preview stopping at `PREVIEW_MIN` (240), a press on a button's top 5 px band.
      - Box 4: at a stored `stripHeight` of 128 the image box is 144 x 96 (measured on a capture).
      - Box 5: a press, a right press and a wheel on the header: no cell selected, no context menu, no scroll.
      - Box 6: `toggleStrip` hides the header with the strip and closes an open menu; the status row keeps its height with the circle shown and gone; `n / shown (all)` under a filter.
      - Box 7: the measurements of list G. Deleted when the three values are measured and the notes replaced.
    - **G. `docs/agents/app.md`**, each bullet edited only as far as the run measured or saw:
      - "The sort and filter menus are deferred popovers over the window": the "Measured before the buttons moved to the strip header … Inferred, not re-measured" bullet gets the measured canvas (expected 1160 x 810), with the method and date.
      - "The folder tree is a `uniform_list` over rows kept in the view": the "Measured before the strip header … inferred 32 px shorter … 220 px wider with the tree hidden … not measured" bullet gets the measured values (the tree-hidden width only if a layout capture gave it).
      - "One progress circle sits in the status row's right region": "(measured before the strip header: 1160 x 842 either way)" gets the measured size with the circle shown and gone; the "Not yet observed: whether the drop … reads well, and whether the extra frames slow paging" sentence becomes what was seen and judged (B box 1), or keeps what is still open. Leave "Inferred: the constant position is not yet seen on screen" alone (the todo-three-decisions item's).
      - "The status row is gpui-component's `StatusBar` with its defaults": "Inferred from the sources, not re-measured on screen" gets the measured row height (expected 30 px); the popovers' "the hand check is pending" clause becomes a Seen note if D passed.
      - "The preview area says why it shows no photo": the "Not verified in the running app" bullet goes, or is narrowed, by A box 1.
      - "The strip, the tree and the meta pane draw the theme's scrollbar": the "Not verified by hand yet" bullet is narrowed to what E left open and gets the measured badge-to-thumb gap.
      - Measure the canvas, the status row and the header on a client-area capture at scale 100 % by scanning for the boundaries (the status row's top border, the header's handle line, the pane edges against the preview's surround), and cross-check against the expected 1160 x 810, 30 and 32. A value that disagrees is recorded as measured, with the capture named, and the box says so.
    - **H. New items**: one per bug the run finds that no existing item covers (search `todo.md` first; the verbatim-path, panic-text, black 1:1 canvas, folder dialog list, display text and todo-three-decisions items exist or are held by other sessions). Place each next to the item it was found under, with a Background ("Seen on Windows 11 on 2026-11-22 …", the results path, the files) and a `#### TODO` box.
    - **I. `learnings.md`**: this step's notes (methods that worked or failed, e.g. the OS drag, the cursor read, the measuring scan) and the deferred checks, each with its basis in `results.md`. `results.md` stays in the folder as the run's record.

## Trade-offs and risks

- **Measuring the canvas without a code change.** The earlier 1160 x 842 came from a temporary debug log of the canvas's paint bounds; nothing logs it now. Option 1 (planned, chosen): pixel-scan a capture at scale 100 %. No build differs from `main`, but it depends on the surround and the panes differing in color and on the scale being 100 %. Option 2: an uncommitted local debug line logging the paint bounds. Exact logical px, but the measured binary is not `main`'s and the patch must not reach the PR. The note in `docs/agents/app.md` names the method either way.
- **The OS appearance change** (scrollbar box 8, System mode). Option 1 (planned, chosen): do not change the user's Windows light / dark setting; the box is narrowed to that sub-point. Option 2: flip `AppsUseLightTheme` under HKCU and restore it. That closes the sub-point but re-themes every running app for the moment and leaves the user's setting wrong if the restore is missed.
- **`DROP_TEXT` needs a real OLE drag.** Synthetic mouse input on an Explorer window may or may not start one. If it does not, the status row item keeps its box, narrowed. The Explorer window shows only the batch's `drop\` folder.
- **Debug build.** Behavioural sub-points close on it. "Paging is not visibly slower" is judged on `page` timings during and after the scan on the same build; "reads well" and "acceptable look" are the session's judgment from captures. All three are recorded as judged, not measured against a rule (the user approved this).
- **Overlap with the todo-three-decisions check.** This run will see whether the position text stays still when the circle appears or goes. It goes to `results.md` only; that item, its boxes and its Inferred rules are another session's.
- **Overlap with the folder dialog list work.** The dialog-body scrollbar sub-points are judged while that layout may be changing on `main`; record the commit, and judge only the bar.
- **Concurrent edits.** Other sessions edit `todo.md` and `docs/agents/app.md`; rebase before editing and again before the PR, and re-run the script's one-match asserts.
- **Cold sets are single-use and large** (about 15 GB each). A wasted launch on `cold-a\` (e.g. a missed first frame) costs the set; `cold-b\` is the spare, and a third must be made with `robocopy /J` again. The session deletes its own cold copies after the run.
- **Synthetic input reaches whatever has the focus.** Confirm the foreground window is the test instance (by PID) before every key or click, since batch 5 had two mis-positioned tree clicks.

## Progress

- 2026-11-22: Step 1 done. The run is in `results.md`, the notes in `learnings.md`. Edits made:
  - A (empty state): both boxes passed (an action's last key cannot be removed in the UI, so the no-key clause is unreachable, recorded as such); the item is deleted.
  - B (StatusBar and circle): boxes 1 and 3 deleted. Box 2 narrowed to the canvas in the pane layouts not recorded (the right pane hidden, both sides hidden, the strip alone hidden); a Background sentence added. The macOS / Linux box kept.
  - C (status row with no current-file info): passed (`DROP_TEXT` and the scan suffix at x 16); the item is deleted.
  - D (strip position): passed; the item is deleted.
  - E (scrollbars): boxes 1, 3, 6 and 7 deleted. Narrowed:
    - box 2 to the meta pane's thumb drag and track click
    - box 4 to a comparison with a pre-gutter build
    - box 5 to the notes' right inset
    - box 8 to the OS appearance change in System mode

    A Background sentence added. The macOS / Linux box kept.
  - F (strip header): boxes 1 to 7 deleted; a Background sentence added. The macOS / Linux box kept.
  - G: the six `docs/agents/app.md` notes replaced with the measured or seen values (canvas 1158 x 767 of surround, 1160 x 768 with borders; status row 30 px; the circle's drain and paging; the popovers' anchoring; the empty state's rebinding; the scrollbars' bullet narrowed to what stays open).
  - H: one new item, "App: a shorter window leaves the preview under PREVIEW_MIN until the strip is dragged", after the strip header item.
