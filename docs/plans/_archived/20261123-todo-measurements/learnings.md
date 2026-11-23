# Learnings

## Step 1

Host: Windows 11 Home 10.0.26200, 24 logical cores, C: the NVMe
`CT1000P5PSSD8` (about 80 % full, 189 GB free at the start), D: the SATA SSD
`CT4000BX500SSD1` (88 % full). Base commit `3918064c` (`main` after #1244).
Other agent sessions were building and testing in other worktrees on the
same host during the whole run, and one of them had a debug Handpic
(`worktree-silver-cloud-a8cf`) open; they were not this session's to close,
so the timing runs below carry that background load (each run's CPU and
disk counters before launch are in its raw notes).

Scratch material, none of it committed: a source copy of the commit
(`git archive HEAD`) built with its own `CARGO_TARGET_DIR` in the session
scratchpad, a `notifyprobe` Cargo binary, and Python drivers for the MCP
endpoint run through `mise x -- python -I`.

### Item 1: delete under a notify watch

A scratch binary (`notify::recommended_watcher`, `RecursiveMode::NonRecursive`
on a canonical temp folder, as `watch.rs` / `treewatch.rs`; `trash = "=5.3.0"`,
the version `crates/app/Cargo.lock` resolves) built twice, `notify = "=8.2.2"`
and `"=8.2.0"`; one run each, the same results on both:

```text
self_empty_remove_dir: depth=0 files=false how=remove_dir -> Ok | exists=false | recreate=Ok | events=[]
self_files_remove_dir_all: depth=0 files=true how=remove_dir_all -> Ok | exists=false | recreate=Ok | events=["Remove(Any) [a.ARW]"] (8.2.0 also listed a.xmp)
parent_remove_dir_all: depth=1 files=true how=remove_dir_all -> Ok | exists=false | recreate=Ok | events=["Remove(Any) [a.ARW]"]
grandparent_remove_dir_all: depth=2 files=true how=remove_dir_all -> Ok | exists=false | recreate=Ok | events=["Remove(Any) [a.ARW]"]
self_files_trash: depth=0 files=true how=trash -> Ok | exists=false | recreate=Ok | events=["Modify(Any) [a.ARW]", "Modify(Any) [a.xmp]"]
parent_trash: depth=1 how=trash -> Err 0x80270021 "access denied to the source" | exists=true | recreate=Err os 183 | exists_after_drop=false (cleaned up after the drop)
grandparent_trash: depth=2 how=trash -> Err 0x80270021 | exists=true | recreate=Err os 183
self_rename_control: depth=0 how=rename -> Ok
parent_rename_control: depth=1 how=rename -> Err PermissionDenied (os error 5)
```

`depth` is how far above the watched folder the deleted folder is. The
event lists are what arrived within 500 ms of the delete. `usage.md` and
`usage.ja.md` now say the file manager can neither rename the folders
holding an expanded folder nor move them to the Recycle Bin. A file
manager's permanent delete (Shift+Delete) was not measured.

### Item 2: `configure` behind `Unwatch` on notify 8.2.2

Read `notify-8.2.2/src/windows.rs` from the Cargo registry (summary in
`docs/agents/shared-crates.md`). The run, the same scratch binary on
`=8.2.2` with a `loop` mode: per iteration create `a\b`, watch `b`, unwatch
it, optionally `configure(Config::default())`, then `fs::rename(a, ...)` at
once; 300 iterations without `configure`, then 300 with it, while 24 threads
spin; three batches:

```text
loop iters=300 load_threads=24 fails_without_configure=2 fails_with_configure=0
loop iters=300 load_threads=24 fails_without_configure=0 fails_with_configure=0
loop iters=300 load_threads=24 fails_without_configure=4 fails_with_configure=0
```

The engine's own `treewatch` tests were not run against 8.2.2 (the
scratch binary was the cheaper way to get 8.2.2 without touching a lock).

### Item 6: plain `mise install` and rust-analyzer

`mise 2026.9.5 windows-x64 (2026-09-10)`; rustup 1.29.3. `mise current rust`
printed `1.97.1` at the root and `1.98.1` in `crates/app`.

- Root, 20:17:44 to 20:18:07 JST: `rustup component remove rust-analyzer
  --toolchain 1.97.1` (`info: removing component rust-analyzer`, plus a
  harmless `could not delete temp file` warning), then `mise install`, which
  re-resolved `rust@1.97.1`, printed `info: downloading component
  rust-analyzer` and `1.97.1-x86_64-pc-windows-msvc updated`, then `rustup
  component list --installed --toolchain 1.97.1` listed
  `rust-analyzer-x86_64-pc-windows-msvc`.
- `crates/app`, 20:18:20 to 20:18:48 JST: the same with `1.98.1`; `mise
  install` downloaded the component again and the list had
  `rust-analyzer-x86_64-pc-windows-msvc`.

So a plain `mise install` restores the component on a toolchain that lacks
it; `--force` is not needed. The component was missing for about 25 s per
toolchain. The `1.97-x86_64-pc-windows-msvc` toolchain was not touched.

### Item 7: the themes' upstream licenses

Checked through `gh api` (each repository's license, then its LICENSE,
README or scheme file header): the per-file result and the notes are in
`crates/app/themes/LICENSE`. Undetermined: Alduin, Adventure (with Adventure
Time), Kibble. CC BY-SA 4.0: Harper, Fahrenheit. The asciinema row's
upstream was found by matching the palette with asciinema-player's
`_asciinema.less`; the Twilight row's is TextMate's `themes.tmbundle`.

### Item 8: the filter / sort and EXIF filter boxes through MCP

Build: debug, `--features screenshot`, from the scratch copy of `3918064c`,
one fresh `--index` / `--settings` / `--log` per run, the settings seeded
from copies of the `D:\Photos\tests\20261113-filter-sort\` JSONs with
`mcpEnabled: true` added, on copies of its folders under
`D:\Photos\tests\20261123-todo-measurements\fs\`, plus `fs\long40` and
`fs\race40` (hard links to 40 ARW each of `2026-11-07`) and `src1000`
(hard links to its first 1000 ARW). Driven by a Python client of the
Streamable HTTP endpoint on a test `--mcp-port`, window 1568x980 in the
captures.

- Filter-sort box 1: `faces` with `settings-filter-good.json`. Polled every
  100 ms: count 0 and `No files match the current filter.  analyzing n/8`
  from `0/0` to `7/8`, then count 2 at the pass's end, current
  `ILCE-1M2_sony_a1_ii_03.arw`. The captures: an empty strip with the
  centered message, then the two cells (`ILCE-1M2_sony_a1_ii_03.arw`,
  `_DSC1758.ARW`) with the green `scan-face` icons and `1 / 2 (8)`. The fill
  is one step because pass 2 writes `BATCH` (10) rows per transaction.
  `scan faces: ... files=8 done=8 ... threads=3 ... in 2718ms`. Box deleted.
- Filter-sort box 2: `menu\faces` with `settings-name.json`:
  `set_judgment` pick on `_DSC1616.ARW` (after `show_photo`), then
  `set_filter` `flags: [picked]`: count 1, current `_DSC1616.ARW`; the
  capture shows one cell with the pick dot and `1 / 1 (8)`. `fs\long40`
  (40 files, longer than the strip) with ratings set through
  `set_judgment` on four files: current `_DSC3222.ARW` at 31, `set_sort`
  rating, still `_DSC3222.ARW` at `32 / 40`, its outlined cell at the
  strip's right end in the capture; with the 3-star `_DSC3227.ARW` current
  at 3, `set_sort` name moved it to `36 / 40`, outlined and in view. Those
  two parts removed from the box; the click-through and the held `→` stay.
- EXIF box 1: `src1000` (fresh index, 1000 ARW of one body and lens),
  polled every 50 ms: `filter_offers.exif` empty from 1.1 s (`scanning
  0/1000`) to the end of pass 1 (`scan extract: ... files=1000 ... in
  9955ms`), then `camera, lens, aperture, shutter, iso, focal` from the
  first poll in pass 2, `lens: [FE 50-150mm F2 GM]`. On `faces`, `lens` held
  six lenses after five cameras. No tool opens the menu: the box is
  narrowed to the drawn menu.
- EXIF box 2: `filter\raw` with `settings-filter-untagged.json`: three
  `set_judgment` picks of the current file, each landing on the next file
  within 15-41 ms (count 5 to 4 to 3 to 2); the captures 1.2 s later show
  the meta pane's heading equal to `get_view`'s current
  (`DSC-RX0M2_RX000224.ARW` with `1 / 4 (6)` in the first). Box deleted.
- EXIF box 3: `sort\raw` with `settings-name.json`: `show_photo`
  `DSC-RX0M2_RX000224.ARW` (3 / 6), `set_filter` `picked` + `red`: count 0;
  `set_filter` `labels: []`: still 0; `set_filter` `flags: []`: count 6,
  current `DSC-RX0M2_RX000224.ARW` at 3, the capture showing it. Read from
  `companion.rs`: without `reset`, `set_filter` sends the menu's toggles
  through `apply_menu`, the same path a menu uncheck takes. Box deleted.
- EXIF box 4: `fs\race40` with `{"flags":["untagged"],"candidates":["neutral"]}`
  on a fresh index: six `set_judgment` picks during pass 2 (`analyzing`
  0 to 38 of 40), each followed 0.7 s later by `get_view` and a capture; all
  six captures showed the current file's name, image and outlined cell, so
  the race did not show in six presses. Box deleted.

### Items 3 and 4: the read cap and the pause, NVMe (and why not SATA)

Builds, all release from the scratch copy of `3918064c`, one shared
`CARGO_TARGET_DIR`: `base` (unpatched: cap 2, pause on), `cap22`
(`crates/app/src/scan.rs`: `pub const READ_SLOTS: usize = 22;`) and
`pauseoff` (cap 2; `crates/app/src/main.rs` `load`: `let paused =
slots.pause();` replaced by `let _ = slots;` and `let paused =
ReadSlots::new(1).pause();`, a pause on a `ReadSlots` no worker uses, so
`Loaded` and `PaintHold` are unchanged). The scratch copy was restored
afterwards; the committed tree never carried either patch.

Material: `D:\Photos\tests\20261123-todo-measurements\src1800`, hard links
to the first 1800 ARW (by name, `_DSC1492` to `_DSC3291`) of
`D:\Photos\2026\2026-11-07` (α7 V), 76.1 GB; per run a `robocopy /J` copy
to `C:\Users\daisu\AppData\Local\Temp\handpic-nvme\<run>` (the first run's
went to a folder named `handpic-nvme$name` by a quoting slip), 60 s of
settling, the CPU and disk counters sampled, then the app with a fresh
`--index`, `--settings` (seeded with `{"mcpEnabled": true}`), `--log` and a
test `--mcp-port`, stdout (the `page` lines) redirected to a file. The
driver (Python over the Streamable HTTP endpoint) polled `get_status` and,
while `scan.pass` was `scanning`, called `show_photo` on the file at index
1799, 1759, 1719, ... (stride 40) every 0.3 s while that index was more than
100 past `scan.done`; it stopped at pass 1's end, the app was closed with
`taskkill` and the copy deleted (each deleted; the empty parent removed at
the end). A probe counts when its `page` line exists with `source=read`;
p50 / p95 are linear-interpolated percentiles of those reads.
`MCP show_photo` was first checked to write a `page` line as a key does
(`page wait=0.02ms ... source=...`).

Chronological runs (CPU = total processor time sampled three times just
before launch; other sessions were compiling and testing throughout):

| Run | Build | Copy start (JST) | CPU before | Probes | Read p50 / p95 (ms) | `scan extract` |
| --- | ----- | ---------------- | ---------- | ------ | ------------------- | -------------- |
| nvme-cap2-1 | base | 21:03 | 66-78 % | 0 of 10 (driver bug: the verbatim prefix lost in a heredoc) | - | 1800 in 10128 ms |
| nvme-cap22-1 | cap22 | 21:19 | 28-88 % | 11 | 11.81 / 15.57 | 1800 in 4475 ms |
| nvme-cap2-2 | base | 21:24 | 100 % | 28 | 14.60 / 39.20 | 1800 in 19965 ms |
| nvme-cap22-2 | cap22 | 21:30 | 55-69 % | 11 | 21.66 / 65.37 | 1800 in 4297 ms |
| nvme-cap2-3 | base | 21:35 | 37-43 % | 18 | 10.98 / 13.60 | 1800 in 9908 ms |
| nvme-cap22-3 | cap22 | 21:40 | 12-13 % | 7 | 10.34 / 11.10 | 1800 in 2756 ms |
| nvme-pauseoff-1 | pauseoff | 21:45 | 21-24 % | 14 | 8.29 / 9.35 | 1800 in 7214 ms |
| sata-on-1 | base | 21:50 | 7-12 % (D: 133-185 % busy) | 20 | 26.63 / 33.62 | 1000 in 10278 ms |
| sata-off-1 | pauseoff | 22:08 | 73-86 % (D: 127-190 % busy) | 2 of 36 | 146.3, 328.4 | 1000 in 47525 ms |
| nvme-cap2-4 | base | 22:23 | 9-10 % | 16 | 8.84 / 10.16 | 1800 in 7541 ms |
| nvme-pauseoff-2 | pauseoff | 22:29 | 64-92 % | 23 | 16.25 / 21.70 | 1800 in 12195 ms |
| nvme-cap2-5 | base | 22:34 | 12-14 % | 16 | 9.62 / 15.11 | 1800 in 8028 ms |
| nvme-pauseoff-3 | pauseoff | 22:39 | 11-12 % | 16 | 9.26 / 17.27 | 1800 in 7669 ms |

Every `scan extract` line had `errors=0 threads=22 canceled=false`. Raw
reads per run (ms):

- nvme-cap22-1: 11.6 11.8 12.8 15.3 10.7 9.1 10.8 10.9 15.9 13.6 12.0
- nvme-cap22-2: 36.2 29.3 21.7 94.5 16.4 30.1 21.7 11.4 13.3 11.5 10.6
- nvme-cap22-3: 9.3 10.3 10.7 10.2 10.0 10.8 11.2
- nvme-cap2-2: 19.5 14.0 28.6 20.0 12.9 12.4 12.0 20.5 9.6 22.6 28.0 21.5 10.7 12.3 11.5 11.2 21.1 19.6 10.3 12.2 10.7 72.0 15.2 25.5 12.5 10.9 37.6 40.1
- nvme-cap2-3: 11.0 7.5 13.4 12.8 10.8 14.7 11.2 10.8 11.0 12.1 10.1 11.2 10.2 10.5 10.6 10.1 11.0 11.2
- nvme-cap2-4: 9.9 8.8 11.0 9.0 9.3 7.9 9.5 7.7 7.7 7.5 7.9 9.2 7.7 8.1 8.9 9.2
- nvme-cap2-5: 9.4 13.8 11.3 9.5 13.2 12.8 8.8 19.0 8.7 8.1 8.6 10.4 10.5 9.8 8.3 7.3
- nvme-pauseoff-1: 7.3 7.1 8.9 9.5 9.3 8.3 7.1 9.3 8.2 8.5 8.2 7.8 7.7 9.1
- nvme-pauseoff-2: 20.9 21.8 15.0 17.8 12.9 18.2 15.0 11.6 18.2 19.3 16.2 18.4 20.8 20.1 9.7 11.3 12.2 10.8 18.8 24.0 12.7 10.7 11.6
- nvme-pauseoff-3: 10.7 10.1 15.7 21.9 12.2 10.5 8.4 7.5 9.6 7.0 6.6 8.4 7.2 10.6 8.9 8.5
- sata-on-1: 20.3 34.4 30.8 26.5 25.7 26.5 33.6 22.2 25.1 29.1 28.3 33.3 27.1 20.9 26.7 28.4 25.7 29.6 16.1 20.9

The SATA runs (`src1000`, the first 1000 of the same files, 42.0 GB, copied
D: to D: under `D:\Photos\tests\20261123-todo-measurements\cold\`, stride
25, margin 60) were stopped after two: another session's
`grep -rl face.png ../../../` (started 20:10, still running at 22:45) was
reading `D:` at 250 to 480 MB/s the whole time, so the disk never settled.
`sata-off-1`'s reads queued behind it for hundreds of milliseconds and most
jumps were superseded before their page drew. Neither run is used in the
guides; item 4's SATA comparison stays in `todo.md`. The `cold` folder was
deleted.

Conclusions recorded in the guides: on NVMe cap 22 fills pass 1 2-3 times
faster than cap 2 with no slower preview read, so the numbers argue
against cap 2 there (`READ_SLOTS` not changed; the todo item became the
decision); the pause made no measurable difference to the read on NVMe.

### Item 5: the app's scan lines for the Focus candidate pass

`D:\Photos\2026\2026-09-19` (282 ARW, 14 GB) copied with `robocopy /J` to
`D:\Photos\tests\20261123-todo-measurements\sep19` (ARW only). The copy was
then read with `cat` (110 s, and 110 s again, so 14 GB did not stay in the
file cache) and scanned by `handpic-cli scan` twice (release, 24 threads:
1.76 s, then 0.23 s). The base release app opened it twice with fresh
`--index` / `--settings` / `--log`, waited out through MCP `wait_for_idle`
until `scan faces:` was logged: first open `scan extract ... in 212ms`,
`scan faces ... threads=3 ... in 7344ms`; second open 251 ms and 6726 ms
(CPU 11-14 % before it). `handpic-cli scan` after: 0.22 / 0.24 / 0.26 s,
mean 18.1 / 19.5 / 21.6 ms, p95 24.1 / 24.5 / 29.8 ms per file. `D:` was
being read at about 480 MB/s by the other session's `grep` during all of
it, so no cold first open was recorded.

## Deferred issues (todo candidates)

- **Align the two `notify` locks so CI tests the shipped version.** The
  re-check above confirmed `configure` still queues behind `Unwatch` on
  notify 8.2.2 (the version `crates/app/Cargo.lock` resolves and the app
  ships), but CI's `treewatch` / `watch` tests run on the root lock's
  8.2.0. Moving the root lock to 8.2.2, or pinning the app lock to 8.2.0, is
  a lock change this measurement plan does not make. Basis: item 2 of this
  step, the plan's Trade-offs ("Item 2: which lock is right"). Files:
  `Cargo.lock`, `crates/app/Cargo.lock`, `crates/engine/src/watch.rs`,
  `docs/agents/shared-crates.md`.
- **Measure a file manager's permanent delete (Shift+Delete) of a folder
  holding a watched one on Windows.** Item 1 measured `std::fs` deletes
  (they succeed) and the shell's move to the Recycle Bin (refused), not the
  shell's permanent delete, which `docs/humans/usage.md` would describe if it
  behaves differently. Basis: item 1 of this step. Files:
  `docs/agents/shared-crates.md`, `docs/humans/usage.md`,
  `docs/humans/usage.ja.md`.
