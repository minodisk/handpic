# Learnings

## Step 1: Import settings

- chrono's `StrftimeItems` yields `Item::Error` for an unknown specifier or a
  lone `%`, and `NaiveDate::format` then fails its `Display` with
  `fmt::Error` for any specifier needing a time, an offset or a timestamp
  (`%H %M %S %p %I %f %z %Z %s %T %R %c %+`). `.to_string()` on that panics,
  so `format_date` writes it with `write!` and maps the error.
- `%n` and `%t` are valid chrono specifiers that emit a newline and a tab;
  the control-character check on each folder name is what refuses them.
- `validate_pattern` formats a fixed date (2026-01-02), and `format_date`
  runs the same folder-name checks on every real date, so a pattern whose
  validity depends on the date (none known among the date specifiers) would
  still be refused at the date that breaks it.
- The stored `importPattern` is re-validated at load (`pattern_setting`); an
  invalid stored value falls back to `%Y/%F` rather than reaching the run.
- `import_pattern_example` joins the formatted path onto the stored root in
  Rust (`PathBuf::extend` over the `/` segments), so the UI shows the OS
  separator without composing paths in TypeScript; with no root set it
  shows the relative path alone and the tab shows the "root not set" note.
- `commands::settings` became `pub(crate)` so `import.rs` reads and writes
  the store itself; `load_settings`'s tuple was left alone.
- The tab icon is Lucide `hard-drive-download` (lucide-static v1.48.0, not
  Feather-derived), added to `icons.ts`' license header.

## Step 2: The copy pipeline and `copybench`

- `copy::copy_files` runs the writer on the calling thread inside
  `std::thread::scope` and spawns only the reader, so `progress` is a plain
  `FnMut` without `Send`. Read buffers are recycled through a return channel;
  at most `buffers + 2` chunks are alive.
- "Never overwrite" is a check of `dst.exists()` in the reader before reading
  and again just before the rename (`fs::rename` replaces on every platform).
  A file created between that check and the rename would still be replaced;
  a no-replace rename needs per-OS calls (`MoveFileExW` without
  `MOVEFILE_REPLACE_EXISTING`, `renameat2`, `renamex_np`) and was left out.
- A failed write tells the reader to stop reading that file through an
  `AtomicUsize` holding the job index + 1; a failed job counts its source
  size as done so the progress bytes still reach the total.
- `TEMP_SUFFIX` now lives in `riffle_core::copy`; `sidecar.rs` and `watch.rs`
  import it from there.
- Measurement (2026-11-06) on the user's machine: Sony ARWs of about 44 MB
  from `E:\DCIM\10261103` (a USB card reader, exFAT) to
  `D:\Photos\tests\20261106-copybench\` (Crucial BX500 4 TB SATA SSD, NTFS,
  96 % full). Each run used 50 files (~2.2 GB) that had not been read
  before, so the card was read cold, with 60 s idle between runs. The copies
  were deleted afterwards.

  | Run | Method | MB/s |
  | --- | --- | --- |
  | A | pipeline, 8 MiB × 4 | 21.6 |
  | B | `robocopy /J` | 14.4 |
  | C | `robocopy` (buffered) | 12.1 |
  | D | pipeline, 32 MiB × 2, `FILE_FLAG_SEQUENTIAL_SCAN` | 12.9 |
  | E | pipeline, 8 MiB × 4, no `sync_all` | 11.1 |
  | F | pipeline, 8 MiB × 4, `FILE_FLAG_SEQUENTIAL_SCAN` | 9.6 |
  | – | read only, `dd bs=8M` to `/dev/null` | 134.7 |
  | G | pipeline, 8 MiB × 4 (A repeated) | 8.1 |

  The throughput fell steadily from run to run (A 21.6, then G 8.1 with the same
  settings), whatever the method, while the card read alone ran at 135 MB/s.
  The bottleneck is the destination: the BX500 (DRAM-less, nearly
  full) loses its write cache under sustained writes. An earlier 4.5 GB run
  reached 45 MB/s cold and only 9 MB/s when repeated right after. So these
  numbers cannot tell the methods apart. The pipeline is not slower than
  robocopy here. `FILE_FLAG_SEQUENTIAL_SCAN` showed no gain, so it was not
  kept, and dropping `sync_all` showed no gain, so the per-file flush stays.
  The defaults stay at 8 MiB × 4. A fair comparison with Explorer needs a
  destination faster than the card, such as the NVMe C: drive or an emptier
  SSD.
- Read-only timings taken while a copy to D: was still running in the
  background came out at 16–22 MB/s. Do not time the card while D: is busy.

## Step 3: The import plan and `import_preview`

- The planner's per-file type is `Planned`, not `Item`: `import.rs`
  already imports `chrono::format::Item` for the pattern check.
- `walk` attaches sidecars from the folder's own listing (a case-insensitive
  match of `SidecarFormat::sidecar_path`'s name among the entries
  `SidecarFormat::Both.matches`), not through `sidecar::existing_sidecar`,
  which re-reads the folder for every RAW without a sidecar of the exact
  case. A sidecar goes to the first RAW (by path) claiming it, so
  `FOO.xmp` next to `FOO.ARW` and `FOO.NEF` is not copied twice. A
  JPEG takes no sidecar.
- `walk` follows symlinked folders (`folders::kind`, now `pub(crate)`)
  and keeps a set of canonical folders it has read, so a link loop ends.
  An unreadable subfolder fails the whole preview rather than silently
  leaving its photos out.
- Two card files with one name on one date (Sony numbering wraps across
  `100MSDCF` / `101MSDCF`) are a collision for the second one: the
  planner keeps the lowercased names already planned per destination,
  since Windows and macOS compare names case-insensitively.
- Sidecar rules as implemented: next to a colliding RAW the sidecar is a
  collision too (and listed); next to a skipped RAW it is copied when
  missing and skipped when anything by that name is there, whatever its
  size (Riffle has likely rewritten it since the first import); next to a
  copied RAW it is decided on its own (missing copy, same size skip,
  other size collision).
- Several dates the pattern maps to one folder (`%Y/%m`) merge into one
  destination, its items in date then file-name order.
- `date_of` accepts day 31 of any month; `NaiveDate::parse_from_str` then
  refuses an impossible date such as `2026:02:31`, which lands in undated.
- A refused preview (root unset, pattern invalid) returns `id: None` and
  the reason without walking the folder. Counts in the preview include the
  sidecars (`copyCount` is files, not photos), matching what the run's
  progress will count.

## Step 4: The run, the dialog and `Import…`

- `import_run(preview_id)` clones the stored plan rather than taking it, so
  a refused second run (one at a time) does not lose the preview; running the
  same preview again just finds every file at its destination.
- The stored preview now also keeps the source folder and the root
  (`Previewed`), which the `import:` log line names; `import_preview`'s walk
  and plan moved into `plan_dir`, which the tests call directly.
- The copy pipeline re-checks each destination before reading and again
  before the rename, failing the job with `exists: ...`. `run_plan` turns a
  failed job whose destination now exists with the planned size into a skip
  (imported meanwhile), so it does not have to match the message text; any
  other failure, a different-size file there included, stays a failure and
  the existing file is untouched.
- Destination folders are created only for those with a file to copy, all
  before the copy begins (Step 5 opens the first one at run start); a folder
  that cannot be created fails its files with the OS error and the run goes
  on with the others. `import-done`'s `folders` lists those created.
- The `import:` line: `import: files=N bytes=N mb_per_s=X skipped=N
  failed=N canceled=B source=... root=... in Nms` (MB = 10^6 bytes, as in
  the dialog's sizes).
- The payloads are camelCase (`runId`, `filesDone`, ...) like Step 3's
  preview, unlike `sequence-*`'s snake_case; `ImportFlow` follows
  `SequenceFlow` minus its vestigial `picking` phase (`start(dir)` goes
  straight to previewing) and keeps the preview id for `run()`. A refused
  preview (`id: null`) still opens the dialog with the reason and Run
  disabled.
- `Import…` is its own last group in the folder menu, offered on roots.
  `datefolder.ts`'s `Rename by Capture Date` was merged after the plan was
  written; its start guard and every other flow's now include
  `importFlow.busy`, and `modalOpen` / the keydown routing `importFlow.isOpen`.
- The re-import criterion is covered by
  `a_run_copies_the_plan_and_a_second_import_copies_nothing` on temp dirs
  with Exif-only JPEGs (the sequence tests' builder, padded to a size).

## Step 5: Cull while importing

- The first date folder did not exist when the frontend learned of the run:
  Step 4's `run_plan` created the folders on the `spawn_blocking` task, after
  `import_run` had returned the run id. The run is now split into
  `prepare` (creates every destination folder and lines up the jobs) and
  `run_plan` (copies); `prepare` runs synchronously inside `begin`'s spawn
  closure (after the one-run-at-a-time check, so a refused run creates
  nothing), and `import_run` returns `{ runId, folders }`, the created
  folders in date order. `main.ts` opens `folders[0]` through
  `openDirectory` with a fresh folder token, always, even over another open
  folder; `openDirectory` already calls `folders.reveal`.
- Step 4 kept the import dialog modal for the whole run, which would have
  made culling during the copy impossible (every key went to
  `importKeydown`, the backdrop took the mouse). `ImportFlow.started` now
  puts the dialog aside (`isOpen` false while running) unless a cancel was
  asked for before the run id came; the status line keeps
  `importing N / M`, and the tree's `Import…` while a run is under way
  brings the dialog back (`reopen`) with Cancel. This design call was made
  in this step and confirmed by the user afterwards (2026-11-06, over a
  cancel button in the status line or keeping the dialog modal); Step 6's
  docs must describe it.
  The other flows (Move to Trash, Settings, Sequence, Rename by Capture
  Date, the sidecar items) still refuse while `importFlow.busy`; flagging,
  rating and labels work. The folder and file renames refuse too (checked
  in `holdRename` at confirm time and again when a held rename drains),
  since `copy.rs` does not recreate a folder renamed mid-run; reported in
  review round 1. `finishImport` decides `importFlow.done` at once but
  reports (errors, status, resync check) after the import's own
  `openDirectory` settles, whose `errors.clear()` would otherwise wipe the
  failures. `import_run` runs `prepare` on the blocking task and gets the
  folders back over a channel.
- Code reading of the growing-folder path, no fix needed:
  - Empty folder: `openDirectory` with `files.length === 0` leaves
    `meta = null`, `selection = single(undefined)`; the next `resync`'s
    `refilter(undefined, true, false, previous)` sees the list changed,
    `anchorAfterFilter` returns `undefined` for an `undefined` anchor so
    `index = 0`, and `mustReshow(false, files[0], undefined)` is true, so
    `show()` runs. Covered by a `resume.test.ts` case over the same
    helpers, plus one for files landing after the current one (it stays,
    no reshow).
  - Temp files: the copy writes `<name>.riffle-tmp` and sets the mtime
    before the rename, so only the rename's event names a listable file;
    `watch::triggers` drops events whose paths are all temp files or
    sidecars (new test `an_import_temp_file_triggers_only_once_renamed`).
    The import's `.xmp` / `.dop` copies are dropped too; they land right
    after their RAW, inside the RAW's 500 ms debounce window.
  - Cadence: a `folder-changed` during a scan is deferred to `faces-done`
    and drained as one rescan; `DEBOUNCE` collapses a burst into one event.
    The embed pass is not "scanning", so a drained or new rescan cancels it
    (`scan_folder` joins it) and the next cycle's third pass resumes from
    the stored embeddings: during a long import with `aiEnabled` on the
    embeddings mostly fill after the import, the strip is never held back
    by them.
  - Tree count: `reveal` expands every node down to the date folder itself,
    so the date folder is in `watchedFolders` and its `tree-changed`
    re-lists it with the new `raw_count`.
  - Another date folder: the open folder's watch is non-recursive on that
    folder alone, and `folder-changed` is dropped unless its `dir` is
    `openDir`, so writes into a sibling date folder leave the strip alone
    (the expanded parent may re-list its children in the tree, no change).
- One gap found and closed: `list_arw` runs before `scan_folder`'s
  `watch::set`, so a file renamed into the folder in between is scanned but
  not in `allFiles` until another trigger. During an import the next file
  covers it; for the last files, `finishImport` now runs
  `resync("import")` (a new `RescanTrigger`) when the open folder is one
  of the run's folders.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, the settings modal's `Import` tab): on
  Windows (and macOS if available) open Settings > Import, press `Choose…`
  and pick a folder; expect the root field to show it and the example line
  to read `<root>\<YYYY>\<YYYY-MM-DD>` for today. Type `%Y/%H` and expect the
  error in red under the input; blur and expect the stored pattern kept
  (reopen the app: the field shows the last valid pattern). Click each
  example button (`%Y/%F`, `%F`, `%Y/%m/%d`, `%Y/%m`) and expect the input,
  the example and, after a restart, the stored value to follow. The step
  was ticked on the automated criteria (unit tests, `mise run ci`); this
  GUI check was not run here. Related files: `crates/app/ui/src/settings.ts`,
  `crates/app/ui/index.html`, `crates/app/src/import.rs`.
- Copy throughput against Explorer is unmeasured on a fast destination
  (Step 2 measurement). The only destination tried, `D:` (BX500, 96 % full),
  capped every method at 8–22 MB/s while the card reads at 135 MB/s. Re-run
  `riffle-cli copybench <card-dir> <dst-dir>` and an Explorer copy of the
  same fresh files into a destination on a faster or emptier drive to settle
  whether the 8 MiB × 4 defaults and the per-file `sync_all` are right.
  Related files: `crates/core/src/copy.rs`, `crates/cli/src/main.rs`.
- The no-overwrite guarantee of `copy::copy_files` has a race between the
  `dst.exists()` check and `fs::rename`, which replaces the destination
  (Step 2 implementation). A file created at the destination in that window,
  such as a sidecar the app writes for a RAW that just landed, would be
  replaced. A per-OS no-replace rename would close it. Related file:
  `crates/core/src/copy.rs`.
- Pending manual check (Step 4, the `Import…` flow in the running app): on
  Windows with a memory card (or any folder of RAWs / JPEGs) and the
  destination root set in Settings > Import, right-click the card's root in
  the folder tree and choose `Import…`. Expect the dialog to list the source,
  one row per date folder with its file count, size and skipped count, the
  colliding and undated names in their own lists, and Run enabled. Run it:
  the status line shows `importing N / M`, Cancel (or Escape) stops before
  the next file and the dialog closes with `Import canceled, ...`; an
  uncanceled run ends with `Imported N files (X MB)...`, the date folders
  hold the copies with the card's modified times and no `.riffle-tmp` file,
  and `Riffle.log` has one `import:` line. Choose `Import…` again on the
  same card: expect the note "Nothing left to copy" and Run disabled. With
  the root unset, expect the refusal in the dialog and Run disabled. Also
  check that the other folder-menu dialogs (Sequence, Rename by Capture
  Date, the sidecar items) and Settings do not open while the import dialog
  is up. The step was ticked on the automated criteria (unit tests,
  `mise run ci`); this GUI check was not run here. Related files:
  `crates/app/src/import.rs`, `crates/app/ui/src/import.ts`,
  `crates/app/ui/src/main.ts`, `crates/app/ui/index.html`.
- Pending manual check (Step 5, cull while importing): on Windows with a
  memory card in the reader (`E:\DCIM\...`, never written to) and Settings >
  Import's root set to a test root such as
  `D:\Photos\tests\20261106-import-step5\` (delete it afterwards), open any
  other folder first, then right-click the card's root, choose `Import…`
  and Run. Expect: the dialog closes at once, the earliest date folder opens
  (empty at first) and is revealed and selected in the tree with its
  parents expanded; the first file appears in the strip and in the preview
  about a second after it lands; the strip keeps growing in batches while
  the status line shows `importing N / M`, the current file stays current
  and flags / ratings set meanwhile stay; no `*.riffle-tmp` ever shows in
  the strip and `Riffle.log` (with Timing logs on) shows `rescan:` lines
  about once per scan cycle, not one per file; with 500+ files the arrow
  keys and the preview stay responsive; the date folder's count badge in
  the tree grows; while files land in the second date folder the strip
  does not change; `Import…` again during the run brings the dialog back
  and Cancel stops it. With `aiEnabled` on, expect the embeddings to fill
  mostly after the import ends, not the strip to stall. The step was ticked
  on the automated criteria (unit tests, `mise run ci`); this check was not
  run here. Related files: `crates/app/ui/src/main.ts`,
  `crates/app/ui/src/import.ts`, `crates/app/src/import.rs`,
  `crates/app/src/watch.rs`.
