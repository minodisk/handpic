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

# Report an ongoing burst of folder changes every maximum wait

## Purpose

During `Import…`, with the destination date folder open, the strip stays empty
while the copy runs. The open folder's watcher (`crates/engine/src/watch.rs`)
reports a burst only `DEBOUNCE` (500 ms) after its last event, and an import
renames a file into the folder more often than that, so no report is made
until that folder's copy ends. Seen in `Handpic.log`: the date folder
`2026-08-22` was listed at 14:23:42 with `files=0` and next re-listed at
14:24:10 with `files=100`, after its copy ended (375 files, 6.3 GB, 187 MB/s,
about 33.7 s in all).

The fix adds a maximum wait to the debounce: a burst that goes on is reported
once that time has passed since its first event, and the next event starts a
new burst. The strip then fills while the import runs. `Resync`
(`crates/app/src/resync.rs`) already keeps the more frequent reports from
restarting a scan, so it does not change.

## Steps

- [ ] Step 1: Give the open folder's watcher a maximum wait, with its test, the docs and a real-device check
  - Done when:
    - `watch.rs` holds the maximum wait as a constant (`MAX_WAIT`, about 2 s) next to `DEBOUNCE`, and a burst is reported at the earlier of `DEBOUNCE` after its last event and `MAX_WAIT` after its first event.
    - A unit test in `watch.rs` proves, with synthetic `Instant`s and no sleeping, that an uninterrupted burst (events closer together than `DEBOUNCE`, lasting several `MAX_WAIT`s) is reported once per `MAX_WAIT` interval, that a burst that stops is still reported `DEBOUNCE` after its last event, and that an event after a report starts a new burst.
    - The existing `a_burst_is_collapsed_into_one_event` test still passes unchanged.
    - `docs/agents/app.md` and `CLAUDE.md` describe the new behavior (the places are listed below).
    - The real-device check below was run on this Windows host and its timeline is in the PR body, together with a note that `crates/engine/src/treewatch.rs` has the same trailing-only debounce and is left as it is.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/engine/src/watch.rs`, `docs/agents/app.md`, `CLAUDE.md`. Nothing else; `crates/app/src/resync.rs`, `crates/engine/src/treewatch.rs` and `todo.md` are not touched.
    - `run` today keeps `pending: Option<(String, Instant)>` and overwrites the deadline with `now + DEBOUNCE` on every event. Keep the loop's shape (wait on the deadline, a timeout emits and clears `pending`) and move the timing decision into a pure function that takes the pending burst, the event's folder and `now`, and returns the new pending burst. For the same folder it keeps the burst's first-event time and sets the deadline to `min(now + DEBOUNCE, first + MAX_WAIT)`. For another folder, or with nothing pending, it starts a new burst at `now` (the existing rule: a folder change drops the old folder's pending report). Because a report clears `pending`, the first event after it starts a new burst; no extra state is needed.
    - The test drives that function alone: step a synthetic clock (`Instant::now()` taken once, then `+ Duration`), feed an event every 100 ms, treat `deadline <= now` as the report and clear the pending burst, and assert the report times. No thread, no channel, no real time. Keep it to this; no injectable clock trait.
    - Update the module doc (lines 4-7), the `DEBOUNCE` comment and the `run` doc comment in `watch.rs`, which all say the report comes after the last event.
    - `Resync` was read and needs no change, which the PR body should say. `changed()` returns false while `busy` and only sets the one `pending` flag. `finished()` is called at the scan's end only (`FacesDone` with no pass 3, else `AllFacesDone`, in `main.rs`'s `scanned`), not at pass 1's `Done`. So any number of reports during a listing or a scan give one re-listing after all passes ended, and pass 1 is never restarted mid-scan. An import does not hold `Resync`, so a report with the folder idle re-lists at once. The strip therefore re-lists at the slower of `MAX_WAIT` and the listing plus scan of the files that landed since the last one.
    - `docs/agents/app.md`, three places:
      - "A folder change re-lists, never mid-scan" (around line 1854): state the report rule (after `DEBOUNCE` of quiet or `MAX_WAIT` into a burst that goes on), that during an import into the open folder the strip fills at the slower of the two cadences above, and reword the rule about a file still being copied in (it is now re-extracted at every report of the burst, at least every `MAX_WAIT`).
      - The paragraph on `wait_for_idle` after a confirmed import (around line 1638): the copies now appear during the run; check on the device whether the race at the run's end (the last files inside the debounce, `import_ran` re-listing only on `resync.changed()`) is still as described and keep, reword or drop the sentence accordingly.
      - The `Import…` rules (around line 5556 to 5593): add a `Seen (<date>, Windows 11)` line with the device check's result.
    - `CLAUDE.md`: the `src/watch.rs` clause says it "reports a settled burst through a callback". Reword it so it also covers a burst that goes on (reported every `MAX_WAIT`). Change that clause only.
    - Real-device check (Windows host, one run, no generated load and no repeated loop):
      - Build a debug app with the screenshot feature in `crates/app` (`cargo build --features screenshot`, as `mise run app:dev:screenshot` does; the `todo.md` device checks put NASM on `PATH` for this build).
      - Material: about 125 RAW files (about 6 GB) from `D:\Photos\tests`, placed in a temp folder of the run's own. Never write under `D:\Photos\tests`, never open a folder under `D:\Photos\` outside it. Hard links need a folder on the same volume outside `D:\Photos\tests`; copies can go anywhere. Check the free space of the destination volume first (`D:` is nearly full and slow). Files of one capture date keep the whole run in the folder that opens.
      - File cache: hard links share the cache with the originals, and a fresh copy warms it. Do not try to reach a given throughput. The run only has to last several `MAX_WAIT`s; read its length from the log's `import:` line, and if it is under about 10 s, use more files or a slower destination rather than any load.
      - Launch with `--index`, `--settings`, `--log` and `--import-settings` all in the run's temp folder and `--mcp-port 47830`, seeding `mcpEnabled`, `mcpAllowDestructive` and `importRoot` (a temp folder) through `--import-settings`. Never the user's index, settings or photo folders. Follow `docs/agents/app.md`'s rules for MCP test runs (the client written to a file, the body on stdin, stopping the instance by the PID whose command line names the temp folder).
      - Start the import with MCP `import_folder` (preview, then the confirming call). While it runs, call `get_view` and `get_status` about once a second and take a few `screenshot`s. Record the listed file count against time and the import's done / total.
      - Pass: the count rises in several steps before the import ends, and the log shows `resync:` lines (some `deferred`) during the run, not only after it. Also record the `import:` line's throughput, since the scan now runs beside the copy.
      - Put the timeline, the log excerpt and a screenshot mid-run in the PR body. Delete the temp folders afterwards.

## Trade-offs and risks

- Local build against the host's memory limits: the device check needs a local debug build of `crates/app`, while the user's standing preference is to leave builds and `ci:local` to GitHub Actions. The plan keeps one local build for the check (approved by the user on 2026-11-28).
- Hard links or copies for the test material: hard links cost no space but need a new folder on `D:` and raise the originals' link count (metadata only); copies touch nothing on `D:` but need about 6 GB elsewhere and warm the cache. Either way the throughput will not match the report's 187 MB/s, which the check accepts.
- The fill is bounded by the scan, not only by `MAX_WAIT`: a report during a scan waits for the end of passes 1 to 3 on the files already landed. On a debug build pass 2 has measured about 0.3 s per file, so the strip may fill in steps longer than 2 s. Re-listing at pass 1's `Done` would be finer but restarts pass 2 over and over, which the existing rule in `docs/agents/app.md` rules out; it is not done here.
- The scan now runs while the copy does, where before it ran after each folder's copy. Both read the same disk, so the import may slow down. The check records the throughput; there is no mitigation in this plan.
- A large file written in place by another program for longer than `MAX_WAIT` is now listed mid-copy at every report and may show a failed cell until the next one. The report `DEBOUNCE` after the last event still comes, so the end state is right.
- Within a burst the reports are `MAX_WAIT` plus the gap to the next event apart, since a new burst starts at the first event after a report.
- `crates/engine/src/treewatch.rs` debounces the same way, so a folder's count in the tree likely stalls during an import too. Out of scope; the PR body mentions it.
- `todo.md`'s "App: real-device checks of Import… (gpui-folder-dialogs Step 5)" section is left untouched, although one of its boxes covers the strip filling during a run.

## Progress
