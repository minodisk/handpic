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

# Revert the immediate paint of a keyed page on Windows

## Purpose

PR #1219 (`6809229f`) made a keyed page turn on Windows paint at once by
calling Win32 `RedrawWindow(RDW_INVALIDATE | RDW_UPDATENOW)` on the window's
HWND. That relies on GPUI internals outside its public API (gpui-pre-windows
drawing from `WM_PAINT` and presenting at once). The user decided to go back
to the public-API way, `cx.notify()` and GPUI's vsync tick, and accepts the
about one refresh it costs. After this plan the app holds no forced paint,
the docs describe the vsync-tick behavior as current again, and the
measurement stays on record with the reason for the revert, so the change is
proposed upstream rather than re-added here.

## Steps

- [x] Step 1: Revert #1219's code and restore the docs to the vsync-tick behavior
  - Done when:
    - `crates/app/src/presentnow.rs` is gone, and `presentnow`, `PresentNow`
      and `RedrawWindow` match nothing under `crates/app`.
    - `git diff c57ec2ff -- crates/app/src/main.rs crates/app/Cargo.toml crates/app/Cargo.lock CLAUDE.md docs/humans/performance.md docs/humans/performance.ja.md`
      is empty (no `raw-window-handle` / `windows 0.62.2` edge left on
      `handpic-app` in the lock).
    - `docs/agents/app.md` no longer has the section "A keyed page's frame
      does not wait for the vsync tick on Windows (Measured)" and holds the
      short Measured note described below in its place; its other
      `frameWait` rules and the "about 15 ms ... is the floor" sentence read
      as before #1220 / #1221.
    - `todo.md` no longer has "App: ruler check of the prefetch-hit page turn
      after the immediate paint (page-present-now)".
    - The archived `20261122-page-present-now` record says it was reverted
      and why; its `raw/`, tables and the `review-history` files are
      unchanged.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (macOS and Linux builds included).
  - Implementation approach (as far as it is known; omit if unknown):
    - A whole `git revert 6809229f` does not apply: it conflicts in
      `docs/agents/app.md` and in the plan's `learnings.md` / `plan.md`
      (moved to `_archived` by #1221) and would delete
      `docs/plans/review-history/page-present-now-step-2/review-20261122-0310.md`,
      which stays. Reverse-apply only the paths that are clean, e.g.
      `git show 6809229f -- CLAUDE.md crates/app/Cargo.toml crates/app/Cargo.lock crates/app/src/main.rs crates/app/src/presentnow.rs docs/humans/performance.md docs/humans/performance.ja.md | git apply -R --index`.
      No commit after `6809229f` touched these paths, so the result equals
      `c57ec2ff`; check with the `git diff` above.
    - `docs/agents/app.md`, by hand (`git show 6809229f`, `31096504` and
      `639ae97b`, each `-- docs/agents/app.md`, show the exact hunks):
      - Remove the whole section added by #1219 and extended by #1220 (the
        ruler paragraph, "Expect 1.5-2 ms at p50 ...", and the rule that
        `keypressToPixels` overstates the on-screen gain).
      - Undo #1220's two edits further down: back to "`frameWait`'s p50
        ranges over 6-10 ms across runs of one binary." and to "Rule: check
        every run's `frameWait` p50 before reading an A/B. One session ran
        at 120-145 ms ...".
      - Undo #1221's rewording: back to "the about 15 ms between `present`
        and the DWM present is the floor for any image size."
      - In the removed section's place add a short Measured note: on Windows
        GPUI (gpui-pre-windows 0.3.12) draws only on its vsync thread's tick
        (`DwmFlush`, then `RedrawWindow(RDW_INVALIDATE)`, then `WM_PAINT`),
        so a page waits `frameWait` for the tick, and its frame, presented
        just after that vblank, is composed at the next one: about one
        refresh more than necessary. Forcing the paint with
        `RedrawWindow(RDW_UPDATENOW)` was tried (#1219), gained a refresh on
        47 % (ARW) / 34 % (RAF) of presses (`screen` p50 33.51 to 25.10 /
        60.99 to 56.14 ms) and was reverted as outside GPUI's public API; a
        real fix belongs in GPUI (e.g. drawing at once when idle). Rule: do
        not re-add an app-side forced paint; propose it upstream instead.
        Link `../plans/_archived/20261122-page-present-now/results.md`.
    - `todo.md`: delete the one item #1221 added (heading at about line
      2100).
    - Archived record: add one sentence under the verdict of
      `docs/plans/_archived/20261122-page-present-now/results.md` saying the
      change was reverted because it relies on GPUI internals outside its
      public API, and one line in that plan's `plan.md` Progress saying the
      same. Its `learnings.md` is left as is. Leave
      `docs/plans/_archived/20261122-tauri-gpui-latency-ab-rerun/results.md`
      ("Painted to screen, per press") as it is.
    - Commit type: `revert(app): ...`, so the release notes pair it with
      #1219's `feat(app)`.

## Trade-offs and risks

- **The archived plan's "Trade-offs" text stays as written.** It already
  names "Dependence on GPUI internals" as a risk; only a Progress line and
  the sentence in `results.md` are added, so the record is not rewritten
  after the fact.
- **The rule about `keypressToPixels` overstating the screen gain goes with
  the section.** It only described builds with the forced paint. The new
  note's numbers keep the finding reachable through the archived results.
- **Performance goes back by measurement, not by estimate.** `screen` p50
  returns to about 33.5 ms (ARW) / 61.0 ms (RAF) on the ruler and `frameWait`
  p50 to 6-10 ms. No new measurement is planned; the pre-#1219 numbers are
  the A side of the archived A/B.

## Progress
