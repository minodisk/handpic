# Learnings

## Step 1

- `screengap.py` re-run against the measurement worktree's `out\20261121`
  gave output identical to the scratchpad's `screengap.txt` (line endings
  aside), so the copies in the archived record's `raw/` are the scratchpad
  files byte for byte and still reproduce.
- Writing the edit script inline through a Python heredoc string tripped
  over Windows paths: `out\20261121` in a non-raw string is an octal escape.
  Use raw strings for text holding backslashes.
- The median-difference estimate and the per-press gap agree within about
  1 ms except RAF Tauri (11.75 against 8.74 ms), whose wide
  `keypressToPixels` spread makes a difference of medians a poor stand-in
  for the median of differences.

## Step 2

- `gpui_kit::Window` has an inherent `window_handle()` returning GPUI's
  `AnyWindowHandle`, which shadows `raw_window_handle::HasWindowHandle`'s
  method of the same name; call it as `HasWindowHandle::window_handle(window)`.
  GPUI does not re-export `raw-window-handle`, so `crates/app` depends on it
  directly (0.6.2, already locked). The lock gained only the two edges.
- In-app A/B on this machine (release, `--auto 19`, 20-file compare folders,
  A = the same build with `paint` a no-op, A B A B, fresh index per run):

  | Folder / path | A `frameWait` p50 | B `frameWait` p50 | A `keypressToPixels` p50 | B `keypressToPixels` p50 |
  | --- | --- | --- | --- | --- |
  | ARW prefetch | 10.04 / 8.01 | 1.97 / 1.94 | 11.14 / 9.01 | 3.06 / 3.03 |
  | ARW `--no-prefetch` | 9.36 / 9.87 | 1.69 / 1.75 | 19.36 / 20.48 | 10.86 / 10.50 |
  | RAF prefetch | 9.12 / 7.57 | 1.95 / 2.10 | 10.61 / 8.84 | 3.45 / 3.62 |
  | RAF `--no-prefetch` | 8.00 / 11.74 | 1.56 / 1.54 | 46.08 / 50.44 | 40.01 / 41.28 |

  19 `page` lines per run (one per press), no panic in any `Handpic.log`.
- `frameWait` does not reach zero: about 1.4-2 ms remains. A's smallest
  `frameWait` per run was 1.6-4.4 ms, so the remainder is building the frame
  (render, layout, prepaint) before the preview canvas's paint closure stamps
  `paint_started`, not a wait.
- Held-key check: a PowerShell driver (`keybd_event` repeats every ~33 ms,
  45 Right, 45 Left, 45 Right, then `q`) on ARW, RAF and RAF
  `--no-prefetch`: 133-135 `page` lines, no panic, and a screen capture
  before `q` showed the last `page` line's file in the preview, meta pane and
  strip. The window needs an Alt tap before `SetForegroundWindow` to take
  the foreground from a background process; without it no key arrives.
- The `deferring re-entrant` lines cannot be checked through `RUST_LOG=debug`:
  the app's logger keeps only this crate's debug lines and drops GPUI's
  backend ones. One `page` line per press and no panic are the evidence that
  no nested draw happened on the page.

## Step 3

- The ruler A/B ran in one session (2026-11-22 03:42-04:31 local): A
  `c57ec2ff`, B `6809229f`, `--no-prefetch`, 1600x1000 at (100,100), ARW and
  RAF, A B A B A B. Every B run's `frameWait` p50 was 1.54-1.75 ms (A
  8.73-10.61). `screen` p50 / p95 moved -8.41 / -7.52 ms on ARW and
  -4.85 / -3.46 ms on RAF. See `results.md`.
- The gain is all or nothing per press, as the plan expected: no A press
  reached the first DWM composition after its page was pended (pended to
  screen always 17.4-37.6 ms), while 47 % (ARW) and 34 % (RAF) of B's did.
  The earliest any B page reached the screen was 7.95 ms (ARW) / 9.01 ms
  (RAF) after it was pended, so a page pended in the last 8-9 ms of a refresh
  still misses that vblank (the smallest toDwm of a first-composition press,
  5.34 / 6.40 ms, bounds the presented side). That is why the in-app
  `keypressToPixels` drop (about 8.6-9.3 ms) overstates the screen gain.
- `screengap.py`'s per-press pairing gets a weak aligned r on B ARW
  (0.12-0.35): with the wait gone, `keypressToPixels` spreads over only a
  few ms against `screen`'s 16.9 ms quantization, the same effect as the
  re-run's Tauri ARW. The shifted r stayed near zero and no gap was negative,
  so the pairing holds.
- Two background processes (Amazon Photos and another session's Python) each
  held about a core, so the 15 % load gate discarded 20 of 32 attempts and
  left `RAF-B1` and `RAF-A2` empty after 4 tries. `raw/redo-slots.ps1`
  (the driver with named slots from try 5 on) filled them afterwards.
- `powershell -File` does not parse `'a','b'` into a `[string[]]`; pass an
  array parameter through `-Command "& script.ps1 -Slots 'a','b'"`.
- The driver now holds `SetThreadExecutionState(ES_CONTINUOUS |
  ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)` itself (`[uint32]2147483651`;
  a hex string cast to `[uint32]` overflows as an `int32` first).
- Building B right after A into the same `CARGO_TARGET_DIR` took 37 s
  instead of 3.5 min; only the workspace crates rebuilt.
- `docs/humans/performance.md` / `.ja.md` quote no number this step changed
  (their `frameWait` description already says about 2 ms since Step 2), so
  they are unchanged.

## Deferred issues (todo candidates)

- `docs/agents/app.md`, "`keypressToPixels` does not include the texture
  upload": its rule calls "the about 15 ms between `present` and the DWM
  present" the floor for any image size. With keyed pages painted at once on
  Windows that stretch (toDwm) now depends on the phase at which the frame is
  presented: p50 15.03 / 17.42 ms but p95 23.36 / 25.97 ms on ARW / RAF, and
  no longer a floor. Basis: Step 3's per-press tables
  (`docs/plans/20261122-page-present-now/results.md`). Out of Step 3's scope
  (which covers the `frameWait` rules only).
- Wrap-up: `docs/agents/app.md` links `../plans/20261122-page-present-now/results.md`;
  the link must follow the plan folder when it moves to `_archived`.
- The ruler measured only the read path (`--no-prefetch`). The on-screen gain
  of the prefetch hit (`show_held`), the everyday path, is unmeasured; its
  in-app `frameWait` is 1.7-1.9 ms at p50 for B. A second ruler pass with
  prefetch on would settle it (the plan's trade-offs name it). Basis:
  `docs/plans/20261122-page-present-now/results.md`, "Caveats".
