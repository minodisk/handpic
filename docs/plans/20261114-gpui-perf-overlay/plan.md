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

# The performance overlay for the GPUI app

## Purpose

The Tauri app's `d` key shows a graph along the bottom of the viewer: the
last 60 seconds of Handpic's and the system's CPU, Handpic's read throughput,
the system's disk busy time, and one bar per preview switch. The GPUI app
(`crates/native`) has the sampler available since the gpui-companions plan
moved it into `crates/backend/src/perf.rs`, but no overlay. This plan ports
the overlay, which is the "performance overlay (`perf.rs` sampler)" part of
Step 9 of [the gpui-port plan](../20261109-gpui-port/plan.md). No other
sub-plan covers it.

Out of scope, owned by other plans: pass 3, the ranking, the prefetch ring,
the settings modal, the MCP companion, the folder tree, trash, the folder
dialogs.

## Steps

- [x] Step 1: The performance overlay on `d`: the model, the sampler wiring, the graph, the docs
  - Done when:
    - In the native app the keymap action `performance` (default key `d`,
      `crates/backend/src/shortcuts.rs`) toggles the overlay, so a user's
      override of the key applies as it does in the Tauri app.
    - Opening the overlay starts the backend's `perf::Sampler` and a
      500 ms redraw; closing it stops both and drops what was recorded.
      Nothing samples, records or redraws while it is off. It is off at
      launch and not remembered.
    - The graph is a 120 px band along the bottom of the preview area,
      painted over the fit view and the 1:1 view alike without moving them
      and taking no input, with the newest point at the right edge and
      `x = width - (now - at) / 60 s * width`. It draws what `drawPerf` in
      `crates/app/ui/src/main.ts` (lines 2156-2351) draws:
      - the `--card` background (`#171717`) at 0.85 alpha;
      - three grid lines at 0, 50 and 100 % in the border color;
      - the four lines: `Handpic CPU` (2 px solid) and `System CPU` (1 px
        dashed `[4, 3]`) in `--chart-cpu` `#60a5fa`, `Handpic disk` (the
        read throughput, clamped to and scaled by the read axis maximum,
        2 px solid) and `System disk` (1 px dashed) in `--chart-disk`
        `#4ade80`; a sample without a disk reading breaks the `System disk`
        line, and with no reading in the window that series has neither a
        line nor a legend entry;
      - one 2 px bar per preview switch in `--chart-latency` `#f87171`,
        stacked from the bottom by stage with alpha `1 - n * 0.11`;
      - the one-line legend at 11 px (`100%  60 s`, each shown line with
        its swatch, `Latency`, the stage names in their alphas) and, right
        aligned, `<read max in MB/s> MB/s  <latency max> ms`.
    - The model follows `crates/app/ui/src/perf.ts`: a 60 s window pruned
      on every add and every view, the latency axis maximum being the
      largest value in the window and never below 500 ms, the read axis
      maximum never below 10 MB/s, recording only between start and stop,
      stop dropping everything.
    - A switch's latency starts at the key or strip click that turned the
      page, else at the selection change (`latencyStart`), and ends where
      the native `page` line's `keypressToPixels` ends (the deferred
      `presented` stamp).
    - The stdout `page` line and `timing::Summary` are unchanged: still
      only for keyed pages, same text.
    - `crates/native` has unit tests for the pure model that cover what
      `crates/app/ui/src/perf.test.ts` covers (nothing recorded until
      started, stop drops, insertion order, the window's pruning on add and
      on view, both floors and both maxima, `latency_start`), and for the
      graph's pure layout (the x of a time, the y of a value, the stage
      stack, which lines are shown).
    - `mise run native:check` passes.
    - `CLAUDE.md`'s layout paragraph names the new native modules, and
      `docs/agents/native-app.md` has an entry for the overlay.
    - `../20261109-gpui-port/plan.md` records the progress: a line under
      its Step 9 that the performance overlay was done by this plan (Step 9
      stays open), and a Progress entry.
  - Implementation approach (as far as it is known; omit if unknown):
    - Rebase on `origin/main` before starting and again before opening the
      PR. `crates/native/src/main.rs` is edited by parallel sessions
      (settings, tree / trash, folder dialogs, companions), so keep its
      diff to the `mod` lines, one `key_down` arm, the fields, the sample
      route and one call in the preview canvas's paint closure. Do not
      touch `crates/native/src/keys.rs`: `Bindings` already resolves `d`
      to `performance`, which today falls into the catch-all arm and is
      logged as "not handled yet".
    - New module `crates/native/src/perf.rs` for the pure model, a port of
      `perf.ts`: the constants (`WINDOW`, `LATENCY_FLOOR`,
      `READ_FLOOR_BPS`), the stamped sample (the backend's `perf::Sample`
      plus its arrival time on the main thread, as the Tauri listener
      stamps `performance.now()`), the latency point, `latency_start` and
      the model with `start` / `stop` / `add_sample` / `add_latency` /
      `view(now)`. No GPUI types in it, so its tests run without a window.
      The existing tests in `timing.rs` build `Instant`s as
      `Instant::now() + offset`, which works here too.
    - New module for the drawing (e.g. `crates/native/src/perfgraph.rs`):
      the pure layout plus one `paint(bounds, window, cx)` function the
      preview canvas's paint closure calls after the image, the 1:1 crop
      and the `f` overlay, and also when there is no image. The band is
      the bottom 120 px of the canvas's bounds.
    - The add arm goes next to the `focus` / `zoom` / `peaking` arms in
      `key_down` (`main.rs` near line 1284), as an action arm and not a
      literal `"d"` (see "Every keystroke goes through the keymap" in
      `docs/agents/native-app.md`).
    - The sampler's callback runs on its own thread, so it only sends
      (rule "Callbacks from other threads go through one channel" in
      `docs/agents/native-app.md`). `Sampler::start` takes
      `FnMut(Sample) -> Result<(), E>` and logs the first error once.
      `Sampler::stop` only signals, so one late sample can still arrive
      after a close; the model ignores it while not recording.
    - The 500 ms redraw is a `cx.spawn` loop on
      `cx.background_executor().timer(..)` held as an `Option<Task<()>>`
      and dropped on close, as `zoom_timer` is held; a sample or a latency
      point also calls `cx.notify()`.
    - The latency point is completed where the `page` line is, in the
      paint closure's `cx.defer` (`main.rs` near line 2431), so the model
      has to be reachable from there as `summary` and `pending` are
      (`Rc<RefCell<..>>`).
    - Today a `Page` exists only when `self.key` is set: `go()` sets it
      for a key and for a strip click, while a folder open, a refilter and
      a re-listing call `request(Instant::now(), ..)` without a key, and a
      chained `send(next, received, ..)` after a stale result overwrites
      `self.sent`. The "else selection change" rule therefore needs its
      own mark of when the selection last changed, and a way to complete a
      latency point for an unkeyed page without printing a `page` line or
      recording into `Summary`. A stale preview drawn on the way (the
      held-key path, `stale()`) and a failed preview add no point, as in
      the Tauri app.
    - The stages are the native ones (`timing::Stages`: `wait`, `read`,
      `decode`, `orient`, `hop`, `frameWait`, `paint`, `present`), not the
      Tauri seven; see the trade-offs.
    - GPUI drawing facts already established: `PathBuilder::dash_array`
      exists at the pinned rev (`gpui/src/path_builder.rs`), so the dashed
      lines need no hand-built segments; a canvas paints text with
      `window.text_system().shape_line` ("A canvas paints text through the
      window's text system"); the legend's running x needs each shaped
      line's width; `main.rs` already has `MUTED_COLOR` (`0xa1a1a1`),
      `BORDER_COLOR` (`0xffffff1a`) and `PANE_COLOR` (`0x171717`, the same
      value as `--card`).
    - `riffle-backend`'s `perf` module and `sysinfo` are already in
      `crates/native/Cargo.lock`; no new dependency is expected.
    - Verify during implementation: with the overlay off, a held-key run's
      `page` lines look the same as before the change (the off path is one
      branch in the paint closure); with it on, the four lines move every
      500 ms and a bar appears per switch, for a key, a strip click and a
      folder open. If the agent cannot see the window, record the visual
      checks as pending manual checks in `learnings.md`, as earlier
      gpui plans did.
    - `docs/humans/usage.md` describes the Tauri app until the parent
      plan's Step 10 rewrites it; leave it alone.

## Trade-offs and risks

- **The stage names in the bar and the legend.** The Tauri graph stacks
  `wait, read, ipc, post, worker, decode, draw`. The native pipeline has no
  IPC, post or worker; its `page` line has `wait, read, decode, orient, hop,
  frameWait, paint, present`. Decided: stack and list the native stages, so
  the graph matches the native `page` line. The rejected alternative mapped
  onto the Tauri seven (`hop` for `ipc + post + worker`,
  `frameWait + paint + present` for `draw`), which keeps the two apps'
  legends alike but hides `orient` and the vsync wait.
- **Where the latency ends.** The native value includes the vsync wait and
  `present`; the Tauri one stops at `drawImage` ("`keypressToPixels`
  differs from the Tauri app's" in `docs/agents/native-app.md`). The two
  graphs are therefore not comparable bar for bar. Also, the native stages
  do not sum to `keypressToPixels` (the `received -> notified` gap is in
  neither), so a stacked bar is a little shorter than the value the axis
  maximum is taken from. Decided: take the point's value as
  `keypress_to_pixels`, not the sum of the stages.
- **Unkeyed switches.** Recording a folder open, a refilter and a
  re-listing from the selection change follows the Tauri rule and the
  acceptance criteria, at the cost of a few more lines in `main.rs` (a
  mark, and a path that completes a point without a `page` line). The
  cheaper reading is "keyed pages only", which already covers keys and
  strip clicks because the native `go()` stamps both; it would leave the
  folder-open bar out. Decided: record them.
- **The sample route.** Decided: a new `Event` variant on the view's one
  channel, which follows the documented rule and costs an arm in the
  event `match` in `main.rs`. The rejected alternative was a channel
  private to the perf module with its own drain task, which touches
  `main.rs` less but adds a second channel against that rule, and whose
  send fails after a close.
- **No A/B performance gate.** While off, the overlay adds one branch to
  the paint closure. While on, the 500 ms `cx.notify()` redraws the
  window, as the Tauri overlay redraws its canvas; that cost is part of
  what the user sees with the overlay on.
- **`main.rs` conflicts.** Parallel sessions edit `key_down`, the `Event`
  enum, the view's fields and `render`. The step keeps its lines there
  few, but a rebase conflict is likely and must be resolved by hand, not
  by taking one side.
- **The parent plan's link.** The line added to
  `../20261109-gpui-port/plan.md` links to this plan's folder; when the
  wrap-up archives this plan, that link has to become
  `../_archived/20261114-gpui-perf-overlay/plan.md` in the same change, or
  lychee fails.

## Progress
