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

# Everything the native app does, over MCP

## Purpose

The GPUI app's MCP companion exposes seven tools (`get_view`, `get_photo`,
`get_preview`, `show_photo`, `select_photos`, `set_view`, `set_judgment`): a
client can follow a culling session and record judgments, nothing else. This
work extends it until every user-facing action of the native app
(`crates/native`) has an MCP tool, or an explicit, documented reason it has
none, so the app can be driven end to end by a script or an assistant, the way
DaVinci Resolve is through its scripting API.

Each tool goes through the code path the UI uses (the undo history, the
sidecar writer, the index, the idle gate, the settings writes), so an action
taken over MCP is indistinguishable afterwards from the same action taken with
a key, a menu or a dialog.

Current state, as investigated on 2026-11-17:

- The backend server `crates/backend/src/mcp.rs` (rmcp 3, loopback Streamable
  HTTP on `MCP_PORT`) hands each view tool to the app through `Bridge::call`,
  which waits at most `BRIDGE_TIMEOUT` (5 s). `crates/native/src/mcp.rs` puts
  the request into the view's channel as `scan::Event::Mcp`; the view's
  `mcp_request` (in `crates/native/src/main.rs`) builds a `companion::View`
  snapshot, `companion::answer` returns an `Action` or a refusal, and the view
  applies it and replies.
- `select_photos` refuses several photos and `get_view.selected` is the
  current file alone, although the view has had a multi-selection
  (`src/selection.rs`) for a while. The module doc of `companion.rs` and the
  MCP section of `docs/agents/native-app.md` still say the view has no
  selection.
- The native app has no compare view and no grayscale: `compare` (`v`) and
  `grayscale` (`g`) are in the shared keymap (`crates/backend/src/shortcuts.rs`)
  but `key_down` only logs them as not handled yet (grayscale is planned
  separately in `docs/plans/20261117-gpui-grayscale/`). It has no file rename
  (only the folder rename is ported), no Clear Cache, no AI / ranking / genre
  features and no MCP settings page (`mcpEnabled` is only read from the
  store).
- The Tauri app (frozen since 2026-11-15) shares the backend server; its
  `companion.ts` answers an unknown request kind with `unknown request: <kind>`.
- `key_down` returns early while the settings modal, the trash dialog or a
  folder dialog is open and while an inline rename owns the keys;
  `mcp_request` has no such guard today.

## Decisions

Made with the user on 2026-11-17.

- **D1, confirmation of destructive operations** (Move to Trash, Delete
  Sidecars, Rewrite Sidecars, Sequence, Import, Rename by Capture Date, folder
  rename): a new settings-store key `mcpAllowDestructive` (off by default,
  shown on a new MCP page of the native settings modal, added in Step 4). While
  it is on, a destructive tool works in two calls: a preview call that returns
  the dialog's content and an id, then a run call naming that id with
  `confirm: true`, no dialog. While it is off, the tool opens the app's own
  dialog and replies "awaiting confirmation" with the preview; the user
  confirms in the app.
- **D1, Delete Permanently**: exposed, but always through the app's dialog,
  confirmed by the user's click, whatever `mcpAllowDestructive` says. Only
  recoverable operations ever run without a dialog.
- **D1, redo of a trash run**: follows `mcpAllowDestructive` like Move to
  Trash (on: runs; off: refused with a message saying so, since the UI's redo
  has no dialog to open).
- **D2, long-running operations**: a tool that starts one replies at once with
  what it started. `get_status` (Step 3) reports what is under way (listing,
  scan pass, trash run, folder operation: waiting for confirmation, held for
  the scan, running with done / total, finished with the status line's
  summary, failed). `wait_for_idle { timeout_ms }` (Step 3) is a long-poll that
  returns when nothing is under way or the timeout passes, with the
  `get_status` answer; the bridge's timeout is overridable per call for it.
- **D3, the Tauri app's tool list**: host-specific. The Tauri app keeps its
  seven tools and its `INSTRUCTIONS` (including the promise that nothing is
  deleted) with `crates/app` untouched, through a `with_port`-style builder on
  `Mcp`; the native app opts into the full set.
- **D4, tool shape**: typed, declarative tools (set a state rather than toggle,
  so a repeated call is idempotent).
- **D6, features the native app does not have yet** (compare, grayscale,
  file rename, Clear Cache, the AI / ranking / genre features): each still
  gets its tool (or its value in an existing tool, such as `set_view`'s
  `compare`), whose call is refused with a message saying the feature is not
  implemented in the native app yet. The client learns the feature exists and
  why it cannot use it; when the feature lands, its tool is implemented in
  place.
- **D5, documentation**: `docs/agents/native-app.md` only, until the human
  docs move to the native app; `docs/humans/usage.md` and `README.md` (which
  describe the Tauri app) are not changed.

## Steps

- [x] Step 1: Toolset plumbing, the real selection, navigation and judgment undo
  - Done when:
    - `select_photos` with several paths sets the view's selection (the first
      path current), `get_view.selected` reports the selection's targets, and
      `set_judgment` without `paths` judges the selection's targets, as a
      judgment key does.
    - New tools exist and are answered by the native app: `navigate`
      (`previous`, `next`, `first`, `last`, `burst_previous`, `burst_next`,
      `burst_frame_previous`, `burst_frame_next`), `select_all`,
      `reject_rest`, `undo` and `redo` (judgment entries; a trash run on top
      of the history is refused with a message until Step 5).
    - The tool list is host-specific (D3): the Tauri app's server lists exactly
      the seven tools and its current `INSTRUCTIONS`, with `crates/app`
      unchanged; the native app's lists the full set.
    - A tool call is refused with a clear message while the settings modal,
      the trash dialog or a folder dialog is open or an inline rename is
      under way, the states in which `key_down` drops a key.
    - The backend's tool-list tests cover both lists; `companion.rs` has unit
      tests for each new request kind and for the refusals.
    - `CLAUDE.md` ("its seven tools", the `companion.rs` description), the
      `companion.rs` module doc and the MCP section of
      `docs/agents/native-app.md` describe the new state.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/backend/src/mcp.rs`, `crates/native/src/companion.rs`,
      `crates/native/src/main.rs` (`mcp_request`, `key_down`),
      `crates/native/src/mcp.rs` for the opt-in.
    - Keep the existing shape: argument structs with `JsonSchema` and cheap
      validation in the backend, one `Bridge::call(kind, args)` per view tool,
      `companion::answer` staying pure (a `View` snapshot in, an `Action` or a
      refusal out) so every rule is unit-tested without GPUI, and
      `mcp_request` only applying the `Action` through an existing view
      method.
    - Reuse the view's own methods so the paths are the keys': `go`,
      `move_burst` with `burst::burst_step` / `burst::burst_frame_step`,
      `select_all`, `set_selection` with `selection::selection_of` or
      `selection::restore`, `reject_rest`, `step`. Consider moving the action
      arms of `key_down`'s `match` into one method both `key_down` and
      `mcp_request` call, so a keymap action and its tool cannot drift.
    - `View` needs the selection (`selection::targets` over `files` and the
      current index) to answer `selected` and the default targets of
      `set_judgment`.
    - Verify how rmcp's `#[tool_handler]` can take a router that is a field
      rather than `Self::tool_router()`, and whether `ToolRouter` can drop
      routes or be built from parts; the seven tools by default, the native
      app opting into all.
    - `mcp_request` has no `Window` (it runs from `scanned`); anything a later
      step needs a `Window` for must find a path that does not, so do not
      design the `Action` enum around window access.
    - Verify end to end as `docs/agents/native-app.md` prescribes for MCP test
      runs (`--mcp-port` with a free port other than 41917, `mcpEnabled`
      seeded through `--import-settings` into a fresh `--settings` store,
      `--log` and `--index` in the test folder, the `curl` body on stdin), on
      test copies of photos.

- [x] Step 2: Sort, filter, panes and overlays
  - Done when:
    - Tools exist for the view state a user changes without touching files:
      `set_sort` (the native sort keys), `set_filter` (flags, stars, labels,
      orientation, `Face focus`, `Eyes`, `Output`, the per-folder EXIF groups,
      and a reset), `set_panes` (left, right, strip shown or hidden), and
      `set_overlays` (the `f` focus overlay, `e` peaking, `d` performance).
    - `get_view` reports what those tools set: the filter's checked items per
      section and the items the open folder offers (its EXIF groups, whether
      the `Output` section applies), the panes shown, and the overlays on.
    - A change made over MCP is persisted exactly as the UI persists it
      (`sortOrder`, `filter`, `panels` in the settings store) and leaves the
      current file where `filter::anchor_after_filter` puts it.
    - Unit tests cover each tool's argument rules and refusals; the tool-list
      test is updated; `CLAUDE.md` and `docs/agents/native-app.md` are
      updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the toolset plumbing and the modal guard).
    - Sort and filter go through `menu::apply` and the tail of `menu_action`
      (the `sortOrder` / `filter` writes, the `kept_anchor` refilter), either
      by turning the wanted state into `menu::Action`s or by factoring that
      tail out; do not write `self.filter` directly.
    - The tools set a state rather than toggle (D4); the underlying view
      methods toggle (`toggle_marks`, `toggle_peaking`, `Overlay::toggle`,
      `Panels::toggle`), so compare with the current state first, as
      `set_view` already does for zoom.
    - Panes go through `change_panels` (it hands the keyboard back from a
      hidden left pane, re-cuts a 1:1 crop and writes `panels`).
    - Verify what the sort menu does in a view-only (JPEG-only) folder
      (`viewonly::sort_for` keeps capture order) and make `set_sort` answer
      the same way.
    - Decide in this step whether pane widths (`panewidths.rs`, set by a
      drag) are settable through `set_panes` or excluded as pointer-only
      geometry; record the choice for Step 8's exclusion list.

- [x] Step 3: Folders, the tree, `get_status` and `wait_for_idle`
  - Done when:
    - `list_folders` answers the roots (no path) or one folder's subfolders
      and photo count, as the tree shows them; `open_folder` opens a
      directory as a click on its name in the tree does; `tree_folder`
      expands, collapses, refreshes, expands all or collapses all under a
      folder, as the tree's expander and right-click items do; `reveal` shows
      a folder, or a file of the open folder, in the OS file manager.
    - `get_status` answers what the status row and the pane notes tell a
      user: whether a listing or a scan pass is under way with its
      done / total, the status line's text, and the notes shown.
      `wait_for_idle { timeout_ms }` returns when nothing is under way or the
      timeout passes, with the `get_status` answer (D2). `open_folder` replies
      at once.
    - An open is refused or deferred exactly as the UI's is (a rename in
      flight on that folder, a run in flight deferring the open).
    - Unit tests, the tool-list test, `CLAUDE.md` and
      `docs/agents/native-app.md` are updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 are merged.
    - `open_folder` calls `open_from_tree`, not `open` or `list` directly. The
      listing of a large or slow folder outlasts `BRIDGE_TIMEOUT`, which is
      why the reply does not wait for it. A path that is a file can follow
      `dropped::dropped_dir` (its parent), or be refused; decide here.
    - `list_folders` can be answered in the backend without the view, like
      `get_photo` (the backend's `folders::roots` / `folders::list` on the
      blocking pool); the home directory is the caller's to give, so check
      how the native app passes it and give `Mcp` the same value.
    - `tree_folder` uses `tree_toggle`, `tree_refresh`, `tree_expand_all` and
      `Tree::collapse_all` followed by `tree_changed`, and paths in the
      tree's own spelling (`tree::respell`).
    - `reveal` follows `folder_action`'s `RevealFolder` (a root opens instead)
      and `files_action`'s `RevealFile` with `context::shown_path`.
    - `get_status` is the reporting channel Steps 5 to 7 extend with the
      operation under way and its result; shape it with that in mind. Its
      inputs exist already: `scan::Status`, `progress.rs`'s `shown` /
      `fraction`, `step_note`, `tree_note`, the folder operations' notes.
    - `wait_for_idle` needs a per-call bridge timeout longer than
      `BRIDGE_TIMEOUT` (capped, e.g. at 55 s, under typical client limits);
      the view answers it when it becomes idle rather than by polling.

- [ ] Step 4: Settings, and the `mcpAllowDestructive` switch
  - Done when:
    - `get_settings` answers every setting the native settings modal shows:
      the sidecar format, the label names (and the Lightroom language preset
      in force, if any), auto-advance, the keymap, the import root and
      pattern, the theme (mode, light and dark theme names, with the names
      available) and `mcpAllowDestructive`.
    - `set_settings` changes any of them except `mcpAllowDestructive` with
      the modal's own rules and effects: a format switch drains, persists and
      resets as `switch_format` does; label names as `switch_names`; a keymap
      edit (add a key, remove a key, reset) through `settingsmodal::edit`,
      refusing what the modal refuses (`APP_KEYS`, reserved keys); the import
      pattern through `settingsmodal::pattern_change`; the theme applied
      live.
    - The settings modal has an MCP page with the `mcpAllowDestructive`
      switch (off by default, persisted in the settings store), which a
      client cannot change over MCP (only the user can grant it).
    - A setting that the settings modal is showing is not left stale: the
      call is refused while the modal is open (the Step 1 guard).
    - Unit tests, the tool-list test, `CLAUDE.md` and
      `docs/agents/native-app.md` are updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged; independent of Steps 2 and 3.
    - The view applies the modal's changes in the `settingsview::Change`
      subscription in `Riffle::new`. Move that `match` into one method and
      have `set_settings` produce the same `Change` values, so the two
      cannot differ.
    - The validation lives in `settingsmodal.rs` (`format_change`,
      `label_change`, `label_names_of`, `mode_change`, `theme_change`,
      `pattern_change`, `edit`) and the backend (`import::root_setting`,
      `import::validate_pattern`, `shortcuts`); reuse it rather than
      restating rules.
    - A folder operation in flight refuses the settings modal (`key_down`
      skips `OPEN_KEY` while `folderops.busy()`, since the run would read a
      format changed meanwhile); `set_settings` must refuse then too.
    - `mcpEnabled` is not settable: the native app reads it only at startup
      and a client would be switching off its own connection. Record it for
      Step 8's exclusion list.

- [ ] Step 5: Move to Trash, Delete Permanently, and undo / redo of trash runs
  - Done when:
    - `trash_photos` moves the given photos (or the selection's targets) to
      the OS trash, or with the delete mode deletes them, through
      `confirm_trash` / `confirm_run` / `run_trash`: the same guards, the idle
      gate (held for the scan's end, the scan canceled), the writer drain, the
      recorded run and the undo entry.
    - Confirmation follows D1: Move to Trash runs on a preview + `confirm`
      call pair while `mcpAllowDestructive` is on and opens the app's dialog
      while it is off; Delete Permanently always opens the app's dialog and
      runs only on the user's click.
    - `undo` and `redo` step a trash run on top of the history as `step`
      does (`step_trash`, restore or move again); a redo while
      `mcpAllowDestructive` is off is refused with a message.
    - `get_status` reports the trash operation (awaiting confirmation, held
      for the scan, running with done / total, finished with the status
      line's summary, failed).
    - Unit tests cover the request rules (paths outside the open folder or
      hidden by the filter, a view-only folder, a run already in flight, the
      confirmation pairing); the tool-list test, `CLAUDE.md`,
      `docs/agents/native-app.md` and the native server's `INSTRUCTIONS` are
      updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1, 3 and 4 are merged.
    - The flow is `trash::TrashFlow` (idle / asking / running) with the
      dialog following it (`follow_trash`); drive the flow's own transitions
      rather than calling `run_trash` around it, so `trashing`, the gate and
      `resync.hold` stay consistent.
    - Opening the dialog from `mcp_request` (no `Window`) needs a path; check
      whether `follow_trash` can run on the next render.
    - Test only on copies; the checks in `docs/agents/native-app.md` for the
      trash flow name the log lines to look for.

- [ ] Step 6: Folder operations I: Rewrite Sidecars, Delete Sidecars, Sequence JPEG Timestamps
  - Done when:
    - `rewrite_sidecars`, `delete_sidecars` and `sequence_timestamps` run
      the tree menu's flows on a folder, with the preview each dialog shows
      available to the client (the rows and notes of `sidecars.rs` /
      `sequence.rs`) and the confirmation D1 decides.
    - `cancel_operation` cancels what the UI can cancel (a sequencing run's
      cancel flag; a dialog awaiting confirmation is dismissed) and says so
      when the operation under way cannot be canceled.
    - The flows keep their rules: one folder flow at a time, the sidecar
      request held by the gate for the scan's end without canceling it, the
      refusal with `SCAN_RUNNING` when a listing started meanwhile, the
      `Delete Sidecars` run recorded for undo.
    - Progress and results are reported through `get_status`; unit tests, the
      tool-list test, `CLAUDE.md` and `docs/agents/native-app.md` are
      updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 5 is merged (the confirmation and reporting mechanism it
      introduced is reused, not reinvented).
    - Entry points: `folder_op` with `context::Action::RewriteSidecars` /
      `DeleteSidecars` / `SequenceTimestamps` (`request_sidecars`,
      `start_sequence`), then `sidecars_previewed` / `sequence_previewed`,
      `run_sidecars` / `run_sequence`, `sidecars_ran` / `sequence_ran` in
      `crates/native/src/folderops.rs`. The dialog content is built there
      from the backend's previews; the preview a tool returns should be that
      same content.
    - The dialogs open through `follow_folder_dialog`, which may need a
      `Window`; verify that a flow started from `mcp_request` (no `Window`)
      can open or skip its dialog.

- [ ] Step 7: Folder operations II: Import, Rename by Capture Date, folder rename
  - Done when:
    - `import_folder` previews and runs the tree menu's `Import…` (the plan's
      summary available to the client, the run cancelable through
      `cancel_operation`, progress and the result line reported through
      `get_status`); `rename_by_capture_date` previews and runs the rename or
      the split; `rename_folder` renames a folder with the inline rename's
      rules (`check_name`, the gate holding it for the scan's end, the refusal
      during an import). All three confirm as D1 decides.
    - After a rename of the open folder or one above it, the view follows as
      it does for the UI's rename (`folder_renamed`).
    - Unit tests, the tool-list test, `CLAUDE.md` and
      `docs/agents/native-app.md` are updated.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 6 is merged.
    - Entry points: `start_import` / `import_previewed` / `run_import` /
      `import_ran`, `start_date_folder` / `date_folder_previewed` /
      `run_date_folder`, and for the rename `request_rename(path, name)`,
      which already takes no `Window`, unlike `start_rename` (the inline
      editor).
    - The import keeps its previewed plan by id (`ImportPlan`) and the run
      copies that plan; the confirming run call must name the plan it
      confirms, so a plan re-read in between cannot be run blind.
    - The import's dialog steps aside during the run and the OS notification
      fires only while the window is not active (`import::done_notification`);
      keep both as they are.
    - An import refused for a missing root should say how to set it
      (`set_settings`, Step 4).

- [ ] Step 8: The coverage table, the final instructions and the exclusions
  - Done when:
    - A table in `crates/native/src/companion.rs` maps every keymap action
      (the names of `crates/backend/src/shortcuts.rs`'s `DEFAULTS`), every
      `context::Action` variant, every `menu::Action` kind, every folder
      operation and every settings page to the tool that performs it or to
      an exclusion with its reason, and a unit test fails when an action is
      added without an entry (an exhaustive `match` for the enums, a
      comparison against the keymap's action list for the names).
    - Every feature the native app does not have yet gets its tool, refused
      with a "not implemented in the native app yet" message (D6): `compare`
      (through `set_view`), `grayscale` (in `set_overlays` or its own tool;
      implemented for real if `docs/plans/20261117-gpui-grayscale/` has
      landed), file rename (`rename_file`), Clear Cache (`clear_cache`), and
      the AI features (ranking, genres, the `recommended` sort, the AI switch).
      The table lists them as "not implemented", not as exclusions, and a unit
      test checks each one's refusal message.
    - The exclusions are documented in `docs/agents/native-app.md` (D5), each
      with its reason. The ones known at planning time: quitting (`q`), opening menus and the settings
      modal, the tree's keyboard cursor and type-ahead, strip scrolling,
      window geometry, the OS pickers and drag and drop (replaced by path
      arguments), `mcpEnabled` and `mcpAllowDestructive` (the user's to
      grant), the clipboard copies (`Copy Path` / `Copy File Name` /
      `Copy Folder Name`: a client already has the paths, and a tool would
      write the user's clipboard unseen), and whatever Step 2 excluded.
    - The native server's `INSTRUCTIONS` describe the whole surface and what
      needs the user's confirmation.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 7 are merged.
    - Use one shared refusal text (e.g. `NOT_IN_NATIVE`) naming the feature,
      so every not-implemented tool reads the same.
    - `DEFAULTS` is private to the backend's `shortcuts` module; check what
      it exposes (`Keymap`'s actions) before adding an accessor.

## Risks

- About 30 tools enlarge every client's context; the descriptions should be
  shorter than `get_view`'s is today.
- `mcp_request` replies from the view's snapshot right after applying an
  action; an action that finishes asynchronously (an open, a refilter after
  a read) can be reported one step stale. Tools that start something reply
  with what was started, not with a `get_view` that pretends it is done.
- An MCP call arriving while the user is mid-gesture (a dialog open, an
  inline rename) is refused by the Step 1 guard; a script has to retry.
- The settings and folder-operation code is under active change on `main`
  (recent PRs touch `settingsview.rs`, the tree and the context menu); Step 4
  and Steps 6 and 7 should rebase early.
- Eight steps is one more than the usual ceiling; the folder operations are
  six flows and one PR for all of them would be too large to review.

## Progress

- (2026-11-17) Step 1 complete
- (2026-11-17) Step 2 complete
