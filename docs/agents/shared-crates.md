# Working on the shared crates and the tooling

Read this before touching `crates/core`, `crates/backend` or `crates/cli`
(the index, the scan passes, the sidecars, the trash, the JPEG decode, the MCP
server), the CI workflows or the release tooling, or before running checks on
this machine. It lists the pitfalls this repository has already hit, each with
the reason it happens. The desktop app itself (`crates/native`) has its own
guide, [`native-app.md`](./native-app.md).

Each item is tagged:

- **Hit**: actually broke something here.
- **Measured**: measured in this repo and steered a design decision; nothing
  broke, but the naive choice would have missed a stated budget or cost more.
- **Inferred**: taken from the sources or docs; nothing has broken on it yet.

Many items were first learned on the earlier webview app, which the native app
replaced; its source is in git history.

## Index, scan and files

### An index/scan mutation must not race a rescan: no scan runs across the disk op and the index write (Hit)

An operation that both changes what is on disk and writes index rows for it
(a folder rename, a split by date, a trash run, its undo and redo, a sidecar
delete) must not let a scan start mid-way and index the half-done state, nor
race the operation's own reconcile. Releasing the guard between files would
let a rescan index a half-moved folder.

- The backend's bodies (`rename::rename_folder`, `datefolder::split_folder`,
  `foldersidecars::delete_run`, `trash::trash_listed` and the rest) do not
  check for a scan or drain the sidecar writer; the caller does. The earlier
  app held a `Scans` lock across the call; the native app holds the operation
  in `idle::IdleGate` until the listing or scan under way ends (see
  `native-app.md`, "A trash or delete goes through the gate, and nothing
  lists under it").
- Do the refusal checks before anything on disk changes, and place a
  blocking step (canonicalizing / stat-ing paths) first, before any flush: a
  pending flush can itself create the file the refusal needs to see.
- Source: `docs/plans/_archived/20260928-rename-from-tree-and-strip/learnings.md`, Steps 1 and 3;
  `docs/plans/20261106-trash-over-scan/plan.md`, Step 3.

### Undoing a trash run after a folder rename, and walking the tree without following links (Hit)

- `Trashed.path` stays the original path because the Windows / Linux Trash is
  looked up by it (`trash_key`). The renamed destination lives in
  `restore_to`, which `Runs::rename_dir` rebases with the same
  `old + MAIN_SEPARATOR` prefix rule as `Index::rename_dir`. `rename_folder`
  calls it outside `if let Some(index)`, so the runs follow even with no
  index.
- trash 5.2.9's Windows `restore_all` fails with "file not found" when the
  original folder is gone. Only the freedesktop backend runs `create_dir_all`.
  For a file with `restore_to`, create the missing old ancestors first
  (`trash::create_missing`, returning exactly the ones created), then
  `restore_all`, then `fs::rename` to the new place. Afterwards `remove_dir` only
  the created folders, deepest first and only while empty, so a pre-existing
  empty old-name folder is never removed. Verified with a probe.
- The folder tree's `Expand All` lists a symlink / junction child (`is_link`
  from `folders::list`) without descending. `list` follows links, so `a/loop -> a`
  would otherwise grow without end. The walk also stops a branch as soon as any
  ancestor was collapsed, dropped or re-keyed by a rename.
- Source: `docs/plans/_archived/20260930-todo-five-more-items/learnings.md`,
  Steps 3 and 5.

### Moving files between folders (split by date): what the index, watchers and trash runs do (Hit)

- `Index::forget_dir` is not `evict_folder`. Eviction keeps dirty `ratings`
  rows and leaves `views`, so an unwritten judgment of a folder still on disk
  is not lost. `forget_dir` is for a folder removed from disk. It drops every
  row of exactly that `dir` (`files`, `ratings` including dirty ones,
  `embeddings`, `views`, `folders`), and touches neither subfolders nor prefix
  siblings such as `photos2`.
- `Index::move_file` leaves the old folder's `folders.last_viewed` naming a
  file now elsewhere. That is harmless, because the app resumes at it only
  when the listing holds it.
- A split that removes the original folder must release the open folder's
  watcher and the tree watches first (`with_released`), or Windows returns
  `PermissionDenied`. `with_released` restores the tree watches only on
  `Err`, so after a split that keeps the original folder, put the open
  folder's watcher back with `watch::set`. The tree watches come back with
  the app's next `TreeWatch::set`, which makes the watched set equal to the
  folders it is given.
- Create each target folder with `std::fs::create_dir`, not `create_dir_all`.
  It fails when the name appeared after the re-plan, so every file of that
  group fails and nothing is ever merged. A split target named like the
  source folder is a collision, because a split merges nothing.
- A move does not rebase `Runs` or the app's undo history. An undo of a
  trash run recorded before the split restores to the old path (which may be
  gone), and judgment undo entries still name the old paths.
- Source: `docs/plans/_archived/20261104-rename-folder-by-capture-date/learnings.md`, Steps 1-3.

### Resolve shortcut overrides order-independently, not in a single pass (Hit)

`from_overrides` in `crates/backend/src/shortcuts.rs` checks each stored override
against the keymap built so far. A single pass in action order silently drops
a valid override that wants a key another override in the same batch frees
later: `{"pick": ["q"], "reject": ["p"]}` left `reject` on its default,
because `reject` comes before `pick`, so `p` still looked taken.

- Fix: apply the non-conflicting overrides first, then retry the conflicting
  ones until no more apply.
- When two overrides genuinely want the same key, the first in action order
  keeps it; only after the retries settle does the later override apply its
  other keys without it (skipped only if no key is left), and the retries run
  again, as that replaces the action's keys and may free one. Applying a
  partial override can itself free a key an earlier partial override
  dropped, so once the outer retry settles, recompute every partly applied
  action's keys as its originally requested keys filtered by what is free
  at that point (never a conflict, since only free keys are added).
- Adding a new default key (`DEFAULTS`) wins over any stored override that
  holds the same key. `from_overrides` drops that key from the user's action
  silently on upgrade. In tests, a stored override that uses an arbitrary key
  (for example `a_stale_default_in_an_old_store_drops_only_that_key` used `d`)
  fails once a default takes that key, so use a key no default holds. A
  main-view key that the folder tree swallows is absent from
  `TREE_PASSTHROUGH` (`crates/native/src/treekeys.rs`).
- Source: `docs/plans/_archived/20260920-pick-shortcut-editable/learnings.md`,
  Step 1; `docs/plans/20260929-shortcut-override-partial-conflict/learnings.md`,
  Step 1; `docs/plans/_archived/20261011-performance-graph-overlay/learnings.md`,
  Step 3.

### Volume listing: dedupe home by canonical path, plus a macOS-only ancestor rule (Hit)

`volumes()` in `crates/backend/src/folders.rs` drops the home volume's duplicate
by canonical-path equality, and, on macOS only, also drops "a volume whose
canonical path is an ancestor of home": `/Volumes/Macintosh HD` resolves to
`/`, which never equals the home directory itself.

- This rule is macOS-specific: on Windows, home is `C:\Users\<name>`, whose
  ancestor `C:\` is a real, independently browsable root (unlike macOS's
  volume alias for `/`), so applying the same ancestor rule there would
  silently drop it from the tree. Don't port the ancestor check to Windows.
- Consequence: when home itself lives on an external volume on macOS, that
  volume is not listed separately.
- On Linux, `/mnt` lists every child directory as a root, unfiltered, since
  plain Linux users mount arbitrary names there. On WSL (`WSL_DISTRO_NAME`
  set) only the single-letter drive mounts like `/mnt/c` are kept, dropping
  WSL's own `/mnt/wsl` and `/mnt/wslg` (`wsl_drive_mounts`).
- `wsl_drive_mounts` is compiled under
  `cfg(any(test, not(any(target_os = "macos", target_os = "windows"))))`,
  not plain `cfg(test)`: its test runs on every platform while the function
  stays out of macOS / Windows builds without a dead-code warning. Reuse this
  shape for other Linux-only helpers that need tests on non-Linux CI.
- Windows also skips entries with `FILE_ATTRIBUTE_HIDDEN` (a `MetadataExt`
  one-liner) in addition to dot-names. `cargo check --target
  x86_64-pc-windows-gnu` in `crates/backend` verifies `cfg(windows)` code
  without a Windows machine, when that target is installed.
- Source: `docs/plans/_archived/20260925-lightroom-layout/learnings.md`, Step 2.

### Measure before choosing a JPEG payload over raw pixels (Measured)

`mozjpeg::Compress`'s defaults turn on trellis quantization and optimized
Huffman tables, so re-encoding a crop costs more than the partial decode that
produced it: 49ms at q85 for a 1037x1024 crop, against 21.5ms to decode it.

- Why: the defaults optimize for file size, and both passes run over every MCU.
- The earlier app's `focus_crop` therefore returned raw RGBA (4.2MB at the 1024
  cap) to its webview rather than a ~180KB JPEG, trading IPC bytes for CPU.
  Before re-encoding a decoded image anywhere, take the measurement, and
  measure with the optimizations off, not with the defaults.
- Confirmed at full-image scale: a DCT-scaled-and-reboxed re-encode capped at
  2048px (`decode::thumbnail_jpeg_near` + mozjpeg q85) took 183-535ms on a
  21.5MP source, slower than shipping and decoding the whole JPEG (~200ms).
  Source: `docs/plans/_archived/20260927-jpeg-view-only/learnings.md`, Step 2.

### A partial decode's cost is set by its row, not its size (Measured)

`jpeg_skip_scanlines` on a baseline JPEG still entropy-decodes the rows it
skips; it only skips the IDCT and color conversion.

- Why: baseline Huffman data is not randomly addressable, so libjpeg must walk
  every MCU row from the start of the scan to reach the wanted one.
- Measured on the 7008x4672 `JpgFromRaw`: a 1024 crop costs 10.8ms at row 300
  and 44.1ms at row 4400, while growing the crop from 512 to 2048 at the same
  row only moves 18.4ms to 29.8ms.
- So a budget for a crop has to be stated for the worst row, not an average
  one, and shrinking the crop is not a way to make it fit.

### A crop's origin snaps to an MCU boundary; don't assume centered or bit-identical (Hit, twice)

`jpeg_crop_scanline` snaps the requested crop origin **down** to an MCU
boundary (16px in the tested files) and does not necessarily widen the crop to
compensate.

- Why: baseline JPEG DCT blocks (MCUs) are the smallest unit libjpeg can crop
  to; it never crops mid-MCU.
- What broke: two independent guesses were wrong. `FocusCrop`'s `point_x`/
  `point_y` are not the crop's center (measured 269 vs a center of 262 on one
  file). `crop_size()`'s width does not grow to keep the point centered (a
  64px-wide request at x=200 came back as width 64 at x=160, landing the point
  at 40, not 32) — that behavior also isn't consistent across files (it does
  widen on a 4:2:2 test file). Always derive the point of interest as
  `center - crop.origin` from the actual returned crop, never assume a fixed
  offset or that width grows.
- A crop decoded via `jpeg_crop_scanline` is also **not bit-identical** to the
  same region of a full `decode_rgb` near its left/right edges (chroma
  upsampling has one fewer neighbor there; differences up to 2 in the first
  columns, 1 in the last, on the tested file). Equality tests against a crop
  must exclude ~8px from each edge or use a tolerance there, not assert exact
  equality everywhere.
- Source: `docs/plans/_archived/20260918-focus-check/learnings.md`, Steps 1
  and 2.

### mozjpeg panics, rather than returning `Err`, on non-JPEG bytes (Hit)

`Decompress::new_mem` and friends panic on bytes that are not a JPEG instead
of returning a `Result::Err`.

- Any code path that cannot guarantee its input is a real JPEG (e.g. scoring an
  arbitrary embedded preview) must wrap its own call into mozjpeg in
  `catch_unwind` and convert the panic to an `Err`; callers then just call
  `.ok()` on it.
- Source: `docs/plans/_archived/20260919-sharpness-cue/learnings.md`, Step 1.

### `partial.rs` replaces libjpeg's `error_exit`, which would call `exit(1)` (Hit)

`crates/core/src/partial.rs` (`decode_region`, behind `decode_focus_crop` and
`decode_crop`) calls libjpeg through `mozjpeg-sys` with `jpeg_std_error`, whose
default `error_exit` calls `exit(1)`: a fatal error there used to end the whole
process, and `catch_unwind` could not stop it.

- The guard is in place: `decode_region` installs `unwind_error_exit`, an
  `extern "C-unwind"` handler that formats libjpeg's message and
  `resume_unwind`s (no panic hook, so nothing is printed), and wraps the body in
  `catch_unwind`. `decode_focus_crop` / `decode_crop` now return `Err` with the
  text (e.g. `libjpeg fatal error: Not a JPEG file: starts with 0x3c 0x44`), so
  the app's 1:1 crop read reports the error instead of exiting, and
  callers no longer need the `decode_rgb` pre-check `handpic-cli check` used to
  run.
- Unwinding through libjpeg's C frames needs `mozjpeg-sys`'s `unwinding`
  feature; `crates/core/Cargo.toml` names it explicitly.
- A JPEG cut inside its scan data is not a fatal error: libjpeg warns
  (`Premature end of JPEG file`) and the crop comes back with filler rows. Only
  a cut inside the header, an empty input or a non-JPEG fails.
- `partial.rs`'s decoder and the `mozjpeg` crate's `decode_rgb` are not
  interchangeable validators: `decode_rgb` also fails on a JPEG cut inside its
  scan data, which `partial.rs` accepts with a warning. To test the `Err` path,
  cut inside the header (e.g. `&jpeg[..64]`), not the middle of the data.
- Set the handler through the pointer,
  `(*cinfo.common.err).error_exit = ...`, not on the local `err` after
  `jpeg_std_error(&mut err)`. rustc does not see the read through the raw
  pointer and trips `unused_assignments`.
- Seen on `NEF\NIKON_D70_Nikon.nef` and `DNG\CGO3P_YUN00007.dng`: before the
  guard, a run died mid-way with libjpeg's `Empty input file` / `Not a JPEG
  file` on stderr, exit code 1 and no summary.
- libjpeg warnings (`Corrupt JPEG data`, `Invalid SOS parameters`) still go
  straight to stderr, while `check`'s failure lines and summary are on stdout.
- Source: `docs/plans/_archived/20261001-cli-check-samples/learnings.md`, Step 1;
  `docs/plans/_archived/20261002-partial-decode-error-exit/learnings.md`, Step 1.

### A bounded drain must fail loudly, and a sidecar test must not swallow writer errors (Hit)

`Writer::flush` returns `bool` (`true` when the drain reply arrived within
`DRAIN_TIMEOUT`). Production callers may ignore it, but tests must not. The
switch-to-Both sidecar test was flaky on `windows-latest` because the 2 s drain
gave up while the writer was still inside `write_kind` (`sync_all` plus the
Windows `rename` retries), or because the write failed and `on_error` only
printed to stderr. Either way the failure surfaced one step later as `NotFound`
on the read.

- In a sidecar-writer test, collect every `on_error` message into a
  `Mutex<Vec<String>>`. Drain with a generous budget (30 s), and assert both the
  `flush` result and that the errors are empty before reading any sidecar.
- Filter out `" (retrying in "` messages when asserting. A write that fails once
  on the deadline path and succeeds on the rewrite is correct. Keep the full list
  in the assertion message so a transient sharing violation stays visible.
- Source: `docs/plans/_archived/20260930-todo-five-more-items/learnings.md`, Step 1.

### Snapshot the state you decided on, not just the lock, across a release (Hit)

`reconcile_sidecars` (decides what to parse) and `store_sidecar_ratings`
(stores the parse and clears `dirty`) run with the lock released in between,
so a `set_rating` landing in that window was silently discarded by the
"sidecar wins" write.

- Why: releasing a lock between "decide" and "act" always opens a window for
  the state to change underneath the decision; re-checking "is the lock still
  free" is not enough, the *value* the decision was based on has to be
  re-checked too.
- Fix pattern: `reconcile_sidecars` hands back the `dirty` flag it observed
  for each path alongside the parse job, and `store_sidecar_ratings` only
  applies its update when the row's `dirty` still matches that snapshot —
  otherwise the newer write is left in place.
- Source: `docs/plans/_archived/20260918-ratings-xmp-sidecars/learnings.md`,
  Step 4.

### This machine cannot compile the macOS target; check `objc2` code in a scratch crate (Hit)

`cargo check --target aarch64-apple-darwin` failed here while the earlier app
was in the workspace, because `objc2-exception-helper`'s build script needs a
C compiler for that target. That crate is no longer in either `Cargo.lock`;
whether `crates/backend` alone now checks for that target here is
unverified. Until it is, the macOS `cfg` branches (such as
`crates/backend/src/trash.rs`'s `trashItemAtURL`) are verified only by CI's
macOS job. To partially self-check new macOS-only code (e.g. `objc2-foundation`
calls) before pushing, copy just that code into a scratch crate depending
on the same `objc2-foundation` / `percent-encoding` versions pinned in
`Cargo.lock`, and run `cargo check` / `cargo clippy -D warnings --target
aarch64-apple-darwin` on the scratch crate. This does not compile the rest
of the macOS cfg (the callers, `dead_code` attributes elsewhere), so
still expect CI's macOS job to catch anything outside the copied slice.

- `objc2-foundation` 0.3.2's `trashItemAtURL_resultingItemURL_error` is a
  safe fn (no `unsafe` block needed); letting the out-pointer argument's
  `None` infer its type avoids a direct `objc2` dependency for `Retained`.
- Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md`,
  Step 2.

### Trash restore: plan and perform in one pass, and re-verify identity before renaming back (Hit)

A restore plan computed before any file is moved cannot know whether a
given RAW's move will actually succeed, but a sidecar's fate depends on it:
sidecars only come back when their RAW does (mirroring `trash::run`'s own
rule), otherwise a reject sidecar could land next to an unrelated file that
took the RAW's freed name. So `trash::restore` plans and performs the
restore in one pure function taking `(run, in_trash, exists, mover)`,
rather than returning a plan struct to execute later.

Also, a file existing at the recorded trashed-to path is not enough to call
it "still in the Trash and safe to restore": the OS trash can hand a freed
name to an unrelated file trashed later from elsewhere (e.g. repeating
camera file names like `DSC00001.ARW` across cards). Record the file's
identity at trash time (on Unix, `(dev, ino)` via `MetadataExt`) alongside
its trashed-to path, and require it to still match before renaming back.

- Source: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md`,
  Steps 1-2.

### A trash run of sidecars alone: check how `trash::redo` regroups the paths (Hit)

`Delete Sidecars…` records a run whose every path is a sidecar, and reuses
the trash runs' undo and redo (`trash::undo_run` / `trash::redo_run`) for it. `trash::restore` takes
such a run as is (`raw_back` starts `true`), but `trash::redo` rebuilds the
RAW-and-sidecars groups from the flat path list, and it used to append every
non-RAW path to the previous group: a sidecar-only run became one group
headed by its first sidecar, so a redo reported one moved file and, when
that first sidecar was gone, skipped all the others. A non-RAW path now
joins the previous group only when that group's head is a RAW. Before
recording a new kind of run in `Runs`, run its undo and redo in a unit test
rather than reasoning from the code (both of these were "read, not run" in
the plan).

- A redo writes no index row (it cannot tell which RAW a
  sidecar path belongs to), so a redo of a sidecar delete relies on the open
  folder's resync to clear the judgments; a closed folder keeps them until
  its next open.
- Source: `docs/plans/_archived/20261005-folder-sidecars/learnings.md`,
  Step 2.

### A new kind of primary file in a trash run: check `trash::restore`'s notion of "primary" (Hit)

`trash::restore` counted only RAWs as a run's primary files, so a run of
JPEGs (the filmstrip's `Move to Trash…` in a view-only, JPEG-only folder)
would have come back from the Trash but listed nothing in
`Restored.restored`. The app would then say `Restored 0 files`, and would
not place them. It now treats a JPEG like a RAW
(`is_jpeg_file`). `redo` needed no change, because a JPEG after a JPEG
already forms its own group. When `Runs` starts recording a new file kind,
test restore's `restored` list for it, not just that the files return.

- The strip's move checks each path's parent against `dir` in
  `trash::groups_of` and reports any other as a `Failure` without moving it.
  That keeps a sidecar from going to the Trash with a same-stem file of
  another folder. It moves no index rows, so it need not stop the embedding
  pass: the third pass skips a vanished file, and the next rescan's
  reconcile drops the gone rows.
- Once the backend has recorded the run, the app must keep its undo entry
  even if the open folder changed meanwhile; only the strip refresh is
  conditional on the open folder being the run's folder.
- Source: `docs/plans/_archived/20261013-trash-from-strip/learnings.md`,
  Steps 1-2.

### A case-insensitive file system makes `exists()` match the wrong spelling (Hit)

On macOS APFS, `sidecar_path(arw).exists()` is true for `H.ARW.DOP` even when
the code built the path as `h.arw.dop` (or vice versa), because the file
system does case-insensitive but case-preserving lookups.

- Why: the OS resolves the path regardless of case, so an existence/rename
  check can silently hit a sidecar with different case than the one just
  built.
- Any code that renames or writes a sidecar based on `exists()` needs to
  tolerate landing on the other case's file rather than assuming its own
  spelling was used; do not assert on the exact resulting name in tests, only
  that exactly one sidecar exists.
- Source: `docs/plans/_archived/20260918-photolab-dop-sidecar/learnings.md`,
  Step 2.

### chrono `format(...)` with a user-supplied pattern: `.to_string()` panics, so write with `write!` (Hit)

`NaiveDate::format` with a pattern the user typed does not fail up front. An
unknown specifier or a lone `%` becomes `Item::Error`. A specifier that needs
a time, an offset or a timestamp (`%H %M %S %p %I %f %z %Z %s %T %R %c %+`)
makes `Display` return `fmt::Error`. `.to_string()` on that value panics.

- Format with `write!` into a `String` and map the error to a refusal message
  (`import.rs`'s `format_date`).
- `%n` and `%t` are valid and emit a newline and a tab. A control-character
  check on each produced path segment is what refuses them.
- Validate a pattern by formatting a fixed date, and run the same segment
  checks on every real date. Re-validate a stored pattern at load, falling back
  to the default, so a bad stored value never reaches a run.
- Source: `docs/plans/_archived/20261106-import-from-card/learnings.md`, Step 1.

### DxO PhotoLab and `.dop` sidecars

The `.dop` format and PhotoLab database pitfalls (indentation and keys, the
`Settings` block and `Orientation`, Uuid matching and virtual copies, and the
lookup's rename caveat) live in [`photolab.md`](./photolab.md).

### Removing an XMP element needs its end tag (Hit)

`xmp.rs`'s `locate` takes the property's local name (`LocalName` compares
against `&str`). For an element-form property it cannot return on the `Text`
event: it waits for the end tag so it can report the whole element's range for
removal.

- Clearing an element-form label removes its whole line only when nothing but
  whitespace shares that line; otherwise it removes just the element.
- Values are spliced raw with no escaping, so a value containing `"` or `<`
  would break the XMP. Fine for the label vocabularies; check it again for any
  new use of this splice path.
- Source: `docs/plans/_archived/20260919-color-labels/learnings.md`, Step 1.

### A sidecar write of two properties patches in two independent passes, and clearing one leaves its `xmlns` behind (Hit)

`xmp::write_rating` and `xmp::write_label` each touch two properties
(`Rating`/`xmpDM:good`, `xmp:Label`/`photoshop:LabelColor`): they patch one,
re-parse the spliced text, then patch the other, so each splice's byte
offsets are never computed against stale text.

- Clearing a property this way removes only the attribute or element itself;
  any `xmlns:xmpDM` / `xmlns:photoshop` declaration Handpic had added earlier
  is left in place. A sidecar that is labeled (or rated via `xmpDM:good`)
  and then cleared is therefore not byte-identical to one that was never
  touched — assert on the specific properties, not full-file equality, after
  a clear.
- Follow the same two-pass-with-re-parse pattern for any future property
  pair added to a sidecar write.
- Source: `docs/plans/_archived/20260922-lightroom-xmp-flags-labels/learnings.md`,
  Steps 1 and 2.

### Label-name reads fall back configured-name-then-English, not the other way (Hit)

`LabelNames::name(&self, color)` returns the English `color` itself when the
configured name for that color is empty — callers don't need a separate
"is this customized" check. Reads match a sidecar's `xmp:Label` against the
configured name first, then against the English name, so a sidecar written
by an English-locale Lightroom still reads as the right color even when
Handpic's settings hold Japanese (or other) names.

- Source: `docs/plans/_archived/20260922-lightroom-label-names/learnings.md`, Step 1.

### quick-xml 0.42 namespace resolution: `&str`, not `&[u8]`, and a borrowed `QName` (Hit)

`ResolveResult::Bound(Namespace)`'s `as_ref()` yields `&str` — compare it
against `XMP_NS` directly, not as bytes. A helper that resolves an
attribute's namespace and returns `ResolveResult<'a>` must tie the
lifetime to the *reader*, not to a local `String` holding the name: the
only variant that borrows (`Bound`) points into the reader's own
namespace buffer, so binding the lifetime to a temporary buffer fails to
compile (or is subtly wrong if it does).

- Source: `docs/plans/_archived/20260920-robustness-cleanup/learnings.md`, Step 2.

### Bumping `SCHEMA_VERSION` can strand an old per-version column guard (Hit)

A migration guard that adds a column for "any version other than the current
one" (e.g. `ratings.label_known` at v6) breaks once a later change bumps
`SCHEMA_VERSION` again (v6 -> v7): the guard now also fires for v6 databases
that already have the column, and the `ALTER TABLE` fails.

- Key such a guard to the version that introduced the column (`version < 6`),
  not to the ever-moving current version, and re-check the versions asserted by
  the existing migration tests whenever `SCHEMA_VERSION` moves.
- The same trap applies to a table drop: `files` used to be dropped for any
  `version != SCHEMA_VERSION`, which would have thrown away every thumbnail on
  the v7 -> v8 upgrade. It is now keyed to `version < 7`, and v8 seeds the new
  `folders` table from the surviving `files` rows.
- Audit every existing guard on each bump, not just the new one: the v11
  `ratings.flag` guard was still `version != SCHEMA_VERSION` and would have
  fired for v11 databases on the v12 bump. A range key also has to exclude
  versions whose table is dropped and recreated with the column: the v12
  `files.extractor` guard is `(10..12).contains(&version)`, because
  `version < 12` would also fire for v2-v9, where `files` has just been
  recreated with the column, and the duplicate-column `ALTER` would discard the
  cache, dirty ratings included.
- Same trap on the v16 -> v17 bump: the `folders` table was introduced at v8
  and its `CREATE TABLE IF NOT EXISTS` already carries `last_viewed`, so the
  `ALTER TABLE folders ADD COLUMN last_viewed` guard has to be
  `(8..17).contains(&version)`, not `version < 17` — the latter would also
  fire for v2-v7 databases, whose `folders` table has just been created with
  the column, adding a duplicate and discarding the cache.
- A migration test fixture built with `open` has the current schema, so faking
  an older version means dropping every column added since then too, and
  every table added since then (`DROP TABLE views;` for the v22 bump). Only
  the v8+ fixtures need this: the v2-v7 fixtures create their own tables or
  drop `folders` entirely, and `CREATE TABLE IF NOT EXISTS` recreates the new
  table anyway. When a bump touches many fixtures, a regex over the previous
  bump's `DROP COLUMN ...;` line adds the new drops in one pass. The regex
  has nothing to key on in fixtures that already had the neighbouring column
  (v19 to v22 had `edge_gap`, so the v23-era `full_eye_sharp` drop had no
  `DROP COLUMN edge_gap` line to anchor to): add the new drop to every
  older fixture by hand, v19 to v22 included.
- Put a new column at the end of `CREATE TABLE`, not next to the column it
  relates to: `ALTER TABLE ADD COLUMN` always appends, so only an end
  placement keeps fresh and migrated databases in the same column order
  (v18's `eyes_ear` and `pose_*` sit after `faces_extractor`, v19's
  `eye_offset` and `edge_gap` after them). Every fixture of an older version
  then has to drop the new columns too, and the version
  asserts should read `SCHEMA_VERSION`, not a literal.
- Dropping a column is the reverse: key the `DROP COLUMN` guard to the
  versions that had it (v23 drops `folders.culling_finished` for
  `(21..23)` and `culling_manual` for `version == 22`), remove the column
  from `CREATE TABLE` and from every older fixture's drop list, and have
  the fixtures faking a version that had it add the column back with
  `ALTER TABLE ... ADD COLUMN` instead of dropping it.
- Source: `docs/plans/_archived/20260919-sharpness-cue/learnings.md`, Step 2;
  `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 3;
  `docs/plans/_archived/20260924-index-extractor-version/learnings.md`, Step 1;
  `docs/plans/_archived/20260927-resume-last-viewed/learnings.md`, Step 1;
  `docs/plans/_archived/20261008-mesh-eyes-index/learnings.md`, Step 1;
  `docs/plans/_archived/20261013-auto-culling-finished/learnings.md`, Step 1.

### Bump `EXTRACTOR_VERSION`, not `SCHEMA_VERSION`, when extraction output changes (Hit)

`reconcile` treats a `files` row as valid only while its size, mtime **and**
`extractor` match. Before `files.extractor` existed, a fix to extraction (e.g.
#376) never reached existing caches: error rows in particular were never
retried.

- Bump `EXTRACTOR_VERSION` (`crates/backend/src/index.rs`) on any change to what
  `handpic_core::scan::extract` produces: ARW/DNG parsing or embedded JPEG tier
  selection (`crates/core/src/arw.rs`), which preview bytes are read
  (`crates/core/src/reader.rs`), thumbnail generation (`crates/core/src/decode.rs`,
  `crates/core/src/scan.rs`). The first pass computes the thumbnail and the
  metadata only.
- Bump `SCHEMA_VERSION` only when the table layout changes.
- An extractor bump re-extracts every row, error rows included, on the next
  scan of each folder, and keeps `ratings`. Stale rows are deleted by
  `reconcile` right away, so the folder shows placeholders until its rescan
  fills them in.
- Parallel branches each bump `EXTRACTOR_VERSION` from the same base, so
  the merge conflict is a number collision. When you rebase or resolve, take
  the next free number above main's (a branch cut at 11 whose sibling landed
  12 resolves to 13). Never keep your own number.
- An error row can carry the metadata of its failed extraction: the failure
  keeps the parsed orientation and shot, and `indexed_file` tells "no
  metadata" apart by a NULL `orientation`. Rows written before the version
  that added this also have it NULL, so `exif` stays `None` for them
  until the rescan the bump forces.
- The second pass has its own version, `FACES_VERSION`, stored as
  `files.faces_extractor`: bump it, not `EXTRACTOR_VERSION`, on a change to
  what `handpic_core::scan::extract_analysis` produces, the face cue (the face
  choice, the face mesh, the detector: `crates/core/src/candidate.rs`,
  `eyes.rs`, `faces.rs`) or the sharpness
  score (`sharpness::score_preview`, `crates/core/src/sharpness.rs`, or what
  `extract_analysis` feeds it) or the full-size eye sharpness
  (`full_eye_sharp`: `crates/core/src/fullsharp.rs`, its `SOFT_BELOW`, the
  eye crop size, or the full-size read it makes in `scan.rs`). Only the second pass then re-runs; thumbnails
  are kept. The column and the constant keep their names although they now
  cover the score too; a change to the columns it fills (`files.sharpness`, `files.full_eye_sharp`, and `files.eyes_ear` with `files.pose_yaw` /
  `pose_pitch` / `pose_roll`, the EAR of the more closed eye and the head
  pose from the mesh the cue already runs, and `files.eye_offset` /
  `files.edge_gap`, how well that mesh sits on the face) is a
  `SCHEMA_VERSION` bump as well. The cue covers the face nearest a trusted
  AF point and, on a frame with none, the largest confident face of the
  whole preview (`eyes::judged_face`); `9` added the latter, so a frame with
  no trusted AF point now fills every one of those columns too. That branch
  costs one extra full-size RGB decode, the luma and the mesh per faced
  file, about +28 ms in release (`handpic-cli features`, `analysis_ms`); a
  file with no face pays nothing, because `whole_image_cue` returns
  `Cue::unknown()` before any decode. The face-free frame itself stays
  Neutral: the sharpness score did not separate picks from non-picks
  (folder-stratified AUC 0.58 against 0.70 needed), so do not re-propose a
  score-based Good / Low for it without a new cue or 30 or more face-free
  rejects. See
  `docs/plans/_archived/20261010-af-eye-good-bad-cascade/learnings.md`,
  Steps 2 and 3. `FACES_VERSION` 10 added the full-size eye check: a frame
  whose AF eyes measure soft at full size (`SOFT_BELOW`) is no longer Good. It costs about +51 ms per measured (`Candidate`) frame in pass
  2, with 8 threads and a warm file cache; the cold-disk cost is unmeasured.
  `FACES_VERSION` 11 measures every frame whose cue judged a face, whatever
  its state, on the full JPEG when `has_larger_jpeg` (and the `MIN_SCALE`
  floor of `full_eye_sharpness` passes); a file without one (a RAF, an HDR
  PQ CR3, a body whose largest JPEG is the preview) stays `None`, so Neutral.
  `has_larger_jpeg` only tells a full JPEG apart from the preview (no full,
  HEVC, same offset), not how much larger it is, so the `MIN_SCALE` floor must
  stay: a full JPEG at most 1.5x the preview is near the preview's scale, where
  the full-size thresholds do not hold (the first commit dropped the floor and
  measured at scale 1; the user then decided those files stay Neutral).
  Do not measure the preview instead: with the full-size thresholds it reads
  about half the full-size value and would put most such frames in Low (see
  `docs/plans/20261115-full-eye-sharp-only/preview-vs-full.md`). A failed
  full-size read or decode stays `None` too. The tiers read
  `full_eye_sharp` alone since that plan's Step 3: Good at or above
  `SOFT_BELOW` with the other cuts, Low under `LOW_BELOW` (0.110, in
  `fullsharp.rs` next to `SOFT_BELOW`), Neutral otherwise, a missing value
  included. Step 5 of that plan removed the preview's in-focus probability
  and its candidate state from the core, the index and the types. A
  new field of a pass-2 progress row must also be copied in the native
  `filter::apply_face_ready`, which copies field by field, or a frame scanned
  while on screen keeps its old state until the folder reloads. See
  `docs/plans/_archived/20261108-full-size-eye-check/learnings.md`, Steps 3
  and 4.
- Moving the score from the first pass to the second bumped neither version:
  the score's computation did not change, so a row the old first pass scored
  (at `FACES_VERSION`) keeps a correct score, and a row the second pass has
  not reached (`faces_extractor = 0`) gets the same score from it.
- Source: `docs/plans/_archived/20260924-index-extractor-version/learnings.md`,
  Step 1; `docs/plans/_archived/20260930-scan-priority/learnings.md`, Step 2.

### `reset_sidecars` must not rewrite a field `mark_written` guards on (Hit)

`Index::mark_written` clears `dirty` only `WHERE path = ?1 AND rating IS ?4 AND
flag = ?5 AND label IS ?6`. A judgment made inside a sidecar format switch's
window has already been queued with the value the row held, so if
`Index::reset_sidecars` rewrites any of those fields in between, the guard no
longer matches: the row stays dirty with the rewritten value and the next
folder open replays that stale value over the sidecar. `reset_sidecars` used to
zero `pick` (now `flag`), which lost a pick set during a switch.

- `reset_sidecars` nulls the stat only (`xmp_size`, `xmp_mtime_ns`); it keeps
  the judgment (`rating`, `flag`, `label`). Keep it that way for any field
  added to `mark_written`'s guard.
- Source: `docs/plans/_archived/20260920-format-switch-pick-race/learnings.md`,
  Step 1.

### The good-photo mark's rule is validated against the user's stars; know what it can claim (Hit)

`crates/native/src/mark.rs`'s `photo_tier` (a port of the earlier app's
`focus.ts` `photoTier`) says whether a frame is good. The rule needs the
stored `full_eye_sharp`, `eyes_ear`, `pose`, `eye_offset` and `edge_gap`;
`af_eye_state` makes Low from `full_eye_sharp` too, on the consumer's side,
no longer from the preview's in-focus probability, which the
full-eye-sharp-only plan removed.

- The user's star scale: 1 = no subject, 2 = likely rejected, 3 and up =
  pass. The mark can say "not a miss" (the frames it marks were 93% 3+ stars,
  no 1-star, on 180 rated frames). It says nothing about taste above 3: good vs.
  not was no different at 4+ stars. Do not describe or tune it as a quality or
  best-of-burst ranking.
- Keep `MAX_EYE_OFFSET`. Every relaxation, including skipping it when YuNet's
  eye distance is small, let a 1-star frame in. Gating on YuNet's eye distance
  fails because YuNet sees a profile where the mesh sees a face
  (`2026-09-27-a__DSC6920`).
- Keep `MIN_EDGE_GAP` as a box-plus-grown-eye-region gap. Testing the 1.25x
  mesh crop flags whole faces near the edge (only hair outside).
- The EAR is inflated on a turned, foreshortened eye. That is why the yaw cut
  is tight (|yaw| <= 35, tightened from 60), not loose.
- The "not a miss" figures above were measured with the preview's in-focus
  probability cut (0.90, since removed) in the rule. The
  full-eye-sharp-only plan replaced that cut with `full_eye_sharp >=
  SOFT_BELOW` and Low with `full_eye_sharp < LOW_BELOW`; its evidence is in
  `docs/plans/20261115-full-eye-sharp-only/` (`experiment.md`,
  `low-threshold.md`). The native `mark.rs` mirrors `SOFT_BELOW` and
  `LOW_BELOW` as `GOOD_FULL_EYE_SHARP` and `LOW_FULL_EYE_SHARP`, each pinned
  by a test (`fullsharp.rs` against the figures, `mark.rs` against the core
  constants).
- Burst-level technical features do not predict a kept scene: burst length
  does (>= 15 frames: 79% hold a pick). Do not rebuild a burst "keep" or
  "best" mark from sharpness / eyes / pose. The first frame of a burst also
  out-ranks any feature.
- Source: `docs/plans/_archived/20261008-burst-keep-score/learnings.md`,
  Steps 3, 5, 5b, 7.

### A learned discard score over the stored technical features did not hold on fresh folders (Hit)

The plan trained scores (`LR/T`: logistic regression over the preview's
in-focus probability `ef`, sharpness, EAR and pose, with and without the
`af-whole` fill) on the user's 1,200 blind "fine to sort last" labels, then
confirmed the winner on 600 fresh labels from six unseen folders. `LR/T` won
the exploration and failed the confirmation: AUC - `ef` -0.013
[-0.048, +0.023], with the `PL <= 5%` bar failing at q10 and the
model-contribution bar failing at q5 and q10. The full-eye-sharp-only plan
later removed `ef` from the app, so the plan's readings of `ef` itself (its
q5 `PL`, the frames without it) are history.

- Do not retrain a model on the stored technical features (sharpness, EAR,
  pose) to rank frames for discard. Fitting looked better in the exploration
  and did not hold on fresh folders.
- Judge a sort-last or discard cue at q5. At q10 every candidate lost more
  than 5% of the picks. The strata and bars are in the plan. A candidate's
  bar 5 (AUC against `ef`) is the only one that measures the model's own
  contribution. A clause that `ef` also meets (for example "close") says
  nothing about the model.
- Source: `docs/plans/_archived/20261023-discard-model/learnings.md`, Steps 6
  and 7 (Decision); `discardmodel.md`, "Results of the confirmation".

### Writing the `Both` sidecar format: one comparator, one stat source (Hit)

`sidecar::newest` is the single newest-wins comparator (larger `mtime_ns`,
ties go to whichever kind is listed first in `kinds()`, i.e. XMP). `write` uses
it to pick the label kept when `label_known` is false and the stat handed to
`mark_written`; `sidecar::reconcile_of` uses the same comparator to pick which
file's stat is compared against the stored one. Keep any future multi-file
sidecar logic going through this one comparator rather than re-deriving
"newest" locally, or a reopen right after writing can re-parse a file that
was just written.

- The per-kind write body is `write_kind`; the final stat always comes from
  `index::stat`, the same function the folder listing uses to compute
  `mtime_ns`, so the write path and the listing path cannot drift in how they
  measure a file's freshness.
- Adding a new `SidecarFormat` or kind means updating every exhaustive
  `match` on it; the compiler catches most, but at least one test's match arms
  needed a manual addition.
- Source: `docs/plans/_archived/20260925-sidecar-format-both/learnings.md`,
  Step 1.

### Folder-index eviction: what it holds and what shrinks the file (Measured)

`Index::evict` runs once at launch on a plain `std::thread` and holds the
writer lock for the whole evict + `VACUUM`; the native app's first scan joins
that thread before it reconciles (see `native-app.md`). The size cap counts
pages in use (`page_count - freelist_count`), since a delete only moves pages
to the freelist until `VACUUM` runs.

`Index::clear` (behind the settings' Clear Cache) is the second caller of
`evict_folder`. A running scan is not canceled for it: the native app holds
the clear for the scan's end (`cache::ClearFlow`).

- Measured (`Index::clear`'s unit test, 2 folders x 5 rows of a 200KB
  thumbnail): `VACUUM` alone gives nothing back to the file system, because
  the rebuilt database is written through the WAL — the on-disk footprint
  (`index.sqlite` + `-wal`) went from 2,121,808 B to 2,212,448 B, i.e. it
  *grew*, though `page_count` dropped by more than 10x. Ending with
  `PRAGMA wal_checkpoint(TRUNCATE)` took it to 40,960 B. Any eviction path
  that is meant to shrink the file needs that checkpoint after the `VACUUM`.
- `rusqlite`'s `pragma_query(None, "wal_checkpoint(TRUNCATE)", ..)` compiles
  but silently does nothing (it quotes the argument, so `TRUNCATE` is lost);
  use `query_row("PRAGMA wal_checkpoint(TRUNCATE)", ..)` instead. A
  `SQLITE_BUSY` from it is fine to just log and continue.
- Measured (M3 Pro, release, ignored test `vacuum_cost_on_a_100_mb_index`):
  evicting half of 5000 rows of 20.8KB thumbnails (106MB in use) and vacuuming
  took ~220ms, leaving 53MB. That a WAL reader does not error during `VACUUM`
  is reasoned, not covered by a dedicated concurrent test.
- Source: `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 3;
  `docs/plans/20260920-clear-index-cache/learnings.md`, Step 1.

### A scan's progress carries flushed paths, not a done-index high-water mark (Inferred)

`done` cannot tell the app what became readable: `run_scan`
(`crates/backend/src/index.rs`) drives `extract_all` on rayon workers that finish
in no particular order, `done` is an unordered `AtomicUsize`, and rows reach
the index in `BATCH`-sized transactions, so a file counted in `done` is not
necessarily answerable by `Index::thumbnail` yet. The only promise the backend
can make is "these paths have been committed since the previous report",
which is what the progress callback's `ready` carries. A path is pushed onto that
list only after its batch's `write_batch` returned `Ok`, which is why the
trailing flush after `extract_all` (the last partial batch, on a normal finish
and on cancel alike) needs a final unconditional `progress` callback —
otherwise those paths would never be reported.

- Source: `docs/plans/20260921-scan-progress-ready-paths/`, Steps 1-2.

### The scan pool's priority levels, and the hot list a stale scan drops (Inferred)

`handpic_core::scan::for_each_path` builds its rayon pool with a
`spawn_handler` that lowers each worker's OS priority once, before rayon's
`run`. `run_scan` and `run_faces_scan` take the priority as a parameter: the
native app (`scan::Priorities::APP`) gives `run_scan` `Priority::BelowNormal`
(Windows `THREAD_PRIORITY_BELOW_NORMAL`, macOS `QOS_CLASS_UTILITY`, Linux nice
of at least 5) and `run_faces_scan` `Priority::Lowest`
(`THREAD_PRIORITY_LOWEST`, `QOS_CLASS_BACKGROUND`, nice of at least 10), its
UI tests `Priority::Normal` to both; the CLI passes
`Priority::Normal`, which makes no call, so its numbers stay comparable with
`docs/humans/performance.md`.

- Why lowered at all: priority only matters under contention, so an idle
  machine still gives every worker a full core, while the viewer's preview
  read (normal priority) and the UI win when the user pages during a scan.
- Why not `THREAD_PRIORITY_IDLE` / `QOS_CLASS_BACKGROUND` for the first pass:
  a disk-bound pass at idle priority can starve behind any other process,
  and on macOS `BACKGROUND` also throttles disk IO. The first pass reads the
  files cold, so it stays at `BELOW_NORMAL` / `UTILITY`; only the analysis
  pass, whose reads follow the first pass's, takes `BACKGROUND`. If a Mac
  shows that pass crawling, move it to `UTILITY` too. On the macOS CI runner
  (one scan worker) one tiny DNG's analysis took 3.2 s to 8.9 s at `Lowest`
  against 0.5 s to 0.9 s at normal priority, and pass 2 of 2 to 4 files 15 s
  to 44 s (see `docs/plans/20261119-uitest-macos-timeout/`); a real Mac is
  not measured yet.
- `thread-priority` has no macOS QoS API, so it is a Windows-only dependency
  and the unix branches call `libc` directly (`pthread_set_qos_class_self_np`,
  `setpriority(PRIO_PROCESS, 0, ..)`, which on Linux sets the calling
  thread's nice alone). A failure to lower the priority is not an error: the
  pass runs at normal priority and `extract_all` / `extract_analysis_all`
  return the message as `Ok(Some(msg))`, which the app logs once per pass.
- Each pass builds its own `WorkQueue`; both read their hot list from the
  one `ScanFocus` handle the scan was started with. The app hands that handle
  the current file and the strip's range before the scan starts and again
  after the view moves (`native-app.md`, "The hot list is set before the
  scan's workers start"). The paths must be the listing's strings, built
  with the same `to_string_lossy` as the scan's `FileStat` paths, or the
  queue will not match them.
- `for_each_path` starts its worker loops with `ThreadPool::broadcast`, not
  `(0..threads).into_par_iter()`: `broadcast` runs the loop exactly once per
  pool thread, while a `par_iter` over the thread count may run two items on
  one thread (the second finds the queue empty) and leave a thread idle. The
  queue's lock is held only for `take_next`, never across `per_file` /
  `on_item`; `ScanFocus::set` takes only the focus lock, so the two cannot
  deadlock.
- Source: `docs/plans/_archived/20260930-scan-priority/learnings.md`, Steps 4-6.

### The scan's reads are capped and yield to the preview's at the disk (Measured)

Every scan read goes through one process-wide `handpic_core::scan::ReadSlots`
(the native app's, built with `scan::READ_SLOTS`): at most 2 workers are inside a
file read at once, on any of the three passes, while decoding and the models
stay on each pass's threads (`scan_threads()` for pass 1, `faces_threads()`,
an eighth of the cores, for pass 2 and `embed_threads()`, a quarter, for
pass 3). The app's preview read holds `pause()`, so no worker starts a new
read while it is held. Pass 2's
workers also wait out a held pause at their stage boundaries (after the read,
the decode, YuNet, the face mesh, the sharpness score and the full-size read;
`scan::yield_to_pause`, one `MAX_PAUSE` budget per file). The native app
holds the pause until its image is painted (`native-app.md`); the earlier app
held it only across the read, which the numbers below measure. The CLI and
the one-file `extract` functions keep an uncapped, never-paused `ReadSlots`,
so `docs/humans/performance.md`'s numbers stay comparable.

- The wait follows the reads in flight (the cap, at most `scan_threads()`) x ms/file: a preview read queues behind the reads
  already at the disk (uncapped, 22 x 7 ms = 155 ms; at a cap of 2, 2 x 7 ms). So the cap gives most of the gain
  and the pause little. Cold first pass of ARW files on a SATA SSD, 22 threads,
  the app closed:

  | Cap | Pause | Read p50 (ms) | Read p95 (ms) | ms/file |
  | --- | ----- | ------------- | ------------- | ------- |
  | 22  | off   | 154-156       | 159-163       | 7.2-7.3 |
  | 22  | on    | 128-129       | 137-140       | 6.5-6.6 |
  | 8   | on    | 29-57         | 46-62         | 7.5-8.1 |
  | 4   | off   | 27            | 38            | 8.0     |
  | 4   | on    | 20-25         | 30            | 7.1-8.0 |
  | 2   | on    | 14-16         | 18-19         | 6.9-7.3 |
  | 1   | on    | 11            | 18            | 14.0    |

  Cap 2 has no pause-off run; cap 4 pause-off and cap 1 have one run each.
- The 1.3 s (42.8 ms/file) first measured was inflated by the debug app reading
  the same disk. Close the app before measuring. The harness,
  `preview_during_pass_1_on_a_real_folder`, was a test of the earlier app's crate and
  left with it.
- The cap was measured on a SATA SSD only. NVMe is untested (no RAW folder on
  it), and a drive that needs deep queues for throughput may lose some at a cap
  of 2. HDR PQ CR3 is untested too.
- The cap also covers `hevc::to_jpeg` until the read is split from it:
  `read_preview` ends in that decode (65-125 ms of CPU per HDR PQ CR3), so at a
  cap of 2 such a folder pays a calculated 33-63 ms/file, and a preview read
  of such a file pauses the scan for the decode as well.
- `MAX_PAUSE` (1 s): a worker honours a pause for at most that long from when
  it first sees one, then reads anyway, so a stuck or leaked pause never stalls
  the scan. The worst case is about 1 s per `acquire` (one per pass 1 file, two
  per pass 2 file) plus 1 s for all of a pass 2 file's stage waits, so about
  3 s per pass 2 file. A wait for a
  permit alone does not use that clock, or a small cap would let workers read
  through a live pause.
- Cancel is polled inside the wait (`CANCEL_TICK`, 50 ms) because the cancel
  flag is a plain `AtomicBool` with no notifier and the scan's cancel has no
  handle on the slots. `acquire` then returns `None` and the worker abandons
  the file, as `extract_unless` does at its other stages.
- Source: `docs/plans/_archived/20261107-preview-read-gate/learnings.md`, Step 2.

### Scan, on-demand faces and eyes: timings taken through the earlier app (Measured)

These three were measured through the earlier webview app, but the Rust code
they time is what the native app still runs: the scan's pass 1
(`index::run_scan`), `ondemand::read_faces` (then the `faces_of` command) and
`ondemand::read_eyes` (then `eyes_of`). They moved here from
`docs/humans/performance.md` / `.ja.md`, which keep only measurements a reader
can repeat with the native app or the CLI.

- **Real-folder first scan, Windows (2026-09-21).** Release build, Windows 11
  Home, real folders of Sony ARWs on a DRAM-less QLC SATA SSD (Crucial BX500
  4TB), 22 extraction threads, a different folder per run; from that app's
  `scan prepare` (the listing, reconcile and sidecar lines together) and
  `scan extract` log lines:

  | Run | files | prepare | extract | total | per file | files/s |
  |-----|-------|---------|---------|-------|----------|---------|
  | Cold first scan, Defender real-time protection on | 2677 | 1427ms | 42509ms | ~43.9s | ~16ms | ~61 |
  | Cold first scan, folder excluded from Defender | 3401 | 639ms | 65605ms | ~66.2s | ~19ms | ~51 |
  | First scan, warm page cache (index cleared, files read moments before) | 2134 | 47ms | 2084ms | ~2.1s | ~1ms | ~1000 |

  Both cold runs finished with 0 errors. Excluding the folder from Defender
  did not make the scan faster, so Defender is not the cause; the warm run
  shows the CPU side costs ~1ms per file, so cold IO dominates. Extrapolated
  to 5000 files, a cold first scan takes ~82-97s; at a 1MiB bounded read per
  file and ~16ms per file, the effective throughput works out to ~63MB/s
  (derived, not measured). The thread-count sweep that pins this on the drive
  (2026-09-22, `handpic-cli scan`) stays in `performance.md`.
- **`faces_of`, now `ondemand::read_faces` (2026-09-25).** Read the preview +
  `detect_around`. Linux WSL2 (24 hardware threads), release build, the
  command's body called directly on two α7 V ARWs read from the Windows NTFS
  drive, warm page cache, 20 calls each after a first one:

  | File | First call | Mean | Median | Max |
  |------|------------|------|--------|-----|
  | `_DSC3113.ARW` (1616x1080 preview, 3 faces) | 70.5ms (with the model build) | 29.9ms | 29.9ms | 34.1ms |
  | `_DSC1942.ARW` (1616x1080 preview, 3 faces) | 34.0ms | 30.6ms | 29.7ms | 37.4ms |

  The first call of a session also builds the model. A cold read from a card
  or a slow disk, and the redraw, are not in these numbers. The timing ran
  from a temporary `#[ignore]`d test removed before committing.
- **`eyes_of`, now `ondemand::read_eyes` (2026-10-07).** Windows 11,
  release build, one thread, `main` at `392f9c25`, the
  `#[ignore]`d `times_eyes_of_on_real_files` test (it left with the earlier
  app), run with no app running; the first file of each set left out because
  it pays the plan builds (256ms on the ARW, 315ms on the DNG). The call then
  fed the meta pane's `Eyes open: NN%` row:

  | Files | Read | Decode | Detect | Model | Total |
  |-------|------|--------|--------|-------|-------|
  | 17 α7 V ARWs with an AF point and a face judged (of `_DSC1881`-`_DSC1899` in `2026-09-19`) | 0.8-6.1ms | 10.6-11.7ms | 27.4-32.7ms | 35.8-46.8ms | 80.2-91.9ms |
  | 11 M11-P DNGs without one (`L1005148`-`L1005191` in `2026-02-01`, a face judged in each) | 7.0-9.7ms | 19.5-27.1ms | 73.8-80.0ms | 36.7-39.9ms | 139.9-156.1ms |

  The two other ARWs had no face to judge and returned after the detection in
  43.9 / 45.5ms. On the AF path one full-size decode serves both the
  detection and the crop; without an AF point the decode is the 3/8 one of
  the whole-image search plus a full-size one for the crop, and the
  whole-image detection, not the second decode, is most of the cost. Every
  file of the two sets labeled closed came back closed.

  The head pose's cost, the same test on the same files, 2026-10-07, `main`
  at `9048e952` against `bde23660` (the commit before the pose), built and
  run back to back on the same machine, the first file of each set left out:

  | Files | Total before | Total after |
  |-------|--------------|-------------|
  | 16 ARWs with a face judged | 60.7-73.1ms (median 65.3) | 61.4-68.2ms (median 65.9) |
  | 2 ARWs without one | 32.8 / 33.5ms | 33.3 / 33.8ms |
  | 10 DNGs | 109.2-119.1ms (median 116.0) | 110.8-120.7ms (median 117.1) |

  The difference is run-to-run noise: the solve is three orders of magnitude
  smaller, and the detection, which the pose does not touch, moved as much.
  Both runs were faster than the table above; a first run of the after build
  straight after compiling it read 81-92ms and 144-160ms.
- Source: `docs/humans/performance.md` before the remove-tauri plan
  (`docs/plans/20261118-remove-tauri/plan.md`, Step 5).

### The third pass: `embeddings` is its own table, and the pass is not "scanning" (Inferred)

`index::run_embed_scan` embeds each RAW file's preview with the SigLIP 2 model
(`handpic_core::embed`) into the `embeddings` table and reports its progress
(`done`, `total`). It shares the scan's cancel flag and `ScanFocus`, at
`Priority::Lowest`, on `embed_threads()`. It runs only when `aiEnabled` is on,
the model file is in place and `embeddings_missing` lists something; a
JPEG-only (view-only) folder never lists anything. Only the earlier app ran it
(and the ranking below); the native app runs passes 1 and 2 only (a
`todo.md` item covers this backend code).

- **A table of its own.** `write_batch` does `INSERT OR REPLACE` of whole
  `files` rows, so a column on `files` would be wiped by every first-pass
  rewrite. `embeddings(path, dir, size, mtime_ns, version, vec)` carries a
  `dir` column (not in the plan's sketch) so `evict_folder`, `rename_dir`
  and `reconcile` treat it like `files` and `ratings`. A row is valid while
  `version` is `EMBED_VERSION` and `size` / `mtime_ns` match the `files`
  row; `vec` is little-endian f32. `NULL` marks a file that could not be
  embedded, which is not retried until its stat or the version moves.
  `reconcile` drops the embeddings of files gone or changed. A row
  re-extracted for an `EXTRACTOR_VERSION` bump keeps its embedding. Bump
  `EMBED_VERSION` (`crates/core/src/embed.rs`) on any change to the model,
  its preprocessing or its input decode.
- **Not scanning.** The pass may embed for minutes after pass 2 ends, so an
  app must not treat it as a scan: the operations held for a scan's end
  (trash, renames, Clear Cache) run while it goes on, and the pass is
  canceled before a clear or a rename deletes or moves rows. A batch already
  in flight can still land after them. `evict` and `clear` drop the
  embeddings of any `dir` without a `folders` row (`drop_stray_embeddings`),
  and `reconcile` drops a renamed file's old path, but that does not cover
  everything: a late batch under a folder whose `folders` row survives a
  clear (it has dirty ratings) stays, and so does the old path after a file
  rename until the next reconcile. The deferred guarded insert in
  `write_embeddings` (see `learnings.md` of the plan) would close this.
- **The model is downloaded, not bundled** (the plan's Decision 1).
  `ai::Ai`'s switch downloads `embed::MODEL_URL` on a plain thread through
  `reqwest::blocking` when `aiEnabled` turns on and the file is missing.
  Each `install` call writes its own `MODEL_FILE.part-{n}` file (a shared
  name let a canceled download delete the new one's file; leftover
  `MODEL_FILE.part*` files are removed at start), checks `MODEL_SIZE` and
  `MODEL_SHA256`, and only then renames the file into place. Each `AiState`
  goes to a callback. reqwest is built with `rustls-no-provider`, so `fetch`
  installs rustls' `ring` provider first when none is set. Its blocking
  reads time out after 30 s without data, not on the whole 378 MB.
- **Genres are derived, not stored.** `handpic_core::genre::genres_of` scores
  a folder's valid embeddings (`Index::embeddings`) on every call, so a new
  genre data file never needs a rescan; a file without a valid embedding
  counts as `Unknown`.
- **`output::outputs_in(dir, files)` feeds a destructive filter, so it never
  guesses.** The `No output` filter section can be piped straight into
  `Move to Trash…`, so a wrong `no` trashes a file that has an export.
  `output.rs` therefore does not reuse `read_listing` or `listing::is_file`,
  which swallow per-entry errors (`entries.flatten()`, `kind` returning
  `None`); it propagates every read error. A missing `dir` is `Ok(None)`, any
  other failure to read it is `Err`, and a dangling symlink named `Output` is
  `Err`. The app turns an `Err` into "no answer" and shows the failure as a
  note, so an unreadable export folder never fails the folder open. Known
  over-matches (`X_1.jpg` also counts for `X.DNG`; any extension counts) are
  accepted because they only keep a file out of `No output`. The symlink
  tests are `#[cfg(unix)]`, so Windows does not run them.
  Source: `docs/plans/_archived/20261106-filter-by-output/learnings.md`, Step 1 and Step 2.
- **The ranking is derived too; only its weights are stored.**
  `Index::rank_rows` reads every `files` row of a folder with its flag,
  capture time, the pass-2 values when the row is at `FACES_VERSION`, and the
  valid embedding, and `handpic_core::rank::rank` ranks each burst on every
  call. `rank.rs` forms the bursts as the native `burst.rs` does (capture
  order, 1000 ms gap). The shared weights and the per-genre offsets
  (`offsets`, one 44-vector per top-level genre of `genre::top_level`;
  `rank` adds the burst's `genre::of_burst` offset, none for `Unknown`;
  `Weights::of` itself ignores `genre::UNKNOWN`, so a `Weights` that carries
  an `Unknown` key still ranks unknown bursts with the shared weights) were
  stored by the earlier app; without them the prior
  (`crates/core/ranking/siglip2.json`, written by the plan's
  `rank_prior.py`, no offsets) ranks. `rank::fit` refits on
  `Index::rank_dirs` (the folders holding a `views` row or a reject,
  `RANK_DIRS`). The fit keeps the prior's PCA basis and standardization and
  pulls the weights toward the prior's at the prior's own `1 / (2 C)`, and
  each genre's offset toward zero at `OFFSET_SHRINK` times that, so a genre
  with few judgments ranks as the shared weights do; the stored weights are
  44 plus 44 per genre, not a basis. `evict`'s 30-day age rule skips a
  folder `RANK_DIRS` lists (its rows are the fit's training data; the one
  SQL fragment keeps the exemption and the training set from drifting
  apart) while its size cap does not, and `clear` drops everything. A
  pick-only folder is not read (no negative to pair against) and so not
  spared.
- **`views` feeds the fit per file.** The app records a file as viewed once
  its decoded preview has stayed on screen for `VIEW_DWELL_MS` (500 ms; the
  native `viewed.rs`'s `Dwell`), measured from the paint, not from the page
  turn: the preview read and decode take 10-100 ms and a held key repeats
  every ~30 ms, so a clock started at the turn would count frames whose image
  never reached the screen. A failed preview does not count: the user saw an
  error, not the photo, and such a file has no embedding to rank. Nor do
  strip thumbnails or a view-only folder. `Index::rank_rows` joins `views`,
  so `RankRow.viewed` becomes `Labeled.viewed`: an unflagged frame is a
  negative (weight 1) only when its file was viewed, a reject
  (`REJECT_WEIGHT`) whether viewed or not, and an unflagged unviewed frame is
  ignored. This replaced the folder-level `Culling Finished` mark (schema
  v21 / v22, dropped at v23), which let one unviewed frame keep a whole
  folder out and a hand-set mark count frames never shown. `views` is its
  own table, not a `files` column, because `reconcile` deletes and pass 1
  `INSERT OR REPLACE`s a `files` row on any size / mtime /
  `EXTRACTOR_VERSION` change; it survives eviction, `rename_dir` /
  `rename_file` carry it, and only `clear` drops it. Since it is never
  pruned, a viewed folder is spared by the age rule for good; the size cap
  and `clear` still reclaim it. Test fixtures for this: `Index::set_rating`
  always writes `dirty = 1` and `evict_folder` keeps dirty `ratings` rows, so
  a reject the eviction may drop needs a clean `ratings` row inserted with
  SQL; `Index::clear` only walks `folders` rows, so a `write_batch` without a
  `folders` row (no `reconcile` / `set_opened_at`) survives a `clear`; and to
  show that eviction keeps `views`, evict through the size cap
  (`max_bytes: 0`), since the age rule now spares a folder holding a `views`
  row.
- **Threads: a quarter of the cores** (`embed_threads()`), not pass 1's
  `scan_threads()` (Measured; pass 2 runs on fewer still, an eighth,
  `faces_threads()`, since the GPUI port measured it costing paging +9.4 ms
  on ARW and +89 ms on RAF at `scan_threads()` and +13 ms on RAF at a
  quarter, see `native-app.md`). Priority does not keep the viewer's reads
  fast against 22 busy workers on 24 logical cores: shared physical cores
  (SMT) and an all-core clock do not care about thread priority. On
  `2026-09-19` (2134 ARW), a preview read + full decode on a
  normal-priority thread took a median 10 ms idle and 27-34 ms during pass 2
  on 22 threads. During pass 3 it took 33 ms on 22 threads (41 s for the
  pass), 22 ms on 11 (56 s) and 15 ms on 6 (70 s). Memory does not set the
  count: the model's load sets the process peak (745-812 MB in that run),
  and the workers' buffers add about 220 MB from 1 to 24 threads
  (`handpic-cli embed`). The harness, the ignored test
  `embed_pass_on_a_real_folder`, was in the earlier app's crate and left with it.
- Source: [image-embeddings learnings](../plans/_archived/20261012-image-embeddings/learnings.md),
  Step 3.

### `crates/core`, `crates/backend` and `crates/cli` are on Rust edition 2021: no `if ... && let` chains (Hit)

`if let Some(x) = a && let Some(y) = b` does not compile there; nest the
`if let`s. `crates/native` is on edition 2024, where the chain compiles, so
code moved from it into the backend may need its chains nested.

- Source: `docs/plans/_archived/20260920-app-quick-fixes/learnings.md`, Step 2.

### The MCP server runs on the caller's tokio runtime; `rmcp` 3.4 macro traps (Hit)

The backend's `crates/backend/src/mcp.rs` serves `rmcp`'s
`StreamableHttpService` through `axum::serve`, spawned with a bare
`tokio::spawn`, so `Mcp::switch` and `Mcp::stop` must run inside a tokio
runtime: the native app's `mcp.rs` owns a current-thread runtime on its one
`mcp` thread (see `native-app.md`). `tokio` is a backend dependency for its `net`,
`rt`, `sync` and `time` features, and the backend's tests drive the server on
a current-thread runtime of their own, so no tokio `macros` feature.

- `#[tool_router]` on an impl with no `#[tool]` fn does not compile in 3.4.1;
  an intentionally empty router needs `#[tool_router(allow_empty)]`.
- `#[tool_handler]` without `router = ...` calls `Self::tool_router()` per
  request, so the handler needs no `tool_router` field (an unread one trips
  `dead_code` under `-D warnings`).
- `ServerInfo` is deprecated in 3.4 in favor of `ServerConfig`.
- `CallToolResult::structured(value)` already adds a text block with the same
  JSON; do not add another. A failure the client should see is
  `CallToolResult::error`, not a protocol error.
- Tool arguments are `Parameters<T>` with `#[derive(JsonSchema)]` and
  `#[schemars(crate = "rmcp::schemars")]`, so no direct `schemars` dependency.
  To tell an omitted field (keep) from `null` (clear), as `set_judgment`'s
  `label` does, use `Option<Option<T>>` with a `deserialize_with` helper.
- `stop` bounds the graceful shutdown with a 2 s timeout and then aborts the
  task, so a lingering SSE stream cannot hold up the exit. The status is a
  `tokio::sync::Mutex` held across the bind, so two quick toggles cannot bind
  twice; a failed bind (port in use) only sets the `error` of the returned
  `McpState`.
- Source: `docs/plans/_archived/20260924-mcp-companion/learnings.md`, Step 1.

### The MCP bridge: never hold the status lock, always reply (Inferred)

MCP tools that need view state ask the app: `Bridge` hands each tool's
`Request` (`{ id, kind, args }`) to the app's send callback, waits up to 5 s
(`BRIDGE_TIMEOUT`) on a `oneshot` keyed by the id, and the app's `reply`
completes it.

- The pending senders live in `Bridge`, beside (not inside) the server's
  status mutex, so `reply` never waits on a toggle that holds that mutex
  across a bind or the 2 s shutdown wait.
- `Bridge` takes its `send` as a closure, so the tests build it without an
  app; `Companion` holds the bridge and the index reader for the same reason.
- The app must answer every request, a failure included (`ok: false`); an
  unanswered request costs the client the whole 5 s timeout.
- The folder the app reports is the one picked, not canonicalized, while
  the paths come from the canonical folder: compare canonicalized parents to
  decide "in the open folder", and use canonical parent + file name as the
  index key.
- A tool's write goes through the same path a key press does, so it writes
  the same sidecar bytes (the native app's `set_judgment`, see
  `native-app.md`).
- Source: `docs/plans/_archived/20260924-mcp-companion/learnings.md`, Steps 2-5.

### The folder watcher cannot loop on the app's own sidecar writes (Inferred)

`crates/backend/src/watch.rs` watches the open folder as the listing
canonicalizes it (non-recursively, so the watched folder can never diverge
from the indexed one) and calls its callback 500 ms (`DEBOUNCE`) after the
last event of a burst; the native app re-lists the folder when it is still
the open one (`resync.rs`). Two things keep the app's own sidecar writes from feeding
back into it:

- `triggers()` drops an event whose paths are *all* sidecars (`*.xmp`,
  `*.arw.dop`, either format, case-insensitive) or `handpic_core::copy::TEMP_SUFFIX`
  temporaries, which is exactly what `sidecar::write` produces and what the
  import's copies produce (`<name>.handpic-tmp` in the date folder, renamed to
  `<name>` when complete). So a rating
  does not cost a rescan at all while culling.
- Even unfiltered it would terminate: `Index::reconcile_sidecars` reparses a sidecar
  only when its on-disk stat differs from the one `mark_written` stored, so a
  rescan after the app's own write queues nothing.

The price is that an *external* sidecar edit (PhotoLab writing a `.dop`) is not
picked up live; it lands on the next re-listing (a reopen or the tree's
`Refresh`).

On Windows, `ReadDirectoryChangesW` keeps a handle on the watched directory,
which pins the watched folder's ancestors against rename while Handpic has it
open; renaming the open folder itself succeeds (measured in the next entry).
Whether the watch blocks deleting the folder was not measured (an open item in
`todo.md`). `set` drops the previous watcher before creating the new one, so
leaving a folder releases it. A watch that cannot be set (SMB, say) is
`log::warn!`ed and ignored: the open must not fail, and a re-listing still
picks up what changed.

### A `notify` watch on Windows pins the watched folder's ancestors, not the folder itself (Hit)

Measured with `notify` 8.2 on Windows 11 (`RecommendedWatcher` =
`ReadDirectoryChangesWatcher`, `RecursiveMode::NonRecursive`): renaming a
watched folder itself **succeeds**; renaming a folder that has a watched
**descendant** fails with `PermissionDenied` (os error 5). So a watch's
`ReadDirectoryChangesW` handle blocks renaming any ancestor of the watched
path, not the watched path itself.

`crates/backend/src/treewatch.rs` uses this to watch the whole folder tree with
one `RecommendedWatcher`, `watch`/`unwatch` per expanded folder, mapped
through a shared canonical-path -> tree-key list (several tree paths can
canonicalize to the same folder, e.g. a symlink; the last key's `unwatch`
is what actually drops the OS watch). Renaming a folder that has a watched
descendant needs `release_under` (unwatch everything at or below the
renamed path) before the rename and `restore` after — see
`treewatch::with_released`, which holds the `TreeWatch` lock for the whole
release/rename/restore-on-failure sequence so a concurrent
`TreeWatch::set` cannot re-watch the path mid-rename. The notify event
handler only ever takes the separate canonical-path map's lock, never the
`TreeWatch` state lock, so this cannot deadlock.

Releasing a watch is **asynchronous** on Windows: in notify 8.2's
`ReadDirectoryChangesWatcher`, `watch` waits for the server thread's ack,
but `unwatch` only queues `Action::Unwatch` and `Drop` only queues
`Action::Stop`; the `CancelIo` + `CloseHandle` of the directory handle run
later on the `notify-rs windows loop` thread. So the handle is not released
the moment `unwatch` returns or the watcher is dropped (an earlier
measurement of 20/20 immediate renames was luck), and a rename issued at once
fails with `PermissionDenied` now and then (about 5 in 300 test runs under
full CPU load). `watch::settle` is the barrier: it calls
`watcher.configure(notify::Config::default())`, which goes through the same
action queue the server drains in order and blocks on its reply, so it
returns only after every earlier `unwatch` closed its handle (about 0.02 ms
when idle). `treewatch::State::remove` runs it after each real `unwatch`, and
`watch::release` unwatches the open folder and runs it before dropping the
watcher, so `rename_folder` needs no retry. `watch::set`'s plain drop of the
previous watcher (its comment says that releases the handle) is still only
eventually true; it is not on a rename path. A notify upgrade must re-check
that `configure` still queues behind `Unwatch` (loop the `treewatch` tests
under load).

Only rename was measured this way, not delete: don't assume the same
ancestor-pinning applies to deletion without measuring it.

Trashing (not renaming) a file inside a watched folder needs no such care: a
throwaway test put a `notify::recommended_watcher` `NonRecursive` watch on a
canonical temp dir (as `watch.rs` does for the open folder) and ran
`trash::run` with `TrashContext::default()` over a RAW and its `.xmp` inside
it — both went to the Recycle Bin with no failure, so trashing the rejects of
the open (watched) folder needs no watcher change, and a folder that is not
open has no watch at all.

- Source: `docs/plans/_archived/20260928-tree-live-watch/learnings.md`, Step 1;
  the asynchronous release is from
  `docs/plans/_archived/20260929-treewatch-unwatch-barrier/learnings.md`,
  Step 1; the trash check is from
  `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`, Step 1.

### `Writer::set` / `set_now` carry `#[allow(clippy::too_many_arguments)]`, not a parameter struct (Hit)

Adding the `LabelNames` snapshot pushed `Writer::set` / `set_now` to 8
parameters, over clippy's `too_many_arguments` limit of 7 (`-D warnings` in
CI). The fix was `#[allow(clippy::too_many_arguments)]` on both, not a new
parameter struct — kept the change small and matches how this pair already
carries `format`. Expect the same call when adding another per-write field
here; every `sidecar.rs` test call site needs updating too.
The scan passes follow the same rule: `run_scan` and `run_faces_scan` went
over the limit (8/7) once they took the `ScanFocus` handle and got the same
`#[allow]`.

- Source: `docs/plans/_archived/20260922-lightroom-label-names/learnings.md`, Step 2.
- Source: `docs/plans/_archived/20260930-scan-priority/learnings.md`, Step 5.

### Clearing a rating writes `Rating = 0`, not "no rating", in both sidecar formats (Hit)

There is no way to represent "unrated" as an absent field once a sidecar
exists: `xmp::write_rating(None)` and `dop::write_rating(None)` both write
`Rating = 0` (see the doc comments in `crates/core/src/xmp.rs` and
`crates/core/src/dop.rs`), and the readers return `Some(0)`, which the app
already treats as unrated. Don't expect `read_rating() == None` after a clear
on a file that already has a sidecar — assert `Some(0)` instead.

- Source: `docs/plans/_archived/20260922-clear-all-flags/learnings.md`, Step 1.

### Every JPEG the reader hands out is orientation-neutral by construction (Hit)

The orientation the parsers report is the only rotation an app should apply.
Every JPEG `handpic_core::reader` hands out (`read_preview`, `read_full`)
leaves through one exit that rewrites its Exif IFD0 Orientation to 1 in place
(`jpeg::neutralize_orientation`), so no container can add a second rotation,
and a parser added later is covered without touching the app.

- What broke: a portrait GFX 100 RAF (Orientation 8) showed upright in the
  filmstrip but sideways in the main preview. The RAF's embedded JPEG carries
  a full Exif APP1 with its own Orientation (so does a plain JPEG, which is
  its own preview), the earlier app's webview decoder rotated it once at
  decode, and the app rotated it again by the reported orientation. The
  original design assumed an embedded preview is a bare JPEG stream without
  Exif, which holds for ARW only. The filmstrip was fine because the index
  thumbnail is re-encoded without Exif.
- The bytes are fixed rather than relying on a decoder option, whose
  behavior differs between decoders (it did across WebView2, WKWebView and
  WebKitGTK).
- `reader::tests::every_format_hands_out_jpegs_without_an_exif_rotation`
  pins it for every supported format; a new format gets an entry there.
- Source: `docs/plans/_archived/20260930-orientation-neutral-preview/learnings.md`.

## Windows shell

### On Windows, editing files with a Python heredoc can corrupt line endings and escapes (Hit)

A Python heredoc edit in text mode writes CRLF into this LF repository
(`core.autocrlf=false`), and a `'''...'''` string silently eats `\U` / `\\`
sequences — a risk for Windows-path test fixtures.

- Fix: open/write with `newline=""` to keep LF (or strip the `\r` afterwards),
  check with `git ls-files --eol`, and re-check backslashes in any
  Windows-path string literals afterward.
- Source: `docs/plans/_archived/20260929-tree-root-dedup/learnings.md`, Step 1.
- Source: `docs/plans/_archived/20261114-gpui-perf-overlay/learnings.md`,
  Step 1 (hit `crates/native/src/main.rs` and `scan.rs` too).

A related trap: a Python heredoc run through the Bash tool can also turn
escape text meant to stay literal (`\0`, `\x01` inside a byte-string
literal destined for Rust source) into real NUL / 0x01 bytes in the
written file, even when the heredoc is quoted (`'EOF'`). Build such bytes
with `bytes([0, 1, ...])` instead of escape literals, or make the edit
with the Edit tool rather than a Python heredoc. A Python script containing
nested `'''` strings can also fail outright in the Bash tool ("unexpected EOF
while looking for matching"); write the script to the scratchpad with the
Write tool and run that file.

A third failure: on this setup the Git Bash `python` / `python3` command can
be a stub, and a `python -` / `python3 -` heredoc hung on it twice without
running anything (the call ended up in the background). Kill the hung process
by its PID, not by image name. Use the venv's `python.exe` by full path on a
script file written with the Write tool, or make a text edit with the Edit
tool and no Python at all. To wait on a log a background job has not created
yet, use `tail -F` (not `tail -f`, which exits at once on a missing file)
bounded by `timeout N`, for example
`timeout 570 tail -F run.log | grep -m1 "done:"`.

- Source: `docs/plans/_archived/20261017-bad-photo-labels/learnings.md`, Steps 4 and 5.
- Source: `docs/plans/_archived/20261023-discard-model/learnings.md`, Steps 4 and 5.
- Source: [heif-cr3-message learnings, Step 1](../plans/_archived/20260930-heif-cr3-message/learnings.md#step-1); face-detection-recall-cost learnings, Steps 2 and 3.

### On Windows, the Bash tool halves doubled backslashes, even in a quoted heredoc (Hit)

Every `\\` in a Bash-tool command lands as `\` (single backslashes survive):
a `cat > f <<'EOF'` heredoc, quoted or not, a single-quoted `printf '%s'` or
`echo` argument, and a heredoc fed to `python -` alike. Text holding Windows
paths, verbatim `\\?\` / UNC prefixes or regex escapes comes out wrong with
no error; the Write tool writes the same text byte for byte.

- Fix: write text holding backslashes with the Write / Edit tools, or put it
  in a script file written by them and run that file; never pass it through
  a Bash command string.
- Source: `docs/plans/_archived/20260928-trash-rejected-from-tree/learnings.md`,
  Step 4; `docs/plans/20260929-todo-five-small-items/learnings.md`, Steps 4
  and 5.

## CI

### Cache Rust builds by `matrix.os`, not `runner.os` (Hit)

`macos-latest` (arm64) and `macos-15-intel` (x86_64) both report
`runner.os == macOS`; a cache key built from `runner.os` would let them
restore each other's `target`, which is wrong across architectures. Key on
`matrix.os` instead.

### release-please: the `rust` strategy fails on a workspace root with no `[package]` (Hit)

`release-please release-pr --dry-run --release-type rust` (v17.9.0) aborts
with `is not a package manifest (might be a cargo workspace)`, because
`CargoToml.updateContent` tries to update the root `Cargo.toml` as a package
even when it only declares `[workspace]`. Use `simple` with `extra-files`
pointing at each crate's `Cargo.toml` instead.

- For a `Cargo.lock` extra-file, `$.package[?(@.source === undefined)].version`
  is the working JSONPath discriminator for "this workspace's own crates" —
  equality on `@.name` matches nothing, and `startsWith`/`match` are rejected
  by jsonpath-plus's safe evaluator.
- `release-pr --dry-run` reads `release-please-config.json` from the **remote**
  branch, not the local working tree, so testing a not-yet-merged config needs
  a scratch clone/bare-repo setup with the config pushed to its `main`.
- Source: `docs/plans/_archived/20260918-github-releases-auto-update/learnings.md`,
  Step 4.

### A green branch-only CI run does not prove the merge result compiles (Hit)

Two branches can each pass CI alone and still break when merged, because
each branch's own copy of an import or `cfg` is still in scope on that
branch. In the earlier app, a macOS menu item added as a plain, un-`cfg`'d
`MenuItem` merged textually clean with a concurrent change that put
`MenuItem`'s import under `#[cfg(not(target_os = "macos"))]`, and the merge
result no longer compiled on macOS.

- Only building the actual merge/rebase result (not the branch tip) catches
  it. A single-OS failure right after a merge that looks like a runner flake
  is a signal to re-run it once, then check out the merge ref before
  assuming flakiness: GitHub builds the PR's merge commit, so line numbers
  and blame on the branch head alone can point at unrelated code.
- Give new platform-specific code the existing `cfg` split of its siblings
  up front.
- Source: `docs/plans/_archived/20260920-scan-timing-logs/learnings.md`,
  "Merge conflict with concurrently-landed macOS menu icon work".

### A `Release-As` footer must be on the squash commit itself, not a branch commit (Hit)

`Release-As: 0.4.0` on the last commit of a multi-commit PR branch did not
reach release-please: this repo squash-merges with
`squash_merge_commit_message: COMMIT_MESSAGES`, and for a multi-commit PR
GitHub appends a `---------` separator and the collected `Co-authored-by`
trailers after the concatenated messages, pushing `Release-As` out of the
final footer block. The release PR stayed at the old version after merge.

- Fix: squash-merge with an explicit body carrying the footer, e.g. `gh pr
  merge --squash --body 'Release-As: 0.4.0'`.
- When a PR needs `Release-As` and has more than one commit, don't rely on
  a commit message footer; set it on the squash merge itself.
- Source: `docs/plans/_archived/20260925-lightroom-layout/learnings.md`,
  "Release-As footer".

### Before deleting or renaming a guide, grep the whole tree, archived plans included (Hit)

Archived plans link to guides (`docs/plans/_archived/*/plan.md` linked
`tauri-app.md` and `ui-styling.md`), and `ci:local`'s lychee run fails on the
dead links. A plan's "nothing else links it" claim is not evidence. Run
`git grep -n '<guide>.md' -- docs` first. Turn each hit in an archived plan into
the plain name marked "removed since" rather than a link. The same applies to a
moved asset: a `git grep 'app/icons'` over the whole tree, not only the plan's
listed files, found `tools/package/linux.sh` reading an icon the plan missed.
Source: `docs/plans/_archived/20261118-remove-tauri/learnings.md`, Steps 1 and 4.

## Verification

### Write a performance number with its measurement conditions (Hit, twice)

Two performance claims here had to be walked back in review because they read
as general when they were not: a 30-second target that rested on warm `cp`
reads, and a batch of README numbers measured on folders of symlinks to a
single file.

- Why: a number without its conditions — page cache state, and whether the
  files were distinct or 5000 symlinks to one inode — is read as a general
  claim and copied forward as one.
- State what was actually measured next to the number, and say plainly what
  still needs a real-folder measurement.
- For a copy or write benchmark the destination is a condition too. A nearly
  full, DRAM-less SATA SSD lost its write cache under sustained writes, so
  throughput fell run after run with identical settings (21.6 then 8.1 MB/s)
  while the source read alone ran at 135 MB/s. Methods measured back to back on
  such a drive cannot be told apart. Compare on a destination faster than the
  source, wait or alternate the order, and do not time the source while the
  destination is still busy (read timings taken during a background copy came
  out at 16-22 MB/s). Source:
  `docs/plans/_archived/20261106-import-from-card/learnings.md`, Step 2.

### Run a multi-minute step in the background and poll it in short loops (Hit)

A single Bash tool call that blocks for several minutes (a feature dump of
11 minutes, a 1-hour index rebuild) trips the 600 s stream watchdog and stalls
the agent. Start the step with `run_in_background` (or detached) and poll its
output file with short bounded loops.

- Source: `docs/plans/_archived/20261012-image-embeddings/learnings.md`, Steps 2 and 7.

### Verify a platform workaround's threshold on the production code path (Hit)

Before shipping a numeric threshold for a platform workaround, measure it on
the exact code path production runs, not on a quoted or secondhand number or
a simpler reproduction.

- What broke: #383 shipped a 12 MP Linux preview limit for the earlier app
  (WebKitGTK drew a large transferred `ImageBitmap` transparent) from
  "12.3 MP drew, 16 MP did not", which was never measured on the shipped
  path. The user's manual check still showed a blank main view, and a
  follow-up had to lower it to 6 MP.
- How it was measured: a MiniBrowser bisect that reproduced the real path (a
  worker `createImageBitmap` with resize options, the bitmap transferred,
  `drawImage` to a canvas, the center pixel read), with each result POSTed to
  a local Python HTTP server so the run could be logged outside the webview.
  It put the threshold at ~6.87 MP by pixel count, independent of shape.
- Rule: build the harness around the same API calls and transfer steps the
  app uses, bisect the number, and write the harness down with the result.
- Source: [linux-preview-pixel-limit-6mp learnings, Step 1](../plans/_archived/20260924-linux-preview-pixel-limit-6mp/learnings.md#step-1).

### GUI automation does not work on this Mac (Hit)

`osascript` is denied assistive access: the app's window cannot be brought
forward, and injected keystrokes are dropped silently.

- Why: macOS privacy settings on this machine, not something the repo controls.
- Plan any GUI check as a **manual confirmation by the user**, and list exactly
  what they should look at. Report unchecked behavior as "not verified"; that
  is more useful than an implied pass.

### `cargo` is not on the Bash tool's PATH (Hit)

- Run it through `mise exec -- cargo ...` instead of bare `cargo ...`.

### A `#[cfg(unix)]` test is silently absent on Windows (Hit)

A `#[cfg(unix)]` test compiles and runs only on macOS/Linux CI; on a Windows
dev machine it's silently absent from the run (not a failure, not a skip
you'll see), so a passing `cargo test <name>` there proves nothing about it.
Confirm unix-only coverage on CI, not locally on Windows.

### A rating/pick test needs a scan-populated `files` row before `Index::entries` reads back (Hit)

`Index::entries` and `Index::entry` (`INDEXED_FILE`, `FROM files LEFT JOIN
ratings USING (path)`) read judgments through `files`, which only a folder
scan fills in, as does `rank_rows`. A unit test that calls `write_batch` (or a
sidecar format switch, `sidecar::switch_to`) and then reads the rating back
must first call `index::stat` to populate that row, otherwise the read
returns nothing regardless of whether the write succeeded.

- Source: `docs/plans/_archived/20260920-robustness-cleanup/learnings.md`, Step 4.
