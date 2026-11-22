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

# Fix four display-text bugs from the 2026-11-22 Windows device checks

## Purpose

The Windows real-device checks of 2026-11-22 left four small `todo.md` items
where the app shows a wrong or misleading text:

1. The status line's `no RAW files in` text shows the Windows verbatim prefix
   (`\\?\D:\...`).
2. The Import dialog's `Source:` line shows the same prefix.
3. An Undo of Move to Trash whose files left the Recycle Bin says
   `Restored 0 files from the Trash, 3 failed` for one RAW with two sidecars,
   and gives the emptied sidecars the reason
   `left in the Trash: its RAW could not be restored`.
4. A held `Rewrite Sidecars from Index…` on an open folder with nothing judged
   answers `No judgments indexed for <name>; open the folder first`.

Once done, the paths the user reads carry no verbatim prefix, the undo's note
counts a RAW with its sidecars as one item as the move's note does
(`Moved 1 file to the Trash`), a sidecar that is itself gone from the Trash
says so, and the rewrite's refusal no longer tells the user to open a folder
that is open. Each fix is pinned by a unit test, and the four items leave
`todo.md`.

Evidence: `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`
(item 3), `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`
(item 1), `docs/plans/_archived/20261122-windows-device-checks-5/learnings.md`
(items 2 and 4).

Out of scope: every other `todo.md` item, in particular the held folder
rename's Access denied and the folder dialogs' file list's blank band.

## Decisions (approved by the user, 2026-11-22)

- **Item 3, the count.** One item per RAW (or JPEG) group: the note counts the
  groups with at least one failed file. RAW gone with two sidecars:
  `Restored 0 files from the Trash, 1 failed`. RAW back, two sidecars failed:
  `Restored 1 file from the Trash, 1 failed` (the move's note keeps its
  per-sidecar count in that case; `trashed_status` is not changed). A sidecar
  with no media file before it (a `Delete Sidecars…` run) is an item of its own.
- **Item 4, the wording.** One neutral engine message,
  `No judgments indexed for <name>`, for an open and a never-opened folder
  alike; the "open it once" advice stays in `docs/humans/usage.md`.
- **Items 1 and 2, the scope.** Besides the two reported texts and the Sequence
  dialog's `Source:` line, also strip the prefix from the Sequence dialog's
  `Output:` line, the sequencing run's done status (`ran.output_dir`) and the
  import's `reading <dir>…` note.

## Steps

- [x] Step 1: Fix the four texts, pin each with a unit test, and delete the four `todo.md` items
  - Done when:
    - The status line of a folder with no files reads `no RAW files in <path>`
      with the path passed through `context::shown_path`; a unit test feeds a
      `\\?\D:\...` path and asserts the text carries no verbatim prefix.
    - The Import dialog's and the Sequence dialog's `Source:` lines, the
      Sequence dialog's `Output:` line, the sequencing done status and the
      import's `reading <dir>…` note show their path through `shown_path`;
      unit tests assert it for a `\\?\`-prefixed path. `preview.source`,
      `ImportPlan.source` and the `import:` log line keep the canonical path.
    - An undo whose RAW and both sidecars are gone from the Trash reads
      `Restored 0 files from the Trash, 1 failed`; unit tests pin the count in
      `crates/app/src/trash.rs` and what the engine reports in
      `crates/engine/src/trash.rs`. The per-file failures stay in
      `Restored.failed` (the pane's errors list each), and `restored_count` and
      the `What::Sidecars` arithmetic keep their results.
    - In the engine's `restore`, a sidecar whose RAW did not come back and that
      is itself not in the Trash fails with `NOT_IN_TRASH`; one still in the
      Trash keeps `RAW_NOT_RESTORED` and is not moved. Both cases have a unit
      test.
    - `foldersidecars::targets` answers `No judgments indexed for <name>` with
      no `open the folder first`; its test asserts the whole message, and
      `docs/humans/usage.md` and `docs/humans/usage.ja.md` quote the new
      message in the same PR (their surrounding sentence kept accurate).
    - The four `todo.md` items are deleted (or narrowed to what stays open),
      and the cross-reference to "the two verbatim prefix items below" in the
      `Import…` real-device item's background is reworded so it points at
      nothing that is gone.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - Item 1. The text is `format!("no RAW files in {}", listed.dir)` in
      `crates/app/src/main.rs` (the listing handler, where `self.files` and
      `self.all` are empty). A few lines above, the same function computes
      `context::shown_path(&listed.dir)` for `lastFolder`. Move the format into
      a small pure function (e.g. in `crates/app/src/empty.rs`, which holds the
      no-file texts) and test it. `held_label` in `main.rs` is log-only; leave
      it. `fail` prints an engine message as is; leave it.
    - Item 2. `crates/app/src/folderops.rs`: `import_previewed` builds
      `format!("Source: {}", preview.source)` and `sequence_previewed` builds
      `format!("Source: {dir}")`; `start_import` sets
      `importing = Some(format!("reading {dir}…"))`;
      `crates/app/src/sequence.rs` `done_status` prints `ran.output_dir`.
      `start_import` canonicalizes the folder before `importer::summarize`, so
      the tree's `Import…` shows the prefix on Windows too. Put each line in a
      pure function next to the dialogs' other line builders
      (`crates/app/src/import.rs`, `crates/app/src/sequence.rs`) and test it.
    - Item 3 (reason). `crates/engine/src/trash.rs` `restore` answers
      `RAW_NOT_RESTORED` for every sidecar after a RAW that did not come back,
      without asking `in_trash`. Ask `in_trash` for such a sidecar: no item
      gives `NOT_IN_TRASH`, an item gives `RAW_NOT_RESTORED` and `mover` is
      still not called. Keep the existing `RAW_NOT_RESTORED` tests meaningful
      for the "still in the Trash" case.
    - Item 3 (count). The grouping is known only in `restore`'s loop, so the
      count comes from the engine (e.g. a failed-item count on `Restored`,
      filled by `restore` and `Restored::none`); `restored_status` in
      `crates/app/src/trash.rs` uses it. Update the test helpers in
      `crates/app/src/trash.rs` and `crates/app/src/history.rs` that construct
      `Restored`. Check whether `docs/humans/usage.md`'s "with how many failed"
      needs a word on what is counted; keep `usage.ja.md` in sync.
    - Item 4. `crates/engine/src/foldersidecars.rs` `targets`; test
      `a_folder_with_no_row_or_no_raw_is_refused`.
    - Commit type: `fix`, Conventional Commits, English.

## Risks

- `docs/agents/app.md` quotes `no RAW files in ...` once (with an ellipsis, so
  it stays true); the only user docs quoting a changed message are the two
  `usage` files.

## Progress
