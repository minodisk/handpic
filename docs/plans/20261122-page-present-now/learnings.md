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
