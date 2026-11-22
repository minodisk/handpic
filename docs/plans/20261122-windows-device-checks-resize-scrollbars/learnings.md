# Learnings: Windows real-device checks, batch 8 (drag-resizable panes, strip height and scrollbars)

## Step 1

- **#1232 (batch 7) merged before this plan's PR**, so the branch sits on `origin/main` `ba9efeaf` and `docs/agents/app.md`'s "Driving the window in a real-device run on Windows" section was on main to edit. The three `todo.md` items were byte-identical before and after it.
- **The cursor probe of batch 7's rule failed on the pane boundaries.** Moving the pointer away and back with `SetCursorPos` alone (450 ms wait) read `ARROW` all along both pane boundaries and `SIZENS` on only one row of the strip's band. Ending the move with a few relative `mouse_event` moves before the read gave the bands, which still shifted by about 1 px between runs. Recorded as a rule in `docs/agents/app.md`. Basis: `results.md`, Incidents.
- **The meta pane's band lies on the preview side of its line.** The first drag aimed at the meta pane's boundary pressed at x 1380, 1 px right of its line at 1379, and moved nothing; the band is x about 1374 to 1379 (the handle is `HandleEdge::Trailing` inside the preview). Pressing at x 1376 worked. Recorded in the `PaneWidths` section's Seen bullet. Basis: `results.md`, Incidents.
- **`--auto` takes a value** (`--auto N`, N automatic pages, then quit); a bare `--auto` exits with the usage error. The `--auto` check used `--auto 40`. `docs/agents/app.md` already says `--auto N`; the old `todo.md` boxes said plain `--auto`. Basis: `results.md`, Incidents.
- **Python on Windows**: the edits were made with a script run as `mise x -- python -I`, reading and writing bytes so no CR got in. An assert that no `\n\n\n` remains in `todo.md` failed at first because the file already holds two; compare the count before and after instead.
- **A heading named in prose matches a "not yet in the file" check**: the pane item's intro names the two new items' headings in backticks, so the check that they were absent before insertion has to match the heading as its own line (`\n### ...\n`).
- `results.md` stays in this folder as the run's record (copied verbatim from the scratchpad).

## Deferred issues (todo candidates)

- All open checks and the two findings of this run are already in `todo.md` through this step (the narrowed pane box 5, strip box 2 and scrollbars box 4, the macOS / Linux boxes, and the new items `### App: decide whether a pane drag keeps the press's offset from the boundary` and `### App: the meta pane breaks words mid-word at its 160 px minimum`). Nothing further to add. Basis: `results.md`; files `todo.md`, `docs/agents/app.md`.
