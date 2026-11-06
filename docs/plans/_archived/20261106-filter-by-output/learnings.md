# Learnings

## Step 1

- `outputs_of` cannot reuse `read_listing` or `folders::is_file`: both swallow
  errors (`entries.flatten()`, `kind` returning `None`), so an entry that fails
  to read would silently drop an export and turn its RAW into `No output`.
  `output.rs` propagates every per-entry error instead.
- The match inverts the rule to stay linear: each export stem adds the RAW
  stems it can stand for to one set (its own, the stem minus a `_<digits>` or
  `-<digits>` suffix, and the text before every `-dxo`), then each RAW is one
  lookup. Every `-dxo` occurrence counts, not just the first, so a RAW whose own
  stem holds `-DxO` still matches its DeepPRIME export.
- Decisions the plan left open, all on the side of giving no wrong answer:
  - A `dir` that no longer exists is `Ok(None)`, since neither folder exists;
    any other failure to read `dir` is `Err`, because an export folder may be
    in it.
  - A dangling symlink named `Output`, or one inside an export folder, is
    `Err` rather than skipped. It cannot be told from an unreachable target.
  - On a case-sensitive file system every spelling is read, so `Output/` and
    `output/` side by side both count.
  - An empty export folder is `Some([])`: the folder was read whole and holds
    no export.
- Known over-matches of the rule as specified, both safe because they keep a
  file out of `No output`: `BF_00634_1.jpg` also counts for `BF_00634.DNG` when
  it is really `BF_00634_1.DNG`'s own export, and "any extension" lets a stray
  `X.xmp` in `Output/` count for `X`.
- The symlink tests are `#[cfg(unix)]`, so they run on the Linux and macOS CI
  legs, not on the Windows machine this step was checked on. The `Err` path of
  an unreadable folder is covered on every platform by handing `export_names`
  a path that is a file.

## Step 2

- `outputs_of` needs the listing, so it cannot sit beside `list_arw` in one
  flat `Promise.all`. Both `openDirectory` and `resync` hold the `list_arw`
  promise and put `listed.then(readOutputs)` next to it; a rejected listing
  still rejects the whole, and `readOutputs` turns only its own rejection into
  no answer, so an unreadable export folder never fails the open.
- `setStatus` would not have shown the failure. `show()` clears `note` and the
  decoded preview clears it again, so a message set in `openDirectory` lasts
  until the first decode. The failure sits in `outputsFailure`, its own line in
  `renderMeta` like `imported`, replaced on every read, so it goes away by
  itself once the folder reads again. `resync` calls `renderMeta` because
  `refilter` does not when the list did not change.
- `setOutputSection` does not `refilter()`, unlike `setGenreSection(false)`.
  In `openDirectory` it runs before `files`, `index` and `openDir` are switched,
  where `refilter` would call `show()` and could write the new folder's file as
  the old folder's last viewed. In `resync` it would drop `previous`. Both
  callers filter on the next lines, so nothing is lost.
- `outputs_of` returns a subset of what it was asked, so `fileOutputs` alone
  cannot tell `no` from "never asked". Trash undo appends the restored files to
  `allFiles` and refilters before `resync("restore")`, and that resync defers
  for a whole scan. A restored file with an export, gone from the last read,
  would have read `no` and been offered to the trash again. `outputOf` takes the
  listing too and answers unknown for a file outside it.
- `No output` stays checked across folder opens like `Genre`, and the first
  filter of the next folder already has that folder's answer. A folder with no
  export folder hides the section and clears the check before that filter runs.
- `resync("focus")`, read from the code, not observed: it re-reads the exports,
  but `focusRescanDue` skips a focus within `FOCUS_RESCAN_INTERVAL` (5 s) of the
  last rescan, and `resync` defers until `faces-done` while a scan runs, so a
  focus during a scan shows the export only after the scan. `usage.md` says so.
  The hand check is pending, see below.

## Deferred issues (todo candidates)

- **Pending manual check: `No output` into `Move to Trash…`.** Step 2's
  checkbox was ticked on the automated criteria; this was not run (it needs the
  running app and a real folder). On Windows, under
  `D:\Photos\tests\<date>-<topic>\`, make a folder of RAW files with an `Output/`
  holding exports of a subset (an exact stem, a `_1` copy, a DeepPRIME export).
  Open it in Riffle, check `No output`, press `CmdOrCtrl+A`, right-click and
  choose `Move N Files to Trash…`. Expected: the section shows, the strip lists
  only the files with no export, and exactly those go to the Trash with their
  sidecars. Also rename `Output/` away and refocus: the section hides, the check
  clears and every file shows again. Basis: `plan.md` Step 2, Implementation
  approach. Files: `crates/app/ui/src/main.ts`, `crates/app/ui/src/filter.ts`.
- **Pending manual check: an export made while the folder is open.** Step 2's
  checkbox was ticked on the automated criteria; this was not run (it needs
  PhotoLab and the running app). On Windows, with `No output` checked, export a
  file from PhotoLab into `Output/`, wait more than 5 s with no scan running and
  bring Riffle back to the front. Expected: the exported file leaves the strip.
  Repeat within 5 s of the last rescan, and during a scan, and record what
  happens; if the delay proves too long, the plan's fallback is to add `Output/`
  to `watch.rs`. Basis: `plan.md` Step 2, Implementation approach, and Trade-offs
  ("No watcher on the export folders"). Files: `crates/app/ui/src/main.ts`,
  `crates/app/ui/src/refresh.ts`, `crates/app/src/watch.rs`.
- `CLAUDE.md`'s description of `src/filter.ts` lists the `AF eye`, `Eyes` and
  `Genre` sections but not the new `Output` section. Step 2 does not list
  `CLAUDE.md`, so it was left alone. Basis: Step 2 implementation. Files:
  `CLAUDE.md`, `crates/app/ui/src/filter.ts`.
