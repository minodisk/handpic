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

# GPUI prefetch ring

## Purpose

The GPUI native app (`crates/native`) decodes a preview only when its file
becomes current: `Pager` keeps one read in flight for the current index, and
`self.image` is the only decoded preview held. Every page turn therefore pays
read + decode + orient (about 18 ms on a 1616x1080 ARW, about 50 ms on a
4416x2944 RAF). The Tauri app avoids this with `crates/app/ui/src/prefetch.ts`,
a ring of the decoded neighbours of the current file, capped by total pixels.
The native app has no equivalent (checked: `pager.rs`, `load` / `loaded` in
`main.rs`, `hold.rs`; `retired` is a release queue, not a cache).

After this work the native app holds the decoded previews next to the current
file, and an arrow key onto a held neighbour draws it with no read and no
decode: the only stages left are the frame wait, the paint and the present.
The ring gives entries up on a file list change, a re-listing and a folder
switch, and its total stays under the pixel cap. This is one milestone of the
port (`../20261109-gpui-port/plan.md`).

## Steps

- [x] Step 1: Port the prefetch ring and draw a held neighbour on a page turn
  - Done when:
    - `crates/native/src/prefetch.rs` holds the ring's pure logic, a port of
      `prefetch.ts`, with unit tests porting the cases of
      `crates/app/ui/src/prefetch.test.ts`, plus one asserting that the held
      pixel total never exceeds `PREFETCH_PIXEL_CAP` after any `settle` or
      `keep`.
    - Once the current file's preview is on screen and no scan pass runs, the
      app reads the next neighbour in the paging direction, then the previous
      one, one prefetch in flight at a time.
    - A page turn onto a held neighbour shows it without a read: its `page`
      line says `source=prefetch` with `read`, `decode` and `orient` at 0, and
      everything that belongs to the current file's result still happens
      (status line, meta pane, `f` overlay, on-demand analysis, dwell, 1:1
      facts).
    - A ring hit while an older read is still out does not re-read the file
      that is already shown, and leaves nothing in flight once that read
      lands.
    - The ring is cleared on a folder open and on a re-listing, keeps only
      listed files on a `refilter`, and every image it gives up goes through
      `retire` (so `cx.drop_image` releases it).
    - `--no-prefetch` (see Trade-offs) turns the ring off, and `args.rs`'s
      tests and `USAGE` cover it.
    - `learnings.md` records the `--auto` comparison with and without
      prefetch on ARW and RAF (p50 / p95 `keypressToPixels` and stages, the
      hit rate from `source=`), and the memory check (process peak memory
      with and without, against the cap).
    - `mise run ci` and `mise run native:check` pass (`ci` does not run the
      native workspace's clippy and tests).
    - `CLAUDE.md`'s `crates/native` module list names `src/prefetch.rs`, and
      `docs/agents/native-app.md` has entries for the ring's rules and the
      measured numbers.
    - The PR description notes the manual check on Windows by the user (an
      arrow key onto a neighbour shows it at once; a folder switch and a
      filter change show no other folder's or hidden file's image) as
      pending, and `learnings.md` lists it under deferred issues.
  - Implementation approach (as far as it is known; omit if unknown):
    - Before starting and before opening the PR, re-check `origin/main`:
      `main.rs` is edited by several sessions. Keep the `main.rs` diff to
      wiring; anything that can be decided without GPUI goes in
      `prefetch.rs` or `pager.rs`.
    - Files: new `crates/native/src/prefetch.rs`; `crates/native/src/pager.rs`
      (see the ring hit below); `crates/native/src/main.rs` (`go`, `loaded`,
      `opened`, `refilter`, the scan's end, a new prefetch send and the hit
      path); `crates/native/src/timing.rs` (`Page` gains the source);
      `crates/native/src/args.rs`; `CLAUDE.md`; `docs/agents/native-app.md`.
      `hold.rs`, `zoom.rs`, `overlay.rs` and `peaking.rs` should need no
      change; record why in `learnings.md` if one does.
    - `prefetch.rs` ports `prefetch.ts` one to one: `PREFETCH_PIXEL_CAP`
      (32,000,000), `neighbours` (the paging direction first), and a ring
      generic over what it holds with `plan`, `next`, `still_wanted`,
      `settle`, `take`, `keep`, `clear`, `retain`, `forget`, the `refused`
      and `crowded` sets and the one `pending` with its stale mark. Each
      method returns the entries given up, for the caller to retire, as the
      TypeScript returns the bitmaps to close. The ring must not name GPUI
      types; it only needs each entry's pixel count (the upright image's
      width x height).
    - Ownership stays single, as in `prefetch.ts`: an image is either in the
      ring or on screen. `take` removes it for the view; the view hands the
      image it stops drawing back through `keep` only after another image
      replaced it, and retires whatever `keep` returns. This matters more
      here than in the Tauri app because a retired image is released with
      `cx.drop_image`; an image still in the ring must never reach
      `retired`.
    - A ring entry holds what `loaded`'s current-file arm needs and nothing
      tied to a read: the image, `px`, `orientation`, `jpeg_kb`, the
      `Metadata` and the focus location. It does not hold the `Paused`
      guard: a prefetch's `Paused` is dropped when its result lands (a hit
      has no pause to hold, and `PaintHold` is only released by a paint).
    - To hand the shown image back to the ring the view must know the shown
      file's path and the rest of its entry, and whether it is still
      keepable. Today it has `self.image` and `self.shown` (an index and the
      `Metadata`). A re-listing may have changed any file's bytes, so the
      shown image stops being keepable there (the Tauri `shown.keepable =
      false`). A stale frame drawn under a held key is an ordinary shown
      image and may be kept if its file is wanted.
    - The paging direction: the Tauri app derives it from the new index
      against the last shown one (`index < lastShownIndex ? -1 : 1`), 1
      after a folder open.
    - When to ask for a prefetch, mirroring `requestPrefetch`'s call sites:
      after the current file's result is shown (a read or a hit), after a
      prefetch settles, after a `refilter` that leaves the current file
      shown, and when the scan ends. The conditions: prefetch is enabled,
      the pager has nothing in flight, the image on screen is the current
      file's, and `self.scanning()` is false. Each ask runs `plan` and then
      `next`.
    - The prefetch read reuses `load` through `guarded` on the background
      executor. Its result is dropped by the ring's own stale mark, not by
      `self.order`: the ring is keyed by path, and `clear` / `retain` mark
      the pending read stale. When the result lands and `still_wanted` is
      false, it is settled as abandoned and its image retired.
    - The ring hit in `go`: take the new current file from the ring, then
      `plan`, as `show()` does in `main.ts`. On a hit, do what `loaded`'s
      current `Ok` arm does (status line, `self.shown`, the image swap with
      the previous image going to `keep`, `marks.show`, `zoom.shown`,
      `analyze_current`, `start_dwell`, `cx.notify`, the pending `Page` with
      zero `read` / `decode` / `orient` and the source), without
      `hold.hold`. Share the code with that arm rather than duplicating it
      if that keeps the diff small.
    - `Pager` needs to learn of a hit. Today `go` always calls `request`,
      and a stale result always sends `next` (`Finished::Stale { draw,
      next }`); with the current file already on screen from the ring, both
      would re-read it. Decide in `pager.rs`, with unit tests, how a hit is
      told (for example `finish` sending nothing when `shown` is the current
      index and clearing the in-flight marker). The existing rule in
      `native-app.md` ("stepping back onto the file whose stale frame is on
      screen still reloads it") is about a stale frame, which has not had
      its analysis, dwell or `page` line; keep that case reloading and
      distinguish it from a hit, which has had them.
    - Release hooks: `opened` (both the folder open and the `carry`
      re-listing) clears the ring and marks the shown image not keepable;
      `refilter` calls `retain` with the new list. No trash, delete or
      rename exists in the native `main.rs` at the time of planning, so
      `forget` has no caller yet (see Trade-offs); note in `native-app.md`
      that whichever plan adds those operations
      (`../20261113-gpui-tree-trash/plan.md`,
      `../20261113-gpui-folder-dialogs/plan.md`) must call `forget` with
      the paths that no longer name what the ring read.
    - `timing.rs`: the `page` line gains `source=read|prefetch` so the hit
      rate can be read from a run. Check `docs/agents/native-app.md` and
      `docs/humans/*.md` for a description of the line's fields and update
      it (with the `.ja` pair for `docs/humans`).
    - Verify during implementation, by reading rather than assuming:
      - images the ring gives up when no repaint follows (an eviction on
        `settle`) are released: `retired` is drained only by the preview
        canvas's paint, so check whether a `cx.notify()` is needed for them
        not to linger past the cap until the next frame;
      - retiring an image that was never painted is fine (today a stale
        `Ok` that is not drawn already does this);
      - under a held key the pager stays in flight, so no prefetch starts
        during the hold and at most the one started before it is out; the
        number of page reads per key sequence does not grow.
    - Measure, in a release build (`mise run native:release`, NASM on
      `PATH`), only on the copies `D:\Photos\tests\20261108-gpui-compare\ARW`
      and `...\RAF` (never a folder outside `D:\Photos\tests\`):
      - use a warm index: prefetch is skipped while a scan runs, so run each
        folder once with its `--index` (kept under a new folder in
        `D:\Photos\tests\`) until the scan ends, and discard that run;
      - then alternate `--auto 60` with and without `--no-prefetch`, three
        runs each per folder in one session, as the A/B rule in
        `native-app.md` asks for work on the page path; record CPU load
        before each run if other sessions are building;
      - record p50 / p95 of `keypressToPixels`, `paint` and `present` (the
        first paint of a prefetched image still uploads its texture, so
        `paint` may grow on a hit), and the hit rate. `--auto` presses
        `Next` every 1003 ms and wraps at the end of the list, so every
        press except a wrap should be a hit;
      - where the prefetch starts is decided by this measurement: starting
        it when the current result is shown is the simplest; if `paint` or
        `present` of the current page grows because the neighbour's decode
        and orient (8 threads) overlap them, start it from the paint's
        deferred callback instead;
      - memory: record the process's peak memory with and without prefetch
        on the RAF folder (two 13 MP neighbours, 26 MP, fit the 32 MP cap)
        and compare the difference with the cap's 128 MB of BGRA. How to
        sample it (for example polling the process from PowerShell) is the
        implementer's choice.
    - `docs/agents/native-app.md`: add entries in the file's form (a `###`
      heading with its tag and `- Rule:` lines): the ring's ownership rule
      and release hooks under the backend / GPUI sections, the measured
      numbers under Timings, the manual check as `Pending`.

## Trade-offs and risks

- **Neighbours at the ends of the list.** `prefetch.ts`'s `neighbours` drops
  out-of-range indices because the Tauri app clamps at the ends; the native
  `Pager` wraps on `Next` / `Previous`. A faithful port never prefetches
  across the wrap, so a wrap is always a miss (and three presses of an
  `--auto 60` run on 20 files are misses). Making `neighbours` wrap would
  make them hits but departs from the port. The plan assumes the faithful
  port (approved by the user).
- **`forget` has no caller yet.** Rename, trash and delete are not in the
  native app at planning time. Porting `forget` now, with its tests, keeps
  the module a complete port but needs an `#[expect(dead_code, reason =
  ...)]` (the form `scan.rs` and `settings.rs` use) until another plan
  wires it. The plan ports it now. Either way the trash and folder-dialogs
  plans need the note that they must call it.
- **The switch to turn prefetch off.** A `--no-prefetch` flag in `args.rs`
  is the smallest switch that lets one binary run both sides of the A/B; it
  stays in the binary and in `USAGE`.
- **No prefetch while a scan runs.** Ported from the Tauri app: a prefetch
  takes `ReadSlots::pause`, so filling the ring during a scan would slow the
  scan for reads the user may never look at. The consequence is that page
  turns during pass 1 / pass 2 get no hits. Whether to prefetch during a
  scan is an open todo for the Tauri app too ("App: prefetch the
  neighbouring previews during a scan").
- **A page turn onto the file whose prefetch is in flight reads it twice.**
  The Tauri app does the same: the page read never waits for a prefetch.
  Adopting the pending prefetch as the page's result would save the read
  but adds a state the pager does not have. The two decodes also compete
  for cores, so a miss right after a hit may be slower than a miss today;
  `--auto`'s 1003 ms interval does not show this.
- **The `page` line's format changes** (`source=`). Archived results quote
  the old lines; nothing parses them. Without the field the hit rate cannot
  be measured.
- **Memory beyond the pixel cap.** The cap counts the ring's pixels (128 MB
  of BGRA at 32 MP). The image on screen is extra, and an entry that was
  painted before it went back to the ring keeps its GPU texture too, since
  `cx.drop_image` is not called for it. The measurement records the real
  difference; whether to release the texture of a kept entry (and upload it
  again on the next hit) is not decided here.
- **Not covered:** the `todo.md` item "Native: a held `→` on RAF previews
  barely draws" is about a held key, where the pager stays in flight and no
  prefetch runs; this plan does not close it. The 1:1 crop is not
  prefetched.

## Progress
