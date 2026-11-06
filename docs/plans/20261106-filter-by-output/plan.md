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

# Filter by export: an `Output` section in the strip's filter menu

## Purpose

The user culls in Riffle, exports the keepers from PhotoLab into the
`Output/` subfolder of the shooting folder (PhotoLab's default export folder,
a child of the RAW folder; see `../../agents/photolab.md`, "A picked virtual
copy is invisible to `dop::read_flag`"), and may then run **Sequence JPEG
Timestamps…** on it, which writes the sibling `Output-sequenced/`, so both sit
directly inside the RAW folder. Whether a RAW has a file in either folder is
the ground truth of what was kept; the pick flag is not (a picked virtual
copy leaves the master item unflagged). To free the SSD the user wants to
narrow the strip to the RAWs with no export, select all and use the existing
`Move to Trash…`.

After this plan the filter menu has an `Output` section with `Output` and
`No output`, shown only while the open folder has an `Output` or
`Output-sequenced` subfolder that could be read, so a missing or unreadable
export folder never classes a file as not exported. The match is decided in
one Rust module with unit tests, the filter decision in `filter.ts` with
vitest tests, and the trash flow is unchanged.

What is established from the code, the docs and the user's library:

- A survey of `D:\Photos` on 2026-11-06 (78 folders holding a RAW and an export
  folder, 12,917 exports, all `.jpg`; 78 `Output`, 12 `Output-sequenced`):
  12,256 exports have a RAW's exact stem, 655 are `<stem>_<n>`, and the other 6
  are `<stem>-<n>` of a DeepPRIME DNG (`L1002140-DxO_DeepPRIME 3-2.jpg`). No
  other suffix exists.
- Of the 58 RAWs that sit next to their own DeepPRIME DNG, 35 have only the
  DNG exported and 23 have neither exported. The user decided the original RAW
  counts as exported in those 35 cases, so trashing `No output` cannot take the
  original of an exported frame.
- The sequencer copies every JPEG under its own name
  (`crates/core/src/sequence.rs`). The names `Output` / `Output-sequenced` are
  hard-coded nowhere today; the library spells them `Output`, the user
  `output`, so the folder name is matched case-insensitively.

## Steps

- [ ] Step 1: The `outputs_of` command: which of a folder's files have an export
  - Done when:
    - A new `crates/app/src/output.rs` holds the pure match: given the RAW
      file names of a folder and the file names found in its export folders,
      which RAWs have an export. A RAW `X.ARW` has one when some export file's
      stem (extension removed, any extension), compared case-insensitively,
      equals `X`, equals `X_<digits>` or `X-<digits>` (a PhotoLab virtual
      copy), or starts with `X-DxO` (a DeepPRIME DNG of `X` was exported). The
      suffix rules apply to the export's stem, never to the RAW's, so Sigma BF
      `BF_00634.DNG` still needs `BF_00634` or `BF_00634_<n>`, and
      `BF_00634_1.DNG`'s own export `BF_00634_1.jpg` is matched by its exact
      stem. Rust unit tests cover: exact stem, case difference, `_1` copy, `-2`
      copy, a DeepPRIME DNG matched by its own stem and by its `-2` copy, the
      original RAW matched through its DeepPRIME export, no match on a
      different stem (`DSC0001` vs `DSC00011`), a file in `Output-sequenced`
      only, and both folders present.
    - A `#[tauri::command] outputs_of(dir, files: Vec<String>) ->
      Result<Option<Vec<String>>, String>` in `output.rs` (registered in
      `main.rs`'s handler list next to `genres_of`): it lists
      `<dir>/Output` and `<dir>/Output-sequenced`, the folder names compared
      case-insensitively against one `read_dir` of `dir` (so `output/` on a
      case-sensitive file system is found too), takes the regular files
      directly in each (no recursion), and returns the subset of `files`
      that have an export. It returns `Ok(None)` when neither folder exists
      (or neither is a directory), and `Err` when one exists but cannot be
      read; it never returns `Some` from a partial read. Runs under
      `spawn_blocking` like `genres_of`.
    - `CLAUDE.md`'s layout paragraph names `src/output.rs`.
  - Implementation approach:
    - Keep the command's input the paths `list_arw` returned, so the backend
      matches against exactly what the strip lists and the frontend keeps a
      `Set<string>` keyed like its other maps. Canonicalize `dir` as
      `genres_of` does (`canonicalize`).
    - Nothing is stored in the index and no watcher is added; the listing is
      one `read_dir` of two small folders and is re-read on every open and
      resync (Step 2).

- [ ] Step 2: The `Output` section of the filter menu, shown while an export folder exists, and the docs
  - Done when:
    - `crates/app/ui/index.html` has a `<div id="filter-output" hidden>` after
      the `Eyes` section (before `#filter-genre`) with a `menu-label`
      `Output` and two `menu-item` buttons, `data-output="yes"` labeled
      `Output` and `data-output="no"` labeled `No output`.
    - `crates/app/ui/src/filter.ts`: `FilterState` gains `outputs:
      Set<OutputState>` (`export type OutputState = "yes" | "no"`), per
      session like `genres` (not part of `StoredFilter`, `toStored` and
      `applyStored` unchanged, so `commands.rs`'s `filter_setting` and the
      MCP filter tool are untouched). `passes` takes one more optional
      argument, `output?: OutputState`, and a checked item passes a file
      only when `output` is given and checked: a file whose state is
      unknown (`undefined`, the command's result not in hand or `null`)
      passes no checked `Output` item, so nothing is shown as "No output"
      on a missing or unread folder. Vitest tests in `filter.test.ts` cover
      `yes`, `no`, both checked, nothing checked, and `undefined` with `no`
      checked (must fail).
    - `crates/app/ui/src/main.ts`: a `shownOutputs` set, a
      `fileOutputs: Set<string> | null` (null while no export folder is
      known), `filterOutput` element wiring in `filterItems`, `mirrorFilter`,
      the item click handler, `filterActive`, `filter-reset` and `passes`
      (`fileOutputs === null ? undefined : fileOutputs.has(path) ? "yes" :
      "no"`). `openDirectory` and `resync` invoke `outputs_of` in their
      `Promise.all` next to `list_arw`, passing the listing, so the first
      `filter(passes)` of an open already knows the exports; the result
      sets `fileOutputs` and shows or hides `#filter-output` through a
      `setOutputSection(found: Set<string> | null)` that, like
      `setGenreSection(false)`, clears `shownOutputs`, `mirrorFilter()`s and
      `refilter()`s when the section goes hidden. A rejected `outputs_of`
      (unreadable folder) is treated as `null` and its message put on the
      status line through `setStatus`, so the user sees why the section
      vanished. The folder-token guard applies as for `genres_of`.
    - `docs/humans/usage.md`'s **Filter menu** entry describes the section
      (what counts as an export, the two folders, the case-insensitive
      names, the copy and DeepPRIME rules, when the section is shown, that the
      checks last the session, and that an export made while the folder is
      open shows up when the window regains focus or the folder is reopened,
      since the folder watcher does not watch the export folders), and
      `docs/humans/usage.ja.md` the same in Japanese, in the same PR.
    - `mise run ci` passes (format, lint, type-check, vitest, cargo tests).
  - Implementation approach:
    - Assumes Step 1 is merged.
    - Follow the `Genre` section exactly for visibility and per-session
      state; do not add the section to the stored filter (a remembered
      `No output` would otherwise hide every file of a folder with no
      exports only until the hide-on-null clears it, which is confusing).
    - The delete step needs no change: with `No output` checked, `Select
      all` then `Move to Trash…` targets the filtered `files` as today
      (`targets(selection, files, index)` in `context.ts` / `trash.ts`).
      Verify by hand on a folder under `D:\Photos\tests\<date>-<topic>\`
      with an `Output/` holding a subset, and record the check in
      `learnings.md`.
    - Verify during implementation that `resync("focus")` after an export
      from PhotoLab refreshes the section (the `FOCUS_RESCAN_INTERVAL` skip
      may delay it; note the observed behavior in `learnings.md`).

## Trade-offs and risks

- **Match rule: strict, not a prefix match.** The survey found no export
  outside the four forms, so a loose prefix rule is not needed, and it would
  let `DSC0001` claim `DSC00011.jpg`'s export. A future exporter suffix would
  read `No output`; the user sees the file in the strip before trashing it, and
  the Trash keeps it.
- **Where the folders are: child only.** `Output` and `Output-sequenced`
  directly inside the RAW folder is what PhotoLab's default and the
  sequencer produce. A user exporting elsewhere sees no section at all
  (safe), not a wrong one. Searching siblings or a configured path is not
  planned.
- **No watcher on the export folders.** The folder watcher is non-recursive,
  so an export made while the folder is open is picked up by the focus
  resync or a reopen. Adding `Output/` to the watch is a small change in
  `watch.rs` but would mean rescanning the RAW folder on every exported
  file; left out unless the hand check in Step 2 shows the focus resync is
  not enough.
- **Interaction with `docs/plans/20261106-trash-over-scan/plan.md`.** That
  plan changes the trash flow's timing and progress, not what it targets;
  the two can merge in either order.

## Progress

- (none yet)
