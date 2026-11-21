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

# Page Present Now

## Purpose

On Windows, a keyed page turn in the GPUI app reaches the screen about one
display refresh later than it has to. The per-press analysis of the
2026-11-22 Tauri vs GPUI re-run
([results](../_archived/20261122-tauri-gpui-latency-ab-rerun/results.md))
puts the time from the image painted to the DWM present at 25.85 / 31.70 ms
(ARW) and 23.50 / 31.99 ms (RAF) at p50 / p95, of which `frameWait` is
10.23 / 16.87 and 7.55 / 16.74 ms and the stretch from the in-app `present`
stamp to the DWM present 14.59 / 15.18 and 14.75 / 25.14 ms.

The cause is in GPUI's Windows platform (gpui-pre-windows 0.3.12, not
vendored, not to be changed):

- A vsync thread loops on `DwmFlush()` and invalidates every window
  (`RedrawWindow(.., RDW_INVALIDATE)`); only the resulting `WM_PAINT` draws.
  So `cx.notify()` waits for the next vsync tick. That wait is `frameWait`.
- The frame is then presented at once (`Present(0, 0)`), just after the
  vblank, so DWM composes it at the following vblank, about one refresh
  later.

This plan makes the app ask Windows to process the paint now
(`RedrawWindow(hwnd, None, None, RDW_INVALIDATE | RDW_UPDATENOW)`) once a
keyed page's result is pended, so the frame is presented before the next DWM
composition instead of after it. Expected gain: about one refresh (about
15 ms) per keyed page turn. The plan also records the per-press analysis the
fix rests on, and measures the fix with the external ruler.

## Steps

- [ ] Step 1: Record the per-press painted-to-screen analysis in the archived re-run record (docs only)
  - Done when:
    - `docs/plans/_archived/20261122-tauri-gpui-latency-ab-rerun/raw/` holds `screengap.py` and `screengap.txt`, copied unchanged from the session scratchpad (`C:/Users/daisu/AppData/Local/Temp/claude/203cdbc2a3441aed/aadfa88c-bb15-4953-9275-fa7805d95db8/scratchpad/screengap.py` and `screengap.txt`; if the scratchpad is gone, `screengap.txt` is regenerated with `python raw\screengap.py out\20261121` in the measurement worktree and must equal the numbers below).
    - `results.md` of that record has a section `## Painted to screen, per press`, after `## In-app endpoints, aligned` (after its `### Reading, aligned`) and before `## Compared with the 20261120 A/B`, containing:
      - The definition: press i of each used run's latprobe CSV (`settled_ms`) is paired with the i-th `page` line carrying `keypressToPixels` of that run's log. painted = GPUI `keypressToPixels - frameWait - present`, Tauri `keypressToPixels`. gap = `screen` - painted. GPUI's parts of the gap: `frameWait`, `present`, and toDwm (`screen - keypressToPixels`). Tauri's: afterFrame2 (`screen - frame2`).
      - The pairing check: 60 / 60 presses per run; Pearson r of `screen` against `keypressToPixels` aligned 0.91-1.00, except Tauri ARW 0.29-0.53; shifted by one press -0.28 to 0.25; no negative gap; smallest gap 5.14 ms.
      - Per-run tables (p50 / p95, ms) for ARW GPUI, ARW Tauri, RAF GPUI and RAF Tauri with a median-of-3 row each, every cell equal to `screengap.txt` (median rows: ARW GPUI gap 25.85 / 31.70, frameWait 10.23 / 16.87, present 0.47 / 0.93, toDwm 14.59 / 15.18; ARW Tauri gap 28.31 / 42.29, afterFrame2 1.69 / 18.29; RAF GPUI gap 23.50 / 31.99, frameWait 7.55 / 16.74, present 0.39 / 0.54, toDwm 14.75 / 25.14; RAF Tauri gap 8.74 / 20.73, afterFrame2 7.80 / 19.93).
      - A reading: the median-difference estimates (25.55 / 24.29 / 27.35 / 11.75 ms for ARW GPUI / RAF GPUI / ARW Tauri / RAF Tauri at p50) against the per-press 25.85 / 23.50 / 28.31 / 8.74, and why RAF GPUI's gap is longer than RAF Tauri's.
      - One sentence on the cause of GPUI's gap: the draw waits for the vsync thread's tick (`frameWait`), and the frame presented right after that vblank is composed by DWM at the next one (toDwm). State it in prose; do not link to this plan (its folder moves when archived).
      - Caveats: input delivery is inside the gap; percentiles do not add up across the parts; Tauri ARW's low r comes from `screen`'s 16.9 ms quantization against a narrow `keypressToPixels` spread.
    - The header's raw-output list names `raw/screengap.py` and `raw/screengap.txt`; the header's bullet list has a pointer saying that the header table's last column is, for GPUI, the toDwm part and not the painted-to-screen gap, linking to the new section; `## Reproduction` gains the line `python raw\screengap.py out\20261121`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Only files under `docs/plans/_archived/20261122-tauri-gpui-latency-ab-rerun/` change. No number in the existing sections changes.
    - Follow the existing sections' conventions: cells `p50 / p95` in ms, run rows 1-3 then a bold median-of-3 row, links relative to the record's folder.
    - `screengap.py` hard-codes the 12 used runs (`USED`); it reads the per-press CSVs and logs that stay in the measurement worktree's `out\20261121`, which the record's caveat on reproducibility already covers. Say so next to the reproduction line.

- [ ] Step 2: Process the paint at once after a keyed page is pended (Windows only)
  - Done when:
    - On Windows, after a keyed page's result is pended and notified, the window's `WM_PAINT` is processed at once through `RedrawWindow(hwnd, None, None, RDW_INVALIDATE | RDW_UPDATENOW)`, on both keyed page paths: the read result (`send` / `loaded`) and the prefetch hit (`show_held`).
    - A release build run from a terminal (`mise run app:release -- <folder>`) prints `page` lines whose `frameWait` is near zero on keyed page turns, with `source=read` (`--no-prefetch`) and with `source=prefetch` alike, and holding an arrow key through a folder neither panics nor leaves a stale image on screen. `Handpic.log` has no panic line (see `docs/agents/app-log.md`).
    - Nothing changes on other platforms: the call and its dependencies are behind `cfg(windows)`, and the non-Windows code path is the one that exists today.
    - The UI tests (`src/uitest.rs`, GPUI's `TestAppContext`) pass unchanged: a window with no Win32 handle makes the call a no-op.
    - No change to GPUI, gpui-kit or gpui-component, and no new version of any crate in `crates/app/Cargo.lock` (only new edges to versions already locked).
    - `CLAUDE.md`'s Layout names the new module if one is added; `docs/agents/app.md` and `docs/humans/performance.md` / `performance.ja.md` are updated wherever their description of `frameWait` ("the wait for the next frame") stops being true for a keyed page on Windows.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (they cover the macOS and Linux builds).
  - Implementation approach (as far as it is known; omit if unknown):
    - **Do not make the call from `cx.defer` / `window.defer`, nor anywhere inside an update.** Both run in the effect flush of the current update, while the `App` is still mutably borrowed. `RDW_UPDATENOW` sends `WM_PAINT` synchronously, into gpui-pre-windows' `draw_window` and from there GPUI's `on_request_frame` callback, which only guards against `draw_in_progress()` before it touches the `App` (gpui-pre 0.3.12 `src/window.rs`, about lines 1904-1930; its comment says touching the App there would panic on the already-mutable borrow). The call has to run on the main thread with no `App` borrow and no draw on the stack:
      - Read path: `send` in `crates/app/src/main.rs` already runs in a `cx.spawn` task; make the call in that task's body after `this.update(..)` has returned, when `loaded` pended a keyed page (have the update return whether it did).
      - Prefetch path: `show_held` runs synchronously inside the key handler; spawn a foreground task (`cx.spawn`) whose body makes the call, so it runs after the key dispatch has unwound.
    - Scope: only when `pend_page` pended a page with `keyed` true. Not the unkeyed pages (strip click, folder open), not `stale` intermediate frames, not the 1:1 zoom crop (`zoom_pending`).
    - The HWND: `gpui_kit::Window` implements `raw_window_handle::HasWindowHandle` (`RawWindowHandle::Win32`). Read it once where the view is built with a `&Window` at hand and keep it in the view as an `Option` (as an integer; `HWND` is not `Send`, though the task is a foreground one). `None` when the handle is not Win32 or `window_handle()` fails, as under `TestAppContext`.
    - Dependencies, both `[target.'cfg(windows)'.dependencies]` of `crates/app/Cargo.toml`: `windows = "0.62"` with the `Win32_Foundation` and `Win32_Graphics_Gdi` features (the lock already holds 0.62.2, which gpui-pre-windows and `crates/engine` use), and `raw-window-handle = "0.6"` (0.6.2 locked) unless GPUI turns out to re-export the crate (verify; no mention was found in gpui-kit 0.9.1's `lib.rs` or `Cargo.toml`). Builds use `--locked`, so commit the `Cargo.lock` edges.
    - Keep the Win32 call in one small `cfg(windows)` function (a module of its own, e.g. `src/presentnow.rs`, or beside the view), with a no-op for other platforms so the call sites carry no `cfg`. Log a failed `RedrawWindow` at `debug`, do not fail the page.
    - `timing.rs` is unchanged: `frameWait` stays `paint_started - notified` and `keypressToPixels` keeps its meaning. `notified` is stamped in `pend_page`, before the call.
    - Verify during implementation:
      - No panic and no `deferring re-entrant` debug line (`RUST_LOG=debug`) from the nested `WM_PAINT` inside the foreground task runner (gpui-pre-windows `platform.rs` `run_foreground_task`, which itself dispatches paint messages between tasks).
      - The next vsync tick's `WM_PAINT` draws nothing new (the window is no longer dirty) and the `page` line is printed once per press.
      - The paint hold (`hold.rs`, `PaintHold`) and the retired images still release on the frame that now comes earlier.
      - `--auto N` still completes and prints its summary.

- [ ] Step 3: Measure keypress to the screen before and after, and record the result
  - Done when:
    - A same-session A/B of two GPUI release builds, A = `main` before Step 2 and B = `main` after it, has been run with the external ruler (`tools/latprobe` of the local-only branch `measure/tauri-latency-ab`, in the worktree `C:\Users\daisu\.herdr\worktrees\handpic\tauri-latency-ab`), alternating A B A B A B, three used runs per app per folder, window 1600x1000 at (100,100), prefetch off, on the ARW and RAF folders under `D:\Photos\tests\20261108-gpui-compare`. Manual verification: it needs the interactive desktop and an idle machine.
    - `results.md` in this plan's folder records, in the form of [the fit-preview-downscale record](../_archived/20261121-fit-preview-downscale/results.md): the verdict, the conditions and attempts, per-run and median-of-3 tables of `screen` and the `keypressToPixels` stages (`frameWait`, `paint`, `present` among them), the per-press painted-to-screen breakdown (gap, `frameWait`, `present`, toDwm) for A and B, the pairing check, caveats and reproduction. `raw/` beside it holds the drivers, `runs.txt`, the summaries and the per-press script with its output.
    - The record states whether B's `frameWait` dropped to about zero and by how much `screen` moved at p50 and p95 on each folder, and what share of presses gained a whole refresh.
    - The record also gives the in-app `frameWait` of keyed `source=prefetch` pages for B (a short run with prefetch on; the ruler itself runs with prefetch off).
    - `docs/agents/app.md`'s rules that quote `frameWait` ranges for reading an A/B (around its "check every run's `frameWait` p50" rule) are brought in line with the measured behavior, and `docs/humans/performance.md` / `performance.ja.md` are updated together if a number they quote changed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged; build A at the commit before its merge and B at the merge (or later `main`), both `cargo build --release --locked` in `crates/app`.
    - Start from the GPUI-vs-GPUI drivers already in the repository, `docs/plans/_archived/20261121-fit-preview-downscale/raw/run-ab-gpui.ps1` and `summarize-gpui.py` (parameters `-Gpui`, `-GpuiB`, `-Probe`, `-Out`), and from Step 1's `screengap.py` for the per-press breakdown: both apps are GPUI here, so both sides use the GPUI branch (`frameWait`, `present`, toDwm), and its hard-coded `USED` table has to name this session's used runs.
    - Follow the attempt rules of the earlier records (load samples, discarding a bad run and redoing its slot, marking superseded lines of `runs.txt`).
    - The per-press CSVs and app logs stay in the measurement worktree; say so in the record, as the earlier ones do.
    - Check every run's `frameWait` p50 before reading the A/B: A's should be in the usual 6-10 ms, B's near zero. A B run with a large `frameWait` means the immediate paint did not happen and is a finding, not noise.
    - If the measurement cannot be run by the agent, it becomes a manual check in `todo.md`, as in the fit-preview-downscale plan.

## Trade-offs and risks

- **Where the call runs.** The brief suggested `cx.defer` / `window.defer`. Reading GPUI's sources says that would re-enter `on_request_frame` with the `App` borrowed, so Step 2 calls from a foreground task body instead. On the prefetch path that costs one message hop, expected well under a millisecond but unmeasured. If even a task body turns out to be unsafe (a platform-side borrow held while a task runs), the fallback is to leave that path on the vsync tick and record why.
- **Scope: keyed page turns only.** Strip clicks, folder opens, stale intermediate frames while a key is held and the 1:1 zoom crop keep waiting for the vsync tick. Extending is one more call per path, but each needs its own check for re-entrancy, and only the keyed page has a ruler. Decision left to the caller: extend after Step 3's result, or not at all.
- **The gain is all or nothing per press.** A frame presented before DWM's composition deadline reaches the screen one refresh earlier; one presented too late lands on the same vblank as today. So `screen` should drop by about one refresh on most presses and by nothing on the rest, and percentiles may move unevenly. No pass / fail threshold is set here. If Step 3 shows no gain, the caller decides between reverting Step 2 and keeping it as harmless.
- **The ruler measures the read path only.** The A/B runs with prefetch off, while everyday paging goes through the prefetch hit (`show_held`). Step 3 checks that path's in-app `frameWait` only. A second ruler pass with prefetch on is the stricter option and roughly doubles the session.
- **Dependence on GPUI internals.** The fix relies on gpui-pre-windows drawing from `WM_PAINT` and presenting at once, and on an active window not being frame-paced. A GPUI upgrade can change any of these; a regression shows as `frameWait` back at 6-10 ms, which the `page` lines make visible. An inactive, low-power or thermally throttled window may still be paced by `FramePacer` and simply fall back to today's timing.
- **Extra presents.** The vsync tick after an immediate paint normally finds nothing dirty. If GPUI re-presents the same frame there (`needs_present`, high-rate input), that is one redundant present, not a visible change.
- **A direct `windows` dependency in `crates/app`.** `windows = "0.62"` matches the engine and the locked 0.62.2. `windows-sys` 0.61.2 is also locked and lighter, but would be a second binding style in the repository.
- **Step 1 edits an archived record.** That is intended (the analysis belongs to that measurement). Its cause sentence is prose without a link to this plan, because this plan's folder moves on archive; the wrap-up may add the link once the path is final.

## Progress
