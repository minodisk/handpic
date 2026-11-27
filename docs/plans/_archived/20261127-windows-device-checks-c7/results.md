# Results: Windows real-device checks run through MCP (2026-11-27)

Host: Windows 11 Home 10.0.26200, one 5120x1440 display at 96 DPI, the
taskbar docked left (visible area x 62 to 5120, 5058 px wide). Other sessions
were building and running instances at the same time (a busy machine).

App: a debug build of `crates/app` at `f5cab4bf` (2.2.0) with the
`screenshot` feature (`cargo build --features screenshot`), run with its own
`--index`, `--settings`, `--log` and `--mcp-port 47817`, seeded with
`{"mcpEnabled": true, "mcpAllowDestructive": true}` through
`--import-settings`. Driven through the MCP companion's tools only (no key
presses, clicks or drags). The strip's height was set by writing
`stripHeight` into the test store between launches.

Photos: copies under `D:\Photos\tests\20261127-c7-checks\`: `longnames` (14
ARW from `20261123-mcp-visual`), `eyes` (24 ARW of a basketball session from
`2026-09-19`, the closed-eyes labeling set), `sidecars` (300 hard links of one
ARW, each with an `.xmp`) and `burst` (120 hard links of `_DSC1926.ARW`, one
120-frame burst).

## Default window centering (todo-ten-closeouts)

A fresh `--settings` store (no `windowBounds`), taskbar docked left:
`GetWindowRect` (1783, 216)-(3399, 1224), 1616x1008 outer, client 1600x1000;
the DWM frame (1790, 216)-(3392, 1217), center x 2591, center y 716.5. The
visible area's center is x 2591 (62 + 5058 / 2), y 720: centered to the
pixel horizontally and within the frame's 3.5 px vertically, 31 px right of
the display's center (2560), no longer 93 px. The taskbar at the bottom was
not tried (the host's taskbar setting was left alone).

## Strip names at the minimum and maximum heights (todo-three-decisions)

At `stripheight::MIN` (96) every long name was shortened in the middle
(`DSC-HX95_....ARW`, `ILCE-6300_...8.arw`, `DSC-RX100....ARW`) and stayed
inside its cell with a gap to the next; short names (`_DSC1616.ARW`) were
centered and unchanged. At `MAX` (302) no name needed shortening; the longest,
`ILCE-6300_Free_raws_by_signatureeditsco_DSC03068.arw`, fitted whole inside
the selected cell's outline. No glyph was clipped at either edge in 2x crops;
`NAME_INSET` left as is. (Heights set through the store, not by a drag.)

## Strip's closed-eyes mark (eyes-mark-tier-mcp)

- On `eyes` after pass 2: one cell, `_DSC1926.ARW`, showed the amber
  `eye-off`; `Filter ▾` > `Eyes` > `Closed` (`set_filter`) kept exactly that
  file, and `get_view` gave its eyes `state: closed`.
- On a fresh `--index` the face icons and the mark appeared on the cells
  pass 2 had reached while the status row showed `analyzing 14/24`, the rest
  following when it ended, with no re-listing in the log.
- At `MIN` the mark sat in its slot at the image box's bottom-left with the
  burst badges (`1/2`, `2/2`) clear of it; on the 120-frame `burst` folder the
  three-digit `117/120` to `120/120` badges stayed clear of it. Rejecting
  `_DSC1926` (`set_judgment`) dimmed the cell and the mark with it.

## Group threshold (all-faces-check Step 4)

`get_settings` gave `group_min_faces` 5 on a fresh store. `set_settings` with
3 started `checking faces 0/4` at once, and the four rows pass 3 then filled
were exactly the four whose `faces_group` was 3 or 4 (`_DSC2906` 4,
`_DSC2401`, `_DSC2914`, `_DSC3051` 3). 25, 1 and a string were refused
(`group_min_faces must be a whole number from 2 to 20`) with the value in
force kept. The value 3 survived a restart. Not driven: typing in the
`Faces in a group photo` field, Enter / Tab / Escape, the `-` / `+` steps,
the refusal in the status line and digits not being shortcuts.

## All-faces meta rows and the filter items (all-faces-check Steps 5 and 6)

On `_DSC2914` (`Similar faces` 3): at threshold 3 the `Analysis` group ended
with `Faces judged 1`, `Faces with closed eyes 0`, `Faces with soft eyes 1`;
at 4 only `Similar faces 3` showed. `Anyone soft` kept 2 frames at 3
(`_DSC2914`, `_DSC3051`, each soft count 1) and 0 at 4; `Anyone soft` with
`Open` kept 19 at 3 (17 at 4); `Anyone closed` kept none (no closed count in
this folder). The threshold 3 and the `anyone_soft` check survived a restart.
Not judged: whether the counts agree with what the photos show (a real group
folder and the user's eye), and a closed count's mark.

## Chunked Delete Sidecars… (chunked-sidecar-delete)

300 `.xmp` beside 300 ARW: `delete_sidecars` previewed
`300 XMP sidecars (86.1 KB)` and, confirmed, moved them in about 1.6 s
(about 5-7 ms a file; polled `0/0`, `100/300`, then finished), against about
140 ms a file before. The status row's progress circle showed empty, then a
partial arc. `undo` restored all 300 in 2759 ms and `redo` moved them again
in 2075 ms (`trash: undo of run 1 in 2759ms`, `redo of run 1 in 2075ms`).
Through the MCP `undo` / `redo` tools, the path the keymap's keys run.

## Windows GUI-subsystem build (windows-gui-subsystem)

The installed release `Handpic.exe` (2.1.0) launched through Explorer
(`explorer.exe` on a `.lnk` carrying test `--settings`, `--index` and `--log`,
Explorer's ShellExecute as a double-click does): parent `explorer.exe`, no
child process, one visible window (`Zed::Window`), no console window.
PowerShell 7 is not installed on this host.

## Riffle beside Handpic (native-release)

On this host Riffle 1.10.0 (`%LOCALAPPDATA%\Riffle`) and Handpic 2.1.0
(`%LOCALAPPDATA%\Programs\Handpic`) are installed side by side with two
uninstall entries (`Riffle`, `dev.minodisk.handpic_is1`). Only Handpic has a
Start menu shortcut, and there is no Riffle `settings.json`, so the seeding
from it cannot be seen here; nothing was uninstalled.

## install.ps1 failure path (install-scripts)

Windows PowerShell 5.1.26100: a local copy of `install.ps1` with the expected
digest replaced by 64 zeros, piped to `iex`, printed `Downloading
SHA256SUMS`, `Downloading Handpic-2.1.0-windows-x64-setup.exe`, then threw
`Checksum mismatch for Handpic-2.1.0-windows-x64-setup.exe; nothing
installed.`; the session went on, `$ErrorActionPreference` and
`$ProgressPreference` kept their values, `Handpic.exe` was untouched and no
`handpic-install-*` folder was left. The real install was not run (it would
replace the user's own install).

## Prefetch ring after the wrap (todo-ten-closeouts)

Release build of `crates/app` at `f5cab4bf`, copies of the 20-file compare
folders (`20261108-gpui-compare/ARW`, `/RAF`), one warm-up `--auto 60` per
folder, then three runs with the ring alternated with three under
`--no-prefetch`, each `--auto 60` (stdout through `| cat`). Counted from the
`page` lines' `source=prefetch`:

| Folder | Ring: hits | Ring p50 / p95 (median) | `--no-prefetch` p50 / p95 (median) |
| ------ | ---------- | ----------------------- | ---------------------------------- |
| ARW    | 60, 60, 60 of 60 | 13.64 / 22.22 ms | 28.01 / 36.30 ms |
| RAF    | 60, 58 of 59, 60 of 60 | 13.34 / 22.04 ms | 55.39 / 71.95 ms |

The wraps now hit: every run's presses across the first / last file came from
the ring. The one miss (RAF run 2, `X-E5_DSCF1008.RAF` mid-list, `wait`
148.70 ms, `keypressToPixels` 243.16 ms, one press coalesced so 59 pages) was
a read on a busy machine, not a wrap. The ring's p95 fell from the wraps'
reads (ARW 20.10, RAF 56.27 ms before) to about 22 ms on both.
