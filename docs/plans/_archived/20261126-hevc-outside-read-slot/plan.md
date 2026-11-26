<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Run the HEVC decode outside the read slot

## Purpose

`reader::read_preview` and `reader::read_full` end in `hevc::to_jpeg` for a CR3
shot with HDR PQ on (65-125 ms of CPU per file). The scan calls them inside
`ReadSlots::acquire`, so with `READ_SLOTS` at 2 only two workers decode at a
time while the permits sit idle at the disk: a calculated 33-63 ms/file against
3-6 uncapped. The app's preview `load` likewise counts that decode as part of
its read. This plan splits the byte read from the decode so a read slot covers
the read alone, and measures what a folder of HDR PQ CR3 files then costs at
cap 2. It closes the `todo.md` item "Core: split the disk read from
`hevc::to_jpeg` so a read slot covers the read alone", filed by
[the preview read gate plan](../20261107-preview-read-gate/plan.md)
(see its [learnings](../20261107-preview-read-gate/learnings.md),
"Deferred issues").

## Steps

- [x] Step 1: Split the read from the decode in `reader`, use it at the scan's
      read sites and in the app's preview load, measure, and close the todo item
  - Done when:
    - `crates/core/src/reader.rs` has public functions that return the
      not-yet-decoded `(arw, bytes, codec)` (or an equivalent type) for the
      preview and for the full-size image, and a way to finish them into the
      JPEG `read_preview` / `read_full` return today. `read_preview` and
      `read_full` keep their signatures and results, so no other caller
      changes.
    - In `crates/core/src/scan.rs` every `ReadSlot` is dropped before
      `hevc::to_jpeg` runs: `extract_unless`, `extract_analysis_unless`,
      `extract_face_counts_unless` and `full_sharps`. A test in the crate
      shows it: on a `ReadSlots::new(1)` gate, the permit is free at the
      moment the decode of an HDR PQ file starts.
    - In `crates/app/src/main.rs`, `load` reads the bytes first and decodes
      after, and its `read` duration ends before `hevc::to_jpeg`. Its `Paused`
      is still handed to `PaintHold` (Option A, decided). A test shows the
      decode runs after the read phase ends.
    - The measurement is recorded in `docs/agents/shared-crates.md` (the read
      cap section): pass 1's ms/file at cap 2 on a folder of HDR PQ CR3
      files, before and after the split, with the method. If it could not be
      run, `todo.md` keeps one item holding only that measurement.
    - `todo.md` no longer has the item "Core: split the disk read from
      `hevc::to_jpeg` so a read slot covers the read alone" (apart from the
      measurement item if it stays pending).
    - `docs/agents/shared-crates.md` no longer says the cap covers
      `hevc::to_jpeg` "until the read is split from it", and
      `docs/agents/raw-metadata-parsing.md` names the function that now calls
      `hevc::to_jpeg`. `CLAUDE.md`'s layout text is still accurate (changed
      only if it is not).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `reader::read_embedded` already holds the undecoded triple: it calls
      `embedded_from` (with the whole-file retry) and then matches on
      `Codec` to call `hevc::to_jpeg`. Cut it there. The comment at that
      point still applies: a decode error is not a truncated-read error and
      must not trigger the whole-file retry.
    - The finishing half must also do what `neutral` does today
      (`jpeg::neutralize_orientation` on the JPEG handed out), for a decoded
      HEVC image and a JPEG alike. A plain `.jpg` (`read_jpeg`) goes through
      the same new functions as `Codec::Jpeg`, so callers do not branch on
      the file kind.
    - `Codec` lives in `crates/core/src/arw.rs`. Check that it is reachable
      from the public API if the new return type exposes it; a type with a
      `decode`-style method avoids exposing it at all. Choose during
      implementation.
    - Scan sites: keep each site's existing behaviour around the read.
      - `extract_unless` falls back to `read_metadata` under a second
        `acquire` when the preview fails. A decode failure must still reach
        that fallback with the same message.
      - Keep the `#[cfg(test)] tests::cancel_mid_read(cancel)` hooks inside
        the slot.
      - Keep the `yield_to_pause(cancel)` and `canceled(cancel)` checks after
        the read. Whether the decode runs before or after the first
        `yield_to_pause` is open: decoding after it lets a preview's pause
        hold the decode back too, which is what the stage boundaries are for.
    - `full_sharps` only reads when `has_larger_jpeg(arw)`. Check whether an
      HEVC `full` can reach it at all (for an HDR PQ CR3 `full` is the
      `PRVW`). Split it either way so no slot spans a decode.
    - Test for the slot: `crates/core/src/cr3.rs`'s `tests::hdr_pq` fixture
      builds an HDR PQ CR3 whose payload reaches `hevc::to_jpeg` (it fails
      with "HEVC preview without hvcC", which is enough to prove the decode
      was reached). `scan.rs`'s tests already use thread-local hooks
      (`cancel_mid_read`, `MidRead`). A hook of the same kind at the decode
      point that tries `slots.acquire` on a cap 1 gate is one way; a small
      private helper taking the read and the decode as closures is another.
    - App: `load` (`crates/app/src/main.rs`) takes `slots.pause()`, calls
      `reader::read_preview` and stores `read = started.elapsed()`. With the
      split, the HEVC decode's time moves out of `read`. Decide whether it is
      added to `decode` or left unattributed, and update
      `docs/agents/app-log.md` if the `page` line's `read` / `decode`
      meaning changes.
    - Not in scope: `preview_size` in `main.rs`, `crates/app/src/zoom.rs`,
      `crates/engine/src/ondemand.rs` and `crates/cli` call `read_preview` /
      `read_full` outside any slot or pause and stay as they are.
      `READ_SLOTS` stays 2.
    - Measurement:
      - Samples: `D:\Photos\samples\CR3` holds six files containing an
        `hvcC` box: `R8.CR3`, `R5m2.CR3`, `Canon_EOS_R5m2_102A6405.CR3`,
        `Canon_EOS-1D_X_Mark_III_AJKC0638.CR3`,
        `Canon_EOS-1D_X_Mark_III_AJKL0637.CR3` and
        `Canon_EOS_R10_IMG_1205.CR3`. Confirm which are HDR PQ
        (`reader::read_metadata(..).hevc`, or `handpic-cli info`) before
        using them.
      - Folder: build one of a few hundred copies under `D:\Photos\tests\`.
      - Figure: ms/file is the `scan extract:` log line's time over its
        files, from the app (release build, fresh `--index`), where
        `READ_SLOTS` is 2. `handpic-cli scan` runs with uncapped slots, so
        it gives the uncapped figure only, unless it is given a cap for the
        run.
      - Run it on the commit before the change and on the change. Copies of
        one file are served from the OS file cache, which is fine here: the
        cost measured is the decode held in the slot, not the disk.
    - Docs:
      - `docs/agents/shared-crates.md` has the bullet "The cap also covers
        `hevc::to_jpeg` until the read is split from it" (in "The scan's
        reads are capped and yield to the preview's at the disk"), and the
        NVMe paragraph's "HDR PQ CR3 is untested".
      - `docs/agents/raw-metadata-parsing.md` says `reader::embedded_from`
        passes an HEVC `Embedded`'s bytes to `hevc::to_jpeg`.
      - `docs/humans/performance.md` only counts the decodes per file and
        needs no change unless that count changes.

## Trade-offs and risks

- **The app's preview pause: decided, Option A (user approved 2026-11-26).**
  The request said the preview pause should cover only the byte read. But
  `load`'s `Paused` is handed to `PaintHold` (`crates/app/src/hold.rs`) and
  deliberately held until the image is painted, "so pass 2's workers leave the
  cores to the decode, the orient and the paint as well as the disk to the
  read". Option A keeps that hold: the preview's own HEVC decode is exactly the
  work the hold gives the cores to. The app change is only the order and the
  `read` timing. Option B (drop the pause after the byte read, no
  `PaintHold`) would undo the paint hold for every file and was not taken.
- **The measurement needs a local release build of the app.** The auto memory
  note "CI on GHA" says to skip local cargo builds, and the full root
  workspace build has run this host out of memory. If the build is not run,
  the measurement stays in `todo.md` as its own item, with the sample paths
  and method above, and Step 1 is still done.
- **The earlier smoke on this folder panicked.** The preview read gate's
  sweep recorded that a smoke on `D:\photos\samples\CR3` "panicked in the
  probe's decode of a sample". If a sample still panics the decode, leave it
  out of the measurement folder and file it in `todo.md`; do not fix it in
  this step.
- **More concurrent HEVC decodes.** Outside the slot, up to the scan's thread
  count of decodes run at once (22 on this host), each using rayon inside
  `hevc::to_jpeg`. That is the intended gain, but memory and the preview's
  latency during a scan of such a folder are unmeasured. The pass 2 stage
  boundaries (`yield_to_pause`) still hold workers back during a preview;
  pass 1 has none after its read.

## Progress

- (2026-11-27) Step 1 complete
