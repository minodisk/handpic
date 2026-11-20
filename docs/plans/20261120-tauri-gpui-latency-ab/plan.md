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

# Tauri vs GPUI page-turn latency, same-session A/B to the screen

## Purpose

The removed Tauri app's `keypressToPixels` ended when the canvas `drawImage`
returned, so the vsync wait, WebView compositing and the present were in none
of its numbers
([the latency breakdown](../_archived/20261108-preview-latency-breakdown/results.md)
left that open). The GPUI app's `keypressToPixels` includes `frameWait` and
`present`, so the two in-app numbers never had the same endpoint.

On 2026-11-20 both apps were measured in one session, alternating, against one
external ruler: keypress (`SendInput`) to the DWM present of the frame that
shows the new image. This plan records that measurement as `results.md` in this
folder, so the comparison with an aligned endpoint is on record next to
[the gpui-port results](../20261109-gpui-port/results.md).

The measurement is already done. The plan changes no code: the measurement
tooling stays on the local-only branch `measure/tauri-latency-ab`.

## Steps

- [x] Step 1: Write `results.md` with the method, the tables and the reading of
      the A/B, and keep the raw summary beside it
  - Done when:
    - `docs/plans/20261120-tauri-gpui-latency-ab/results.md` exists, in
      English, and holds: the headline, the method (both builds, the external
      ruler, the driver and its idle gate, the percentile rule), the machine,
      the attempts and discards, one table per folder and app with every run
      and the median row for `screen`, `screenFirstChange` and each in-app
      stage, the interpretation, the caveats and open question, and the
      reproduction commands naming the branch and its commit.
    - `summary.md` and `runs.txt` from the raw data folder are copied unchanged
      into `docs/plans/20261120-tauri-gpui-latency-ab/raw/`, and `results.md`
      links to them, so the record survives the local worktree.
    - Every number in the tables equals the corresponding cell of
      `summary.md` in the raw data folder, digit for digit (GPUI cells to two
      decimals, Tauri in-app cells as `summary.md` prints them). Derived
      numbers (differences, sums) are recomputed from those cells and match
      to the shown precision.
    - The discard counts equal what `runs.txt` holds (see the approach for the
      counts found at planning time; recount, do not copy).
    - Every observation in the interpretation is checked against the data
      before it is written; one the data does not support is dropped or
      reworded, not kept.
    - The doc names no personal data (no path under the user's profile; name
      the branch and the relative `out\20261120` folder) and says of the
      migration only that the branch disables it because the changed
      identifier would otherwise move the real app's data.
    - `mise run ci:local` passes locally (for a `.md`-only change it selects
      lychee, the offline link check; the repository has no Markdown
      formatter) and the PR's GitHub checks pass.
    - The PR changes only files under
      `docs/plans/20261120-tauri-gpui-latency-ab/`.
  - Implementation approach (as far as it is known; omit if unknown):
    - Style: follow `../20261109-gpui-port/results.md` (verdict first, then
      "Measurement conditions", an "Idle" subsection with the attempts, per-run
      `p50 / p95 (ms)` tables with a bold median row, then the reading) and
      `../_archived/20261108-preview-latency-breakdown/results.md` (states
      what each number's endpoint is, and what was not measured). Cells are
      `p50 / p95` in ms.
    - Sources, all outside the repository, in the measurement worktree
      `C:\Users\daisu\.herdr\worktrees\handpic\tauri-latency-ab`:
      - `out\20261120\summary.md`: the `summarize.py` output and the source of
        every table. Copy numbers from this file, not from this plan.
      - `out\20261120\runs.txt`: every attempt, its load samples and why it
        was discarded.
      - `out\20261120\<run>.csv` (per press: `settled_ms`, `first_change_ms`,
        `frames`, `max_accumulated`), `<run>.probe.err.txt`, and the page
        lines (`<run>.stdout.txt` for GPUI, `<run>.app.log` for Tauri).
      - `tools\latprobe\` (`src\main.rs`, `run-ab.ps1`, `summarize.py`) and
        `git show 1e97309e` for the method and the reproduction commands.
    - Builds to state: A = GPUI, main `dc6fa3d7`, `crates/native` release
      build (Rust 1.98.1), run as
      `handpic.exe <folder> --no-prefetch --index <temp> --settings <temp> --log <file>`,
      page lines from stdout. B = Tauri, `a9b0e165` (the parent of `80221f61`,
      which removed the app) plus the measurement commit `1e97309e` on
      `measure/tauri-latency-ab`, built with `pnpm tauri build --no-bundle`.
      The measurement commit's changes: timing logs always on; prefetch
      disabled (to match `--no-prefetch`); paging wraps at the end (GPUI
      wraps, Tauri clamped); nested `requestAnimationFrame` marks `frame1` /
      `frame2` (ms from `startedAt`) on keyed page lines; `last_folder` honors
      `HANDPIC_MEASURE_FOLDER`; the data migration disabled; identifier
      `dev.minodisk.handpic.latencyab`.
    - Ruler to state (`tools/latprobe`, Rust, `windows` 0.58): QPC stamped
      just before `SendInput(VK_RIGHT)`; Desktop Duplication
      `LastPresentTime` (the QPC of the DWM present) per frame; an FNV hash of
      a 64x64 patch at the window's client center; `settled` = the first frame
      from which the patch holds its final value until the next press;
      `first_change` recorded beside it. Window at (100,100), 1600x1000; 60
      presses at 1003 ms per run. Why not PresentMon: WebView2's presents come
      from a separate GPU process and are hard to correlate with inputs. Why
      not the Event Timing API: the draw is asynchronous after IPC and a
      worker, so the event's next paint is not the image's frame.
    - Driver to state (`run-ab.ps1`): per folder one warm run of each app
      (unused), then A B A B A B; before each run an idle gate (CPU load
      <= 15 %, no `cargo` / `rustc` / `link` / `msbuild` / `cl`); a run with a
      load sample over 15 % after the first, a compiler process or a non-zero
      probe exit is discarded and redone in its slot, up to 4 tries; 8 s
      settle before the first press. `summarize.py`: percentiles at sorted
      index `n / 2` and `n * 0.95`, per-run p50 / p95, the median of the three
      runs' p50s and of their p95s separately.
    - Files: `D:\Photos\tests\20261108-gpui-compare\ARW` (1616x1080 previews)
      and `\RAF` (4416x2944), 20 files each, so 60 presses wrap the folder
      three times and each file is timed three times per run.
    - Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM;
      RTX 4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a
      Crucial CT4000BX500 SATA SSD; WebView2 157.0.4341.60.
    - Date: the raw folder is named `20261120`, but the Tauri app log's first
      lines are stamped 2026-11-19 23:31 local. Read the first and last
      timestamps from the logs and state the session's real span.
    - Attempts, as counted at planning time (recount from `runs.txt`):
      25 lines = 4 warm runs + 21 measured attempts. 12 measured attempts are
      valid, one per slot; 9 were discarded, every one for a load sample over
      15 %, none for a compiler process: ARW 6 (A1 two, B1 two, A2 one, B3
      one), RAF 3 (B1, A3, B3 one each). Of the warm runs, 3 were flagged and
      not redone, which does not matter as they are unused: `ARW-warm-A` also
      ended with probe exit 2, having lost the foreground before press 31
      (`ARW-warm-A.probe.err.txt`), so the ARW GPUI warm run pressed only 30
      times. Present the attempts as a table like the gpui-port doc's (sample
      before, samples during, used).
    - Tables: keep `summary.md`'s four tables (ARW GPUI, ARW Tauri, RAF GPUI,
      RAF Tauri). Its columns are in alphabetical order; reordering them into
      pipeline order (as the two reference docs do) is fine, changing a value
      is not. Add one headline table: per folder and app, `screen` and
      `keypressToPixels` median p50 / p95, and screen minus in-app.
    - Observations to verify, then state. At planning time they held:
      - Headline, median of 3, p50 / p95 ms: ARW `screen` GPUI 34.34 / 47.93,
        Tauri 48.37 / 69.53; RAF GPUI 115.46 / 140.75, Tauri 131.11 / 150.33.
        GPUI is ahead by 14.03 ms (ARW) and 15.65 ms (RAF) at p50, and by
        21.60 ms (ARW) and 9.58 ms (RAF) at p95. Give both p95 numbers, not
        one adjective for both folders.
      - In-app `keypressToPixels` p50: ARW GPUI 20.79, Tauri 20.20 (even, as
        the old comparison read); RAF GPUI 67.51, Tauri 115.60.
      - Tauri ARW: `frame1` 28.50, `frame2` 44.80, `screen` = `frame2` + 3.57;
        `screen` minus `keypressToPixels` = 28.17 ms happened after the old
        endpoint. Tauri RAF: `frame1` 115.80 and `frame2` 117.90 sit on
        `keypressToPixels` 115.60 (the 23.20 ms synchronous `draw` runs past
        the frame), `screen` is 15.51 ms later.
      - GPUI: `screen` minus `keypressToPixels` is 13.55 ms on ARW and
        47.95 ms on RAF. So the in-app `present` stamp precedes the frame
        reaching the DWM by far more on the large image. Write the cause
        (possibly the 4416x2944 texture's upload or sampling on the GPU) as a
        hypothesis and an open question, not as a finding: nothing in this
        data separates it from other causes.
      - `first_change_ms` equals `settled_ms` on every press of all 12 valid
        runs (checked per press in the CSVs), so the patch never showed an
        intermediate value.
      - The CSV `frames` column: 1-3 presented frames per press for GPUI ARW,
        2-3 for GPUI RAF, 3-7 for Tauri. State it only if it serves the
        reading.
    - Caveats to carry:
      - `LastPresentTime` is the DWM present, not scanout: display and panel
        latency are in no number, and are equal for both apps.
      - At 59 Hz a frame is about 16.9 ms, so `screen` is quantized to the
        refresh; a 14-16 ms difference is about one frame.
      - Differences of percentiles are not per-press differences.
      - `ARW-B3-try2`, a valid Tauri run, has one press (press 16) whose
        Desktop Duplication `AccumulatedFrames` was 2 (`accumulated>1=1` in
        `runs.txt`). The other 11 valid runs have none.
      - Prefetch is off in both apps, so this is the uncached page turn.
      - The Tauri in-app numbers are printed to one decimal, GPUI's to two.
      - Only a same-session A/B is a valid comparison on this machine, as the
        gpui-port results found.
      - The `SendInput` stamp precedes the OS input path, so `screen` holds
        input delivery that neither in-app number has.
    - Reproduction section: the branch `measure/tauri-latency-ab` at
      `1e97309e` on `a9b0e165` (local only), the two build commands, the
      `latprobe` build, the `run-ab.ps1` invocation with its `-Gpui`, `-Tauri`,
      `-Probe`, `-Out` parameters and `python summarize.py <out dir>`. Write
      paths as placeholders or in backticks; nothing under `tools/latprobe`
      exists on main, so none of it may be a Markdown link (lychee would
      fail).
    - Links: only to files that exist on main or are added in this step,
      relative to this folder (`../20261109-gpui-port/results.md`,
      `../_archived/20261108-preview-latency-breakdown/results.md`,
      `raw/summary.md`, `raw/runs.txt`).
    - Commit: `docs(plans): record the Tauri vs GPUI page-turn latency A/B`.

## Trade-offs and risks

- **The tooling is not in the repository.** The branch and commit
  (`measure/tauri-latency-ab`, `1e97309e`) exist only in one local worktree.
  `summary.md` and `runs.txt` are copied into `raw/` so the numbers stay on
  record; the CSVs and logs stay local.
- **The GPUI under-report on large images is a hypothesis.** Following it up
  (where the 34 ms goes, whether the stamp should move) is outside this plan.
- **No Markdown formatter runs.** `ci:local` on a `.md`-only change runs lychee
  only, so table layout is a review matter.

## Progress
