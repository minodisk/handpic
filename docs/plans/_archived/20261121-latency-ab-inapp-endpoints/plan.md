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

# Latency A/B: the in-app numbers on aligned endpoints

## Purpose

The archived Tauri vs GPUI latency A/B
(`docs/plans/_archived/20261120-tauri-gpui-latency-ab/results.md`) reports the
two apps' in-app `keypressToPixels` side by side (ARW p50: GPUI 20.79 ms, Tauri
20.20 ms) and says they do not share an endpoint: GPUI's ends at the `present`
stamp, after `frameWait` and `present`; Tauri's ends at the return of the
canvas `drawImage`. It does not say what the in-app numbers read once the
endpoints are aligned.

The per-press stage values of all 60 presses of the 12 used runs exist, so the
aligned comparison needs no new measurement. This plan adds it to that
`results.md` as one section, for two endpoints ("image painted, before any
vsync wait" and "next frame"), and keeps the script and its output in that
folder's `raw/` so the numbers can be recomputed. The result lives in the
archived folder, next to the tables it is derived from; this plan folder only
points at it.

## Steps

- [x] Step 1: Add the aligned in-app endpoint comparison to the archived A/B
      results, with its script and output
  - Done when:
    - `docs/plans/_archived/20261120-tauri-gpui-latency-ab/raw/endpoints.py`
      and `raw/endpoints.txt` exist and are the measurement worktree's files
      unchanged (line endings aside).
    - `results.md` in that folder has a new section that, for ARW and RAF,
      gives per run and as the median of 3 the p50 / p95 of:
      - "painted": GPUI `keypressToPixels - frameWait - present` per press,
        against Tauri `keypressToPixels` as logged;
      - "next frame": GPUI `keypressToPixels`, against Tauri `frame2`, with
        Tauri `frame1` for reference.
    - The section says, in one sentence per endpoint and folder, which app is
      faster and by how much at p50 and p95, and carries the caveats listed
      below.
    - Every number in the section is `endpoints.txt`'s, and the GPUI
      `nextFrame`, Tauri `painted`, `frame1` and `frame2` cells equal the
      `keypressToPixels`, `frame1` and `frame2` cells of the existing tables
      in `results.md`, per run and in the median row.
    - No existing table, verdict or Reading bullet of `results.md` is changed,
      and nothing outside the two plan folders is changed.
    - `mise run ci:local` passes locally (it runs lychee for a changed `.md`)
      and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Sources, in the measurement worktree (not in this repository):
      `C:\Users\daisu\.herdr\worktrees\handpic\tauri-latency-ab\out\20261120\endpoints.py`
      and `endpoints.txt` beside it. Copy both into
      `docs/plans/_archived/20261120-tauri-gpui-latency-ab/raw/` without
      editing. `endpoints.txt` has CRLF line endings there; `.gitattributes`
      (`* text=auto eol=lf`) normalizes it on commit, so compare ignoring line
      endings (for example `diff --strip-trailing-cr`).
    - Do not re-measure and do not re-run the apps. Re-running
      `python endpoints.py <out dir>` against that `out\20261120` folder and
      diffing it with `endpoints.txt` is a cheap check worth doing; record the
      outcome in `learnings.md`.
    - Take every number from `endpoints.txt`, not from this plan. The medians
      this plan expects (p50 / p95, ms), to compare against, not to copy:
      - ARW painted: GPUI 8.76 / 11.71, Tauri 20.20 / 29.40.
      - ARW next frame: GPUI 20.79 / 29.01, Tauri `frame2` 44.80 / 54.30
        (`frame1` 28.50 / 37.70).
      - RAF painted: GPUI 57.01 / 68.03, Tauri 115.60 / 134.90.
      - RAF next frame: GPUI 67.51 / 79.46, Tauri `frame2` 117.90 / 135.70
        (`frame1` 115.80 / 135.10).
      If `endpoints.txt` differs from these, stop and record it rather than
      pick one.
    - The differences (Tauri minus GPUI, of the medians) these give, to be
      recomputed from the file: ARW painted 11.44 / 17.69; ARW next frame
      24.01 / 25.29 against `frame2` (7.71 / 8.69 against `frame1`); RAF
      painted 58.59 / 66.87; RAF next frame 50.39 / 56.24 against `frame2`
      (48.29 / 55.64 against `frame1`). GPUI is the faster app in every one.
    - Placement: a new `##` section between `## Reading` and `## Caveats`
      (for example `## In-app endpoints, aligned`). Leave the verdict table,
      the Tables section and the Reading bullets as they are. Small additions
      outside the new section are fine where they only name the new files:
      the paragraph listing the raw output (`raw/summary.md`, `raw/runs.txt`)
      and the Reproduction list (`python endpoints.py <out dir>`, with the
      script named as `raw/endpoints.py` of that folder, since it is not
      under `tools/latprobe`).
    - Format: follow the existing tables, cells `p50 / p95` in ms, one row
      per run (1, 2, 3 in the order of `endpoints.txt`, which is the slot
      order A1 / A2 / A3 and B1 / B2 / B3) and a bold `median of 3` row. The
      script's key names are `painted`, `nextFrame` and `frame1`; for Tauri
      `nextFrame` is `frame2`. The layout (one table per folder with both
      apps, or one per folder and app) is the implementer's choice.
    - State the method in the section: which runs (`runs.txt` lines with
      `bad=[]` that are not warm runs, the 12 used ones), which lines (GPUI
      `page` lines from `<run>.stdout.txt`, Tauri from `<run>.app.log`, 60
      per run), and the percentile rule, which is the one already described
      under Measurement conditions (sorted index `n / 2` and `n * 0.95`; the
      median row is the median of the three runs' p50s and, separately, of
      their p95s). `endpoints.py`'s `pct` implements that rule.
    - Caveats the section must carry (each checked at planning time):
      - GPUI "painted" is not a timestamp. `keypressToPixels` is the key to
        the `present` stamp (`crates/app/src/timing.rs`, `Page::stages`), and
        the value is that minus two stage durations. `frameWait` sits between
        `hop` and `paint`, so the value is the time to the end of `paint` had
        `paint` not waited for the frame, not a moment that occurred.
      - Tauri `keypressToPixels` includes its own `wait` stage (it equals
        `wait` + `total` in the log lines); it is used as logged.
      - Tauri's lines carry one decimal, GPUI's two; the summary prints two.
      - Differences of percentiles are not per-press differences. The GPUI
        "painted" percentiles are of the per-press differences, so they are
        not the `keypressToPixels` percentile minus the `frameWait` and
        `present` percentiles of the existing tables.
      - Neither in-app endpoint is the DWM present. The `screen` column
        remains the fair end-to-end comparison.
      - On RAF, GPUI's `screen` is 47.95 ms after its `keypressToPixels`
        (13.55 ms on ARW), the open question of `results.md` and of the root
        `todo.md` ("Native: the in-app `keypressToPixels` stops about 48 ms
        before the DWM present on 4416x2944 RAF previews"). So GPUI's in-app
        endpoints under-report on the large image, and the RAF in-app leads
        here overstate what reaches the screen (15.65 ms at p50).
      - On RAF, Tauri's `frame1` and `frame2` sit within 2.30 ms of its
        `keypressToPixels` (the existing Reading bullet), so "next frame"
        there is not a refresh after the draw as it is on ARW.
    - Links: only relative links to files that exist after this step
      (`raw/endpoints.py`, `raw/endpoints.txt`, and the ones already in the
      file). Paths in the measurement worktree go in backticks. In this
      plan folder, name the archived folder in backticks rather than as a
      link, because the wrap-up's archive move changes this folder's depth.
    - Stage only the named paths. The worktree holds an untracked
      `crates/native/` folder that is not part of this change.
    - Commit: `docs(plans): compare the latency A/B's in-app numbers on aligned endpoints`.

## Trade-offs and risks

- Editing an archived plan's `results.md`. The comparison is derived from that
  measurement's raw data and belongs beside its tables, and the request asks
  for it there. The edit is additive: no existing number, table or reading
  changes.
- "Next frame" on RAF for Tauri. `frame1` and `frame2` sit on the draw's end
  there, so the Tauri side of that row is not a vsync later. The plan keeps
  the requested comparison and states this as a caveat.
- The script cannot run from the repository alone: it reads the per-run logs,
  which stay in the measurement worktree's `out\20261120` folder.
  `raw/endpoints.txt` is the kept output.

## Progress

- (2026-11-21) Step 1 complete: added the `## In-app endpoints, aligned` section to the archived A/B `results.md`, with `raw/endpoints.py` and its kept output `raw/endpoints.txt`; the re-run reproduced `endpoints.txt` exactly.
