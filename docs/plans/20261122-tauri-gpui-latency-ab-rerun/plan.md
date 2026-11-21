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

# Tauri vs GPUI page-turn latency A/B, re-run after the fit-view decode scale

## Purpose

The archived Tauri vs GPUI latency A/B
(`docs/plans/_archived/20261120-tauri-gpui-latency-ab/results.md`) measured
GPUI when the fit view decoded a preview at full size. Since then the fit view
decodes at the smallest `n/8` DCT scale that covers the canvas (#1192, #1196).
That change was measured GPUI against GPUI only (#1201,
`docs/plans/_archived/20261121-fit-preview-downscale/results.md`). What the
Tauri vs GPUI comparison reads now is recorded nowhere, and numbers from
different sessions are not comparable on this machine.

The same-session re-run is done (2026-11-21 23:35 to 2026-11-22 00:14 local).
This plan records it: a `results.md` with the screen and in-app numbers, the
conditions and the one out-of-order substitution, and the raw output and the
scripts used beside it. No app code changes.

## Steps

- [x] Step 1: Record the re-run's results and raw output in this plan folder
  - Done when:
    - `raw/` in this folder holds `summary.md`, `runs.txt`, `endpoints.txt`,
      `endpoints.py`, `run-ab.ps1`, `redo-raf-b3.ps1` and `redo-raf-a3.ps1`,
      each the file of the measurement worktree's `out\20261121` folder
      unchanged (line endings aside). `raw/runs.txt` is the original log: 29
      lines, with `RAF-A3-try1` marked `bad=[]`, not the scratch copy that
      marks it superseded.
    - `results.md` exists in this folder and contains:
      - the measurement time and that both apps ran in one session,
        alternating, against the external ruler;
      - a headline table, per folder and app, of the median of the three
        runs' p50 / p95 of `screen`, of the in-app `keypressToPixels`, and
        of `screen` minus `keypressToPixels`, with the note that the last is
        a difference of medians, and how far GPUI is ahead on `screen` per
        folder at p50 and p95;
      - the per-run tables from `raw/summary.md` (Tauri's with `frame1` and
        `frame2`);
      - the aligned in-app endpoints (painted, next frame) from
        `raw/endpoints.txt`;
      - the measurement conditions: the two builds, the ruler, the driver
        and its one change, the folders, the machine configuration;
      - an explicit statement that the window is at (100,100), 1600x1000,
        the same as in the 20261120 A/B and in #1201, so the fit view's
        decode scale and the drawn size match both records;
      - the `decoded=` sizes of the GPUI `page` lines per folder;
      - the attempts: which runs are used, which were discarded and why, the
        `RAF-B3` redo after the main pass, and the substitution note:
        `RAF-A3-try1` passed, `RAF-A3-try2` was run at the user's request so
        the third RAF pair sits together in time, try2 replaces try1 in
        every median, `raw/summary.md` and `raw/endpoints.txt` were computed
        from a copy of `runs.txt` that marks try1 superseded, and what the
        RAF GPUI `screen` median would be with try1 instead;
      - a comparison with the 20261120 A/B, worded as an observation across
        sessions and not as a measured difference;
      - caveats, and how to reproduce;
      - links to every file in `raw/`, to the 20261120 A/B's `results.md`,
        to the fit-preview-downscale `results.md`, and to the
        latency-ab-inapp-endpoints `plan.md`.
    - Every number in `results.md` is a cell of `raw/summary.md`,
      `raw/endpoints.txt` or `raw/runs.txt`, or a difference of such cells
      that the text names as one.
    - Nothing outside this plan folder is changed (`todo.md`, the archived
      records and the code included).
    - `mise run ci:local` passes locally (it runs lychee for a changed
      `.md`), and the PR's GitHub checks pass.
  - Implementation approach:
    - Copy the seven raw files from
      `C:\Users\daisu\.herdr\worktrees\handpic\tauri-latency-ab\out\20261121\`.
      Do not copy anything else from there (per-run CSVs, logs, the two
      `.sqlite` files). `.gitattributes` normalizes the CRLF files to LF on
      commit; do not edit their content. `endpoints.py` is byte-identical to
      `docs/plans/_archived/20261120-tauri-gpui-latency-ab/raw/endpoints.py`;
      say so in `results.md`.
    - Model `results.md` on the two earlier records' structure and wording
      (verdict and headline table, what each number measures, conditions,
      attempts table, per-run tables, reading, aligned endpoints, caveats,
      reproduction). For "What each number measures", link to the 20261120
      record's section instead of repeating it, as the fit-preview-downscale
      record does.
    - Conditions to state:
      - A = GPUI: `main` at `d29807fb` (contains `9f650e9f`),
        `cargo build --release --locked` of `crates/app` in a temporary
        worktree, run with `--no-prefetch` and a temporary index and
        settings file.
      - B = Tauri: `1e97309e` on the local-only branch
        `measure/tauri-latency-ab`, the same build as in the 20261120 A/B.
      - Ruler `latprobe`, `run-ab.ps1` and `summarize.py` from `1e97309e`,
        unchanged, except that the driver copy (`raw/run-ab.ps1`) adds the
        1 px mouse move and back before each run that #1201 added, 5 lines,
        to keep the foreground.
      - ARW (1616x1080 previews) and RAF (4416x2944), 20 files each, under
        `D:\Photos\tests\20261108-gpui-compare`. Per folder one warm run of
        each app, then A B A B A B; 60 presses, one every 1003 ms; idle gate
        of at most 15 % load and no compiler process; up to 4 tries per slot.
      - Machine: Windows 11 Home 10.0.26200; i7-13700 (16C/24T); 32 GB RAM;
        RTX 4060 Ti, driver 32.0.16.1742; display 5120x1440 at 59 Hz; `D:` a
        Crucial CT4000BX500 SATA SSD; WebView2 157.0.4341.60. The same
        machine as both earlier records.
    - Headline numbers, median of 3 runs, p50 / p95 in ms (check each
      against `raw/summary.md` while writing):
      - ARW GPUI: `screen` 33.52 / 41.52, `keypressToPixels` 18.54 / 26.99,
        difference 14.98 / 14.53.
      - ARW Tauri: 45.15 / 69.01, 17.80 / 24.90, 27.35 / 44.11; `frame1`
        25.60 / 36.30, `frame2` 42.30 / 53.10.
      - RAF GPUI: 60.14 / 72.83, 45.16 / 57.98, 14.98 / 14.85.
      - RAF Tauri: 125.05 / 147.53, 113.30 / 129.60, 11.75 / 17.93; `frame1`
        113.40 / 129.70, `frame2` 116.20 / 131.20.
      - GPUI ahead on `screen`: ARW 11.63 / 27.49, RAF 64.91 / 74.70.
      - Aligned (`raw/endpoints.txt`): GPUI painted ARW 7.97 / 9.97, RAF
        35.85 / 45.19; Tauri painted equals its `keypressToPixels`; GPUI
        next frame equals its `keypressToPixels`; Tauri next frame is
        `frame2`.
    - Used runs: ARW `A1-try1`, `B1-try2`, `A2-try1`, `B2-try1`, `A3-try1`,
      `B3-try4`; RAF `A1-try3`, `B1-try1`, `A2-try1`, `B2-try3`, `A3-try2`,
      `B3-try5`. Discarded: `ARW-B1-try1`, `ARW-B3` tries 1 to 3, `RAF-A1`
      tries 1 and 2, `RAF-B2` tries 1 and 2, `RAF-B3` tries 1 to 4 (load
      over 15 % three times, probe exit 2 once: the window lost the
      foreground to another process). `ARW-warm-A` was flagged (load, probe
      exit 2) and is a warm run, unused either way. Take each line's load
      samples and reason from `raw/runs.txt`.
    - The redo order in time: the main pass ended with `RAF-B3-try4`; then
      `RAF-B3-try5` (`raw/redo-raf-b3.ps1`), then `RAF-A3-try2`
      (`raw/redo-raf-a3.ps1`). `RAF-A3-try1` read `screen` 60.19 / 73.94,
      try2 60.64 / 72.04. With try1 the RAF GPUI `screen` median would be
      60.14 / 73.04 instead of 60.14 / 72.83 (from `raw/runs.txt`).
    - Reproduction must say that `summarize.py` and `endpoints.py` take
      every non-warm `bad=[]` line, so they need the `RAF-A3-try1` line of
      `runs.txt` marked bad to give `raw/summary.md` and
      `raw/endpoints.txt`, and that the per-press CSVs and app logs they
      read stay in the measurement worktree's `out\20261121` folder, not in
      the repository.
    - GPUI `decoded=` sizes to state: RAF `1656x1104` on 51 pages and
      `1104x736` on 9; ARW `1212x810` on 57 and `808x540` on 3. Verify the
      counts in the six used GPUI runs' `.stdout.txt` files in `out\20261121`
      before writing "in every run".
    - Caveats to carry: `ARW-B2-try1`, a used Tauri run, has
      `accumulated>1=1` in `runs.txt`; `screen` is the DWM present, not
      scanout, and is quantized to the 59 Hz refresh; differences of
      percentiles are not per-press differences; prefetch is off in both
      apps; `RAF-B3` and `RAF-A3` ran out of the A B A B A B order.
    - Claims about the per-press CSVs go in only if checked against the CSVs
      in `out\20261121`; otherwise leave them out. `screenFirstChange` equals
      `screen` in every cell of `raw/summary.md`.
    - Comparison with the 20261120 A/B (GPUI ARW 34.34 / 47.93, RAF 115.46 /
      140.75; Tauri ARW 48.37 / 69.53, RAF 131.11 / 150.33): Tauri moved
      little between the sessions and GPUI's RAF `screen` p50 is about 55 ms
      lower. That record says numbers from different sessions are not to be
      compared, so word this as an observation consistent with #1201's
      same-session result (RAF `screen` 112.60 to 64.94), and note that
      GPUI's RAF `screen` minus `keypressToPixels` (14.98) now equals its
      ARW one (14.98).
    - Links: while this folder is under `docs/plans/`, a link to an earlier
      record is `../_archived/<folder>/results.md`. The wrap-up's archive move
      rewrites them to `../<folder>/...`.
    - Commit message: `docs(plans): record the Tauri vs GPUI latency A/B re-run`.

## Trade-offs and risks

- **Links to the earlier records and the archive move.** No relative path to
  `docs/plans/_archived/<earlier>/` works both from `docs/plans/<this>/` and
  from `docs/plans/_archived/<this>/`. Step 1 writes `../_archived/<earlier>/...`;
  the wrap-up's move must rewrite those links to `../<earlier>/...`, or the
  PR's lint fails.
- **The 20261120 record gets no forward pointer.** Its verdict describes GPUI
  before the fit-view decode scale. This plan leaves it untouched (the user's
  choice).
- **The closed `todo.md` section.** "App: the in-app `keypressToPixels` stops
  about 48 ms before the DWM present on 4416x2944 RAF previews" is still in
  `todo.md` with its only item ticked by #1201. Step 1 does not touch
  `todo.md`; removing the section is the wrap-up's todo curation.
- **Cross-session numbers.** The comparison with the 20261120 A/B is across
  sessions and is kept as an observation only.
- **The raw tables are not reproducible from the repository alone.** The
  per-press CSVs and app logs are not committed, as in the earlier records,
  and `raw/summary.md` / `raw/endpoints.txt` came from a `runs.txt` copy with
  one line changed. `results.md` states both.
- **`RAF-A3-try2` replaces a run that passed.** Keeping try1 would follow the
  driver's rule (first passing try per slot); try2 keeps the third RAF pair
  together in time, as the user asked. The medians differ only at `screen`
  p95 (73.04 against 72.83), and `results.md` gives both.

## Progress
