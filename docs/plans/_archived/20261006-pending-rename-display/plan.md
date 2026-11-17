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

# Pending rename display

## Purpose

An inline rename (folder tree, strip) confirmed while a scan runs is held by
`idle.ts`'s `IdleGate` and runs when the scan ends, but the cell keeps
showing the old name; only the status line says `Rename…: waiting for the
scan to finish`. This closes `todo.md`'s
`### App: a pending rename waits silently with no visible pending state`: the
cell shows the new name at once in a pending style (muted, italic, a small
clock icon titled "Renames when the scan finishes"), goes back to the old name
when the held rename is replaced, discarded or fails, and lets the user edit
or cancel the pending name from the same cell.

Agreed spec (settled with the user; do not reopen):

1. Display: a confirmed inline rename that `IdleGate` holds shows the new
   name in the cell immediately in a pending style: `--muted-foreground` +
   italic (shadcn Neutral dark tokens, see
   `ui-styling.md`, removed since) plus a small Lucide `clock`
   icon with `title="Renames when the scan finishes"`. When the held rename
   runs and succeeds, the existing real-rename path takes over and the cell
   shows the normal style.
2. Revert to the old name + normal style when: (a) another pressed operation
   (Move Rejected to Trash, Undo / Redo Move Rejected to Trash, another
   rename) replaces the held rename in `IdleGate`'s single slot; (b) a
   folder switch discards it (`idle.discard()` in `openDirectory`); (c) the
   rename invoke fails after the scan ends (existing error status kept).
   For (a) and (b) the status line shows a transient `Rename… was canceled`
   note.
3. Further actions on the same cell: restarting the inline edit on a pending
   cell uses the pending name as the input's initial value; confirming a
   different name replaces the held rename; confirming back to the original
   name cancels the pending rename (revert, no rename); Escape keeps the
   pending one. Opening the folder in the tree and judging the file keep
   working as now (paths are still the old ones).
4. A user-initiated cancel (confirming back to the original name) goes
   through `idle.discard()` and also shows the `Rename… was canceled` note
   (user decision: one code path, the note is true).
5. On a label-colored strip cell (`.cell span.name.labeled`), the label's
   text color (`--foreground`) wins; the pending state shows there through
   italic + the clock icon only (user decision: keeps the text readable on
   the label color).
6. The scan-loading-feedback work (#684, scan progress bar and pulsing cells
   without a thumbnail) has merged into `main`; the pending style must sit
   consistently with it (same tokens, no clash with the pulse on a cell
   without a thumbnail).

## Steps

- [x] Step 1: Show, revert and re-edit a pending inline rename in the tree and the strip
  - Done when:
    - `IdleGate.request` accepts an optional cancel callback that fires when
      the held operation is replaced by a later `request` while busy, or
      dropped by `discard()`, and not when `drain()` runs it; `request`
      tells the caller whether the operation was held. `settings.ts`'s
      `clearGate` keeps working unchanged. `idle.test.ts` covers: held then
      replaced fires the first's cancel once; `discard()` fires it; `drain()`
      does not; a request that runs at once neither holds nor fires.
    - Pure helpers in `rename.ts` (tested in `rename.test.ts`) decide the
      pending-cell logic without DOM: the name a cell displays (pending name
      when one is held for that path, else the real one) and the outcome of
      confirming an edit on a pending cell: typed empty or equal to the
      pending name -> keep the pending rename; equal to the real original ->
      cancel the pending rename; anything else -> replace it with a new
      rename.
    - Folder tree: a held folder rename draws that row's name as the pending
      name with the pending class and the clock icon (surviving re-renders
      from listings, since `render()` rebuilds every row from `tree.nodes`);
      the row still opens / expands / right-clicks on its old path.
    - Strip: a held file rename draws that cell's name the same way, keyed by
      path so it survives the cell being released off-screen and recreated
      by `render()` / `createCell`, and `setFiles` on a rescan; judging the
      file keeps working on the old path. On a labeled cell the label's text
      color wins (spec item 5).
    - Revert cases (a), (b), (c) from the spec behave as specified in both
      views, including the `Rename… was canceled` status note for (a), (b)
      and the user-initiated cancel, through the existing `setStatus` idiom.
    - Re-edit cases from spec item 3 behave as specified in both views.
    - Lucide `clock` is added to `icons.ts` verbatim from lucide-static
      v1.48.0 (the version the file's license header names), its name added
      to that header and to its Feather-derived note (`LICENSE-lucide` lists
      `clock` as Feather-derived).
    - `style.css` gains the pending rule(s) using tokens only (no new hex);
      the `docs/agents/ui-styling.md` "Before opening a PR" greps still match
      only the listed exceptions. If the clock icon needs a size rule, it
      follows the existing `.cell span.candidate svg` pattern. The rules sit
      consistently with #684's pulse / progress styles.
    - `docs/humans/usage.md` and `usage.ja.md` (the two "A rename confirmed
      while a scan runs waits for the scan..." sentences, tree and strip)
      describe the pending display, the cancel cases and the re-edit
      behavior, in the same PR; `docs/agents/tauri-app.md`'s `IdleGate`
      note mentions the cancel callback.
    - `todo.md`'s `### App: the wait-for-scan manual checks for Move Rejected
      to Trash and Rename are still open` TODO list gains one item for the
      real-device check of the pending display (show, replace, folder
      switch, failure, re-edit, user cancel, labeled cell, both views). The
      pending-rename section itself is closed in wrap-up, not here.
    - `pnpm exec vp test` and `mise run ci` pass.
  - Implementation approach (as far as it is known):
    - `idle.ts`: extend `held` to `{ label, run, cancel? }`; `request(label,
      run, cancel?)` returns `boolean` (held or not); when busy and a
      previous op is held, call its `cancel` before replacing; `discard()`
      calls it too; `drain()` only runs. Keep the one-slot semantics and the
      existing tests intact. `main.ts`'s `whenIdle` returns the boolean.
    - `main.ts` (`renameFolder`, `renameFile`): after
      `whenIdle("Rename…", run, cancel)` returns held, call
      `folders.markPending(path, name)` / `strip.markPending(path, name)`;
      `cancel` clears the pending mark and `setStatus("Rename… was
      canceled")`. On invoke failure (existing `(err) => setStatus(...)`
      branches) clear the pending mark too, with no cancel note. On success
      the existing `folders.renamed(...)` / `strip.renamePath(...)` take over:
      clear the pending mark there (or inside those functions) before the
      redraw so the new name is drawn in the normal style. Also pass a
      "cancel pending rename" callback to the views (via `folders.init` /
      `strip.init`, or a small setter) that calls `idle.discard()`; since
      the only way a cell is still pending is that its rename is the held
      op, `discard()` is exact and the cancel callback does the revert +
      note. Keep edits localized to these functions and `whenIdle`; do not
      restructure `renderMeta`.
    - `rename.ts`: two pure helpers, e.g. `displayName(pending: {path,
      name} | null, path, real)` and `pendingOutcome(real, pendingName,
      typed): "keep" | "cancel" | { rename: string }` (reuse `confirmName`'s
      trim). `inlineRename(kind, path, original)` is then called with the
      pending name as `original` when a pending cell is edited, so an
      unchanged confirm or Escape leaves the pending rename alone; the
      "back to original" case needs the real name too, hence the helper.
    - `folders.ts`: module state `pending: { path, name } | null`;
      `render()`'s name span branch uses the helper for `textContent`,
      toggles a `pending` class and appends the clock `span` with
      `innerHTML = CLOCK_SVG` and `title`; `startRename` seeds the edit with
      the pending name when the path matches; `finish` / `finishInPlace`
      route through `pendingOutcome`. Export `markPending(path, name)` /
      `clearPending(path)` (render on change).
    - `strip.ts`: `pending: Map<string, string>` keyed by path (or a single
      slot); `createCell` and `renamePath` apply it; `startRename` /
      `finishRename` as in the tree. `cell.name` stays the span `Cell`
      holds, so a badge repaint still leaves the input alone; put the clock
      icon inside the name span (or as a sibling span added / removed with
      the mark), whichever keeps `input.replaceWith(cell.name)` correct.
    - `style.css`: `.folder .name.pending, .cell span.name.pending { color:
      var(--muted-foreground); font-style: italic; }` plus an inline-sized
      `svg` rule; `.cell span.name.labeled.pending` keeps `--foreground`
      (spec item 5).
    - Icon: take the `clock` SVG from lucide-static v1.48.0 (e.g. after
      `pnpm install`, `node_modules/lucide-static/icons/clock.svg`) and
      convert to the file's inline form (`viewBox`, `fill="none"`,
      `stroke="currentColor"`, `aria-hidden="true"`), as the other icons are.
    - Tests stay DOM-free (`vite.config.ts` test environment is `node`):
      the `IdleGate` behavior and the two helpers; the views are checked by
      hand (the new manual-check item).

## Trade-offs and risks

- Strip rescans: `setFiles` rebuilds cells; keying the pending mark by path
  keeps it across the rebuild, but if the rescan no longer lists the file
  (deleted on disk) the held rename will fail at drain time and revert via
  case (c), which is the existing behavior.
- Single-step scope: the step is one coherent feature across six source
  files plus docs. Splitting `IdleGate` + helpers into a first PR would leave
  an unused API in `main`; kept as one PR.

## Progress

- (2026-10-06) Step 1 complete
