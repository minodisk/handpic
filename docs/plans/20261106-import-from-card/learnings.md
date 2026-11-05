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
