# Learnings: record the Windows real-device checks run through MCP (2026-11-27)

## Step 1

- What MCP plus `screenshot` closed on Windows: the scan stop on a folder
  switch, the held and dropped trash runs, the trash undo / redo, the
  rename refusal of `open_folder`, the `f` overlay's judgment sampling, the
  1:1 view's memory plateau, the steered strip thumbnails, the grayscale
  latch and the peaking layer. What it cannot: `present` p95 (only key
  presses print `page` lines; `navigate` prints none), a dialog's own
  behaviour (an MCP-confirmed run opens none), the menus' clicks, a
  Ctrl-click and the IME. A second monitor and an idle `D:` were not at
  hand.
- The setup worth keeping: per-run `--index` / `--settings` / `--log` /
  `--mcp-port`, `mcpEnabled` and `mcpAllowDestructive` through
  `--import-settings`, one MCP session for a series of `navigate` calls to
  page faster than key repeat, windows closed with WM_CLOSE, and hardlinks
  of a date folder not opened that session as a stand-in for `robocopy /J`
  copies.
- To reach the rename refusal the folder needs index rows: 2148 gave a
  796 ms rename; with none the rename ends in 10 ms, before any open
  arrives.
- `mise run fmt` formats no Markdown; LF, blank lines and the table's
  alignment were checked by hand.
- A doubled backslash in a Python script written through a quoted Bash
  heredoc reached the file as a single one, so the `...\trash-large` path
  in a string literal became a tab in `todo.md`; checking the edited file
  for tab characters caught it. Write a backslash through `chr(92)` in such
  scripts, or write the script with the Write tool.
- Left as it is: the `Camera` / `Lens` item's box in `todo.md` still names
  the reference Tauri build "under" the removed `f` overlay item
  (`App: real-device checks of the f focus mark, face boxes and face mesh`);
  the box also names the build inline, and the item belongs to another
  session.

## Deferred issues (todo candidates)

- The eyes supersede rule is stale in `CLAUDE.md` and in the code comments.
  `CLAUDE.md`'s `src/eyes.rs` description says "so a newer request or a
  clear supersedes the running judgment", and the doc comments of
  `EyesTicket` and `EyesCache::current` in `crates/app/src/eyes.rs` say the
  same. `EyesCache::request` issues no ticket while a judgment is in flight,
  so only a `clear` supersedes a running one (seen on 2026-11-27: 60
  `navigate next` 8 ms apart logged three `eyes` lines, see `results.md`
  here; `docs/agents/app.md`'s section "An on-demand judgment is superseded
  through a shared atomic" was corrected in this step). A later change that
  touches either should correct the wording. Files: `CLAUDE.md`,
  `crates/app/src/eyes.rs`.
