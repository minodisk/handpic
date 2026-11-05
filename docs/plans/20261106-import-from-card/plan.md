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

# Import from a memory card into date folders

## Purpose

The culling workflow starts with a card in the reader: `E:\DCIM\10061024\...`
(Sony names) has to land in `D:\Photos\2026\2026-10-24\` before Riffle is of
any use, and today that is an Explorer copy followed by a rename by hand.
The folder tree gets a right-click item, `Import…`, on any folder (a volume
root such as the card itself included), which previews and then copies every
supported photo under it, recursively, into `<destination root>/<pattern>`
folders named after each file's capture date, skipping what is already there,
and opens the first date folder so culling can start while the copy is still
running.

Decided design (from the conversation; the "Trade-offs and risks" section
lists what is still open):

- **Files imported**: the RAW formats Riffle supports (`riffle_core::scan::is_raw_file`: ARW, CR3, NEF, RAF, ORF, DNG) and JPEG (`is_jpeg_file`), plus an existing `.xmp` / `.dop` sidecar next to a RAW (`SidecarFormat::{Xmp,Dop}.matches`). Videos and everything else are out of scope. The card is never modified.
- **Grouping**: by the Exif `DateTimeOriginal`'s local date (`datefolder::date_of`, no time-zone handling). Undated files are listed in the preview and skipped.
- **Destination**: `<root>/<pattern>`, the pattern in chrono strftime syntax, default `%Y/%F`; `/` separates folders. Date-only: time, zone and timestamp specifiers are rejected.
- **Existing destination**: add only what is missing. Same name and same size there = already imported, skipped. Same name, different size = collision, listed, skipped, never overwritten.
- **Partial files**: each file is written to a temp name in its destination folder and renamed on completion, so the watcher and the scan never see a partial file. The source's modified time is set on the copy.
- **Speed**: one sequential reader thread on the card, large buffers, a writer thread overlapping the writes; the throughput is logged (and measurable from the CLI) so it can be compared with Explorer. No speedup is promised.
- **Cull while importing**: once the run starts, the first (earliest) date folder is opened (always, even when another folder was open; confirmed by the user), and the strip fills as files land, through the existing watcher → resync path.

## Steps

- [x] Step 1: Import settings: destination root and folder pattern, validated, persisted, with live help in the settings modal
  - Done when:
    - `crates/app/src/import.rs` (new module, registered in `main.rs`) holds the pattern logic: `validate_pattern(pattern) -> Result<(), String>` and `format_date(pattern, date: NaiveDate) -> Result<String, String>` (the formatted relative path, `/` already the separator); the settings' parsers in the style of `label_names_setting` (`importRoot` string, `importPattern` string defaulting to `%Y/%F`); and the commands `import_settings` (both values), `set_import_root(path)`, `set_import_pattern(pattern)` (rejects an invalid pattern with the reason and keeps the stored one) and `import_pattern_example(pattern)` (the live example for the UI, or the reason).
    - A pattern is rejected when (in this order): `chrono::format::StrftimeItems` yields any `Item::Error`; formatting a `NaiveDate` through `write!` (never `.to_string()`, which panics) fails, which is how `%H %M %S %p %z %Z %s %T %R %c %+ ...` are refused; the output has an empty segment, a `.` or `..` segment, a segment with a character invalid on Windows or macOS (`< > : " | ? * \`, control characters) or ending in a space or a dot. Unit tests cover the accepted set (`%Y %y %m %d %b %B %a %A %j %F` and literal text) and every rejection class.
    - The settings modal has a new `Import` tab (`index.html`, `settings.ts`, `style.css` only where a new shared class is needed): a destination root field with a `Choose…` button that opens the existing `pick_folder` command and saves; a pattern input that saves on change; under it the live example (`<root>\<formatted today>` with the OS separator, e.g. `D:\Photos\2026\2026-10-24`) or the validation error in `--destructive`; a short table of the allowed specifiers (`%Y %y %m %d %b %B %a %A %j %F`, `/` = folder separator); and clickable examples `%Y/%F`, `%F`, `%Y/%m/%d`, `%Y/%m` that fill the input and save.
    - `mise run ci` passes.
  - Implementation approach:
    - Add `chrono = "0.4"` to `crates/app/Cargo.toml` (it is only in `riffle-core` today).
    - Persist through `commands::settings(app)` and load at startup where `labelNames` / `aiEnabled` are read (`commands.rs` ~l.700, the `main.rs` `.manage(...)` block), keeping the in-memory copy in a managed `AppImportSettings(Mutex<...>)` so the run never re-reads the store.
    - Validation lives in Rust; the frontend calls `import_pattern_example` on input (debounced) and renders the result. Keep pure text helpers (specifier table rows, example list, the "root not set" note) in a new `crates/app/ui/src/importsettings.ts` with a `.test.ts`, following `settings.ts`'s pattern of small exported helpers.
    - The example date is today; the unit tests pass a fixed `NaiveDate`.
    - Follow `docs/agents/ui-styling.md`: reuse `.field`, `.input`, `.button outline`, `.dialog-actions`; the tab button is another `.sidenav-item` with an icon from `icons.ts`.

- [x] Step 2: The copy pipeline in `riffle-core` and a CLI benchmark of it
  - Done when:
    - `crates/core/src/copy.rs` (new) exposes `copy_files(jobs: &[Job { src, dst }], cancel: &AtomicBool, progress: impl FnMut(Progress)) -> Summary` that copies each job with one reader thread (sequential over the jobs, large chunks) feeding a bounded channel of buffers to one writer thread; each destination is written to `<dst file name><TEMP_SUFFIX>` in the destination folder, flushed, renamed to the final name, and given the source's modified time (`File::set_modified`); a destination that exists at write time is never overwritten (the job fails with a message); a cancel stops before the next file and removes the temp file of the file in flight; `Summary` holds per-job results, bytes copied, elapsed time and whether it was canceled; `progress` reports files done / total and bytes done / total.
    - The temp suffix is the app's `.riffle-tmp` (`sidecar::TEMP_SUFFIX` moves to, or is mirrored by, a `riffle_core` constant the app's `sidecar.rs` and `watch.rs` use), so the open folder's watcher filters the temp file's events and `read_listing` never lists it (its extension is not a RAW's).
    - `riffle-cli copybench <src-dir> <dst-dir> [chunk-mib] [buffers]` copies every RAW/JPEG under `src-dir` (recursive) flat into `dst-dir` through the pipeline and prints files, bytes, seconds and MB/s, so the user can time it against Explorer on the real card.
    - Unit tests on temp dirs: content and mtime preserved, no temp file left after a run, an existing destination is not overwritten and is reported, cancel leaves no partial file, progress reaches the total.
    - `mise run ci` passes.
  - Implementation approach:
    - Measure before fixing the chunk size and the number of buffers in flight; start from 8 MiB chunks and 4 buffers and record the numbers for the user's card in `learnings.md`. On Windows, try `OpenOptionsExt::custom_flags(FILE_FLAG_SEQUENTIAL_SCAN)` on the source and record whether it helps; keep it only if it does.
    - `sequence.rs` in core is the model for the run / cancel / progress shape (`riffle_core::sequence::run(dir, dry_run, cancel, progress)`).
    - Do not touch the index here: the pipeline is pure file copying.

- [ ] Step 3: The import plan and the `import_preview` command
  - Done when:
    - `import.rs` gains the pure planner: `walk(dir)` lists every RAW / JPEG under `dir` recursively (following `folders::kind`'s symlink rule, skipping hidden folders as `folders::list` does) with each RAW's existing `.xmp` / `.dop` sidecars; `plan(files_with_dates, root, pattern, probe)` groups by date through `datefolder::date_of` + `format_date`, and decides per file: `copy`, `skip` (same name and size already at the destination), `collision` (same name, different size) or `undated`; sidecars follow their RAW's decision (a sidecar whose RAW is skipped is itself skipped unless missing at the destination).
    - `import_preview(dir)` reads each file's `DateTimeOriginal` from its head (`riffle_core::reader::read_metadata`, as `date_folder_preview` does when there is no index row), builds the plan, stores it in managed state under a preview id, and returns to the dialog: the preview id, the source folder, per destination folder (in date order) its path, files to copy, bytes to copy, skipped count and the colliding file names, the undated file names, totals, and a refusal reason when the root is not set or the pattern is invalid.
    - Unit tests on temp dirs for `walk` (recursion, extensions, sidecars, nothing else) and `plan` (grouping, skip by same size, collision by different size, undated, pattern applied, JPEG next to a RAW counted as its own file).
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 1 (settings) is merged; Step 2 is not needed for the preview.
    - `datefolder::plan` is not reused: it is tied to one folder's siblings and rename-collision semantics; only `date_of` is shared.
    - The preview reads every file's head on the card once; the run (Step 4) takes the stored plan by id rather than re-reading, re-checking each destination just before writing. Keep the stored plan in a `Mutex<Option<(id, Plan)>>` in the managed state, replaced by the next preview.
    - Nothing on disk changes in this step.

- [ ] Step 4: The run: `import_run` / `import_cancel`, progress events, the dialog and the tree's `Import…` item
  - Done when:
    - `import_run(preview_id)` creates the destination folders, runs the Step 2 pipeline on the plan's `copy` jobs (RAW first, then its sidecars, in date then name order) on a `spawn_blocking` task, streams `import-progress` (run id, files done / total, bytes done / total, the file in flight) and ends with one `import-done` (run id, copied, skipped, failed with messages, canceled, bytes, elapsed ms, the destination folders in order); `import_cancel(run_id)` stops before the next file; one run at a time (`sequence.rs`'s `begin` / `finish` shape in a managed `Imports` state).
    - After every run `Riffle.log` has one `import:` line with files, bytes, elapsed and MB/s, plus the source and root, so the throughput can be compared with Explorer.
    - `crates/app/ui/src/import.ts` (+ `.test.ts`) holds the dialog text and an `ImportFlow` state machine modeled on `SequenceFlow` (early `import-done` kept until `started`, cancel passed on); `index.html` gets an `import-dialog` in the `sequence-dialog` shape: the source, per destination folder a row with the path, the count and bytes to copy and the skipped count, the collision names and the undated names in their own lists, a note when nothing is left to copy, the running note, `Run` / `Cancel`; while running, the status line shows `importing N / M`.
    - `context.ts`'s `folderMenuGroups` gets `Import…` (offered on roots too, since a card is a volume root); `main.ts` wires it like `sequenceTimestampsOf`; `modalOpen` and the other flows' `start` guards include the import flow.
    - Re-running `Import…` on the same card previews zero files to copy and the run copies nothing.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Steps 1–3 are merged.
    - The run does not touch the index and does not wait for a scan (as `sequence.rs` does not), so `idle.ts` is not involved.
    - A collision or an undated file is informational: the run button is enabled whenever at least one file is to be copied.
    - Errors join the sticky `errors` list keyed by path, as the sequence flow's do.

- [ ] Step 5: Cull while importing: open the first date folder at run start and let the strip fill
  - Done when:
    - When the run starts, `main.ts` opens the first destination folder (the earliest date; created empty by `import_run` before the copy begins) through `openDirectory`, and the folder tree reveals it (`folders.reveal`, which the existing open path already does).
    - As files are renamed into the open folder, the existing watcher → `folder-changed` → `resync("watch")` path lists them and scans only the new ones: the strip grows without a full reload, the current file stays current, the judgments made so far stay. The folder's node in the tree updates its count (`tree-changed` on the expanded parent through `treewatch`).
    - Verified by hand on a real card and recorded in `learnings.md`: a folder opened while still empty shows its first file once it lands (check `openDirectory`'s `files.length === 0` path followed by `resync`'s `refilter` — fix if the first file does not appear or is not shown); the temp file never appears in the strip nor triggers a rescan storm (the `.riffle-tmp` events are dropped by `watch::triggers`); the rescan cadence during a 500+ file import keeps the UI responsive (each `scan_folder` after `faces-done`, 500 ms debounce); and when the import writes into a date folder other than the open one nothing in the strip changes.
    - `mise run ci` passes.
  - Implementation approach:
    - Assumes Step 4 is merged. The folder tree's `tree-changed` watches only expanded folders; if the destination's parent is not expanded, the count refresh is left to the reveal, which expands the path.
    - If the hand test shows a problem, the fix goes here (likely candidates: `resync` while `files.length === 0` not calling `show()`; a rescan deferred behind the AI embed pass; the tree count not refreshed). Record what was and was not needed.
    - Do not add a special-cased "import mode" to the scan; the goal is that the general growing-folder path works.

- [ ] Step 6: Documentation
  - Done when:
    - `README.md` / `README.ja.md` mention `Import…` in the feature list and the quick-start; `docs/humans/usage.md` / `usage.ja.md` get an `Import…` entry under the folder-tree operations (settings, the pattern and its specifiers, skip and collision rules, partial-file safety, cull while importing, the throughput log line) and the settings section lists the `Import` tab; `docs/humans/performance.md` / `.ja.md` get a short note on how to compare the copy with Explorer (`riffle-cli copybench`, the `import:` log line) if that document is where measurements go; `CLAUDE.md`'s layout paragraph names `crates/app/src/import.rs`, `crates/core/src/copy.rs`, `ui/src/import.ts`, `ui/src/importsettings.ts` and the `importRoot` / `importPattern` keys.
    - `mise run ci` (lychee included) passes.

## Trade-offs and risks

- **Where the copy pipeline lives**: `riffle-core` (chosen, so the CLI benchmark runs the same code the app runs) vs `crates/app` (keeps core free of a generic file copier, but the CLI could not reuse it).
- **"Already imported" by name and size only**: fast (one `stat` per file), but a same-size edit in place (a sidecar rewrite, a retouched JPEG of the same length) is treated as imported. A hash would be exact but reads the whole card again, defeating the skip. Chosen: name + size; mention in the docs.
- **Preview reads the card once; the run reuses the stored plan**: if the card changes between preview and run, the run re-checks each destination (skip if it appeared) and reports a source that vanished as a failure. The alternative (re-plan at run start) reads every file head again on a slow card.
- **Opening the first date folder on run start**: always, per the user's choice. Side effects: a folder being culled is left, and its scan is canceled by the open.
- **The AI embed pass and the rescan cadence**: `resync` defers only while `scanning()` is true, and the embed pass is not counted as scanning, so a rescan during the third pass cancels and restarts it per batch of new files (`scan_folder` joins it). On a long import with `aiEnabled` on, embeddings of the folder finish only after the import; acceptable, but verify in Step 5 that it does not starve the strip.
- **Temp suffix shared with the sidecar writer**: reusing `.riffle-tmp` gets the watcher filter for free; a reader of `watch::triggers` must know the import's temp files share it.
- **Pattern path validity**: Windows reserved names (`CON`, `NUL`, ...) and NTFS/HFS length limits are not checked; a pattern producing one fails at folder creation with the OS error, which the dialog shows.
- **JPEG next to a RAW on the card**: both are copied; the destination folder's strip lists only the RAWs (`folders::Media`'s rule), which is the existing behavior for a RAW+JPEG folder.
- **Explorer comparison**: Windows' copy already overlaps reads and writes and uses the cache; the pipeline may only match it. The plan commits to measuring (Step 2 CLI, Step 4 log line), not to a speedup.
- **Scope kept out**: deleting from the card after import, videos, a "move" mode, a per-run undo (the copies can be moved to the Trash with the existing strip item).

## Progress

- (2026-11-06) Step 1 complete
