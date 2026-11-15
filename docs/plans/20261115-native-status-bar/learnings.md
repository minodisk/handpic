# Learnings

## Step 1

- The preview canvas was measured with a temporary `log::info!` of the
  canvas's paint bounds in a debug build (`--auto 1` on a scratch folder with
  isolated `--index`, `--settings` and `--log`): 1160 x 842 logical px at
  scale 1 in the default 1600 x 1000 window with the tree shown, 10 px less
  height than the recorded 852 (the plan expected about 9). Forcing the
  circle to always show gave the same size, so the circle does not change
  the row's height.
- A screenshot (PowerShell `CopyFromScreen`) showed the row filled with a
  thin top border, the circle left of `Sort ▾` and the buttons unchanged. A
  status text far longer than the row stays on one line, clipped by the
  center region's `overflow_hidden`, with the circle and the buttons in
  place, so no wrapper is kept around the status text.
- `relative` (the function) became unused in `main.rs` after the bar went;
  `MUTED_FILL` is still used by the strip's cells.
- A Python heredoc holding a long Markdown section with backticks failed in
  Git Bash ("unexpected EOF while looking for matching `''"); writing the
  script to a file in the scratchpad worked.
- `mise run fmt` failed in this fresh worktree because `node_modules` was
  missing (`vp` not found); `pnpm` is not on the Git Bash `PATH`, so
  `mise exec -- pnpm install --frozen-lockfile` installed it and the task
  then passed.

## Deferred issues (todo candidates)

- Pending manual check (native app, Windows / macOS / Linux): the Step 1
  "Manual check" in `docs/plans/20261115-native-status-bar/plan.md`. Steps:
  launch the native app, open a folder; confirm the row has a top border, its
  fill and 12 px muted text; open and dismiss `Sort ▾` and `Filter ▾`
  (including a tall `Filter ▾` menu, which must not be clipped by the
  regions' `overflow_hidden` and must fit above the row with the 40 px cap),
  right edge on the button's, dismissed by a press outside; during a scan,
  watch the circle fill through pass 1, restart for pass 2 (the full-to-empty
  drop animates over 180 ms; judge whether it reads well) and disappear at
  the end, with the status text and the buttons not moving, and confirm
  paging during a scan is not visibly slower from the circle's animation
  frames; run a Move to Trash on several files and watch the circle; confirm
  an error text is readable in the muted color. Expected: all as described
  in `docs/agents/native-app.md`'s two status row sections. The step's
  checkbox was ticked on the automated criteria, the measured canvas size
  and the screenshots of the row, the forced-shown circle and a long text;
  the menus, the live scan and the trash run were not exercised by hand.
