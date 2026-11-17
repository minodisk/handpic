# Learnings: native-mcp-everything

## Step 1: Toolset plumbing, the real selection, navigation and judgment undo

- **rmcp takes a router field.** `#[tool_handler(router = self.tools)]`
  expands to `self.tools.call(..)`, `.list_all()` and `.get(..)`, so the
  `Companion` holds a `ToolRouter<Companion>` and the host picks it. A second
  `#[tool_router(router = native_router)]` block generates a separate
  constructor for the native-only tools, and `ToolRouter` implements `Add`, so
  `with_all_tools` is `tool_router() + native_router()`. The same attribute
  options exist in rmcp-macros 3.4.1 (the root workspace's lock) and 3.10.0
  (the native workspace's), so nothing needed a version bump.
- **The instructions are per host too.** `Companion.instructions` is
  `INSTRUCTIONS` for the Tauri app (unchanged text) and `NATIVE_INSTRUCTIONS`
  after `with_all_tools`; Steps 5 and 8 extend the native text only.
- **The busy guard refuses every request except `get_view`.** The backend's
  `get_photo` and `get_preview` resolve their path through a `get_view` bridge
  call, so refusing `get_view` would have taken the reads down with it, and a
  read changes nothing the user is looking at. The native instructions say "a
  call that changes anything is refused".
- **`rename_owns_keys` has a side effect** (it disarms the tree's slow
  click), so the guard reads a new side-effect-free `FolderOps::editing`
  instead.
- **One `perform(companion::Key)`** now runs the moves, Select All, Reject
  Rest, undo and redo for both `key_down` and `mcp_request`; `Key::of_action`
  maps the keymap names, and Home / End map to `First` / `Last`.
- **`undo` / `redo` refuse up front** from `history::Top` (a new
  `Undo::top`): a trash run on top, an undo or redo of one in flight, a
  view-only folder, an empty history, in `step`'s own order. A judgment batch
  whose files are all unlisted still pops silently, as the key does, and the
  tool then replies with an unchanged `get_view`.
- **End to end** (`--mcp-port 41967`, `mcpEnabled` seeded through
  `--import-settings` into a fresh `--settings`, `--index` and `--log` in the
  test folder; three copies of one ARW, so one burst, and a NEF): `tools/list`
  gave the twelve tools; every `navigate` direction moved as its key does;
  `select_photos` with two paths made the first current and `get_view.selected`
  list both in strip order; `set_judgment` without paths judged the selection;
  `reject_rest` rejected the two other frames (`xmpDM:good="False"` in their
  `.xmp`), `undo` restored them, `redo` rejected them again; `undo` on an empty
  history answered `nothing to undo`, `reject_rest` outside a burst `the
  current photo is not part of a burst`.
- **Git Bash mangles backslashes** in an inline `python -c` or a here-string
  that builds the `\\?\` paths; write the JSON-building script to a file and
  call it.

## Step 2: Sort, filter, panes and overlays

- **Sort and filter become menu clicks.** `companion::set_sort` /
  `set_filter` answer `Action::Menu(Vec<menu::Action>)`: the clicks that turn
  the filter in force into the asked one (`toggles`, the menu's `Reset` first
  when `reset` is given). `menu_action`'s tail moved into `apply_menu`, which
  both the menu click and the tool run, so the `sortOrder` / `filter` writes
  and the `kept_anchor` refilter are one path. A call that changes nothing
  answers no action (D4).
- **The sort in a view-only folder:** the Sort menu's button is disabled
  there (`status_row`'s `!self.view_only`), so `set_sort` refuses it, and
  `get_view.sort` now reports the order in force (`viewonly::sort_for`,
  `capture`), not the user's kept key.
- **`recommended` is a `set_sort` value refused with "not implemented in
  Handpic's native app yet"** (D6, its natural place being this step's tool).
  The `Genre` filter section was not added to `set_filter`: it needs the
  ranking's label list, so Step 8 adds it with the other AI refusals.
  Grayscale (the momentary hold landed in #1125) is not in `set_overlays`
  either: a hold has no on / off state to set; Step 8 decides.
- **Pane widths and the strip height are excluded** from `set_panes` as
  pointer-only geometry, like the window's; Step 8's exclusion list takes
  them (`docs/agents/native-app.md` says so).
- `filter.rs`'s item name tables (`FLAGS`, `ORIENTATIONS`, `CANDIDATES`,
  `EYES`, `LABELS`) are now `pub`, so `set_filter` and `get_view.filter` use
  the stored names; `Peaking` got an `is_on`, and the `d` key's toggle moved
  into `toggle_perf`.
- **End to end** (`--mcp-port 41968`, fresh `--settings` seeded with
  `mcpEnabled`, six ARW / NEF copies of four cameras): `get_view` listed the
  folder's EXIF groups under `filter_offers`; `set_filter` with two cameras
  and `picked` / `untagged` hid the rejected current file and landed on its
  neighbour; repeating it changed nothing; `set_sort rating`, `set_panes`
  and `set_overlays` were reported back by `get_view`, and the settings store
  held `sortOrder` `"rating"`, the `filter` sections and `panels`;
  `recommended` and `output` (no `Output` folder) were refused; `reset`
  showed all six again.
- **A Bash heredoc holding a large Python script failed** with "unexpected
  EOF while looking for matching `'`" twice; writing the script with the
  Write tool and running the file worked.
- The first `mise run ci:local` failed on clippy's `explicit_auto_deref`
  (`Toggle::Label(*label)` over `filter::LABELS`, a `&[&str]`); the
  `cargo test` run before it had compiled it fine.

## Step 3: Folders, the tree, `get_status` and `wait_for_idle`

- **`open_folder` refuses a file** rather than opening its parent as a drop
  does (`dropped::dropped_dir`): a client has `list_folders`, and opening a
  folder other than the one named would surprise it. The backend checks the
  path is absolute and a folder on the blocking pool; the view strips the
  `\\?\` prefix (`context::shown_path`) so the tree's `root_of` and
  `ancestors_within` see the spelling they expect. Opening the folder the tree
  shows open changes nothing, as a click on that row only toggles it.
- **`get_status` is `companion::Status`**, built by `status_of` in
  `main.rs`. Steps 5 to 7 add the operation under way as another field (and
  to `Status::idle`); today `running` is only `IdleGate::in_flight`, since
  every run on the open folder's files enters it. A listing is
  `Resync::busy` with no scan pass and nothing in flight: `Resync` stays busy
  from the open through pass 2's end (`finished`), and a run holds it too.
  The meta pane's notes moved into `pane_notes` / `pane_errors`, which the
  pane and `get_status` both read, so they cannot drift.
- **`wait_for_idle` without polling:** GPUI's `Context::observe_self` runs
  after every `cx.notify()` of the view, and every idle transition (a scan's
  `finished`, a run's end) notifies. `observe_self` needs the entity, which
  does not exist yet inside `cx.new`'s closure (`Riffle::new` runs there), so
  it is registered from a spawned task, as `read_folder_counts` is. A timer
  per call replies with the status as it is at `timeout_ms`.
- **Read-only requests skip the busy guard** (`companion::READS`:
  `get_view`, `get_status`, `wait_for_idle`); `list_folders` never reaches
  the view. `reveal` and the tree actions stay guarded like any key.
- **`tree_folder` replies with the row, also on a no-op.** The first end to
  end run answered a repeated `collapse` with `get_view`, since a call that
  changes nothing has no action; the reply is now chosen by the request kind.
  `collapse_all` leaves the folder itself expanded (`Tree::collapse_all`, as
  the menu item does), which the tool description now says.
- **A test folder under a hidden folder is never drawn in the tree.** The
  first end to end run used the scratchpad under `AppData` (hidden on
  Windows): `open_folder` and the scan worked, but no tree row existed for
  `tree_folder` to act on. `docs/agents/native-app.md` has a rule for it.
- **End to end** (`--mcp-port 41969`, `mcpEnabled` seeded into a fresh
  `--settings`, three ARW copies in `a`, a NEF in `b`, an empty `b\c`):
  `get_status` / `wait_for_idle` answered idle
  with no folder; `list_folders` gave the home and the drives, then a folder's
  children; a relative path and a file were refused; `open_folder` replied at
  once with `listing: true`, `get_status` then showed `scanning 0/3`, and
  `wait_for_idle` returned idle 0.6 s later; a 1 ms `wait_for_idle` right
  after an open returned `idle: false` with the scan pass; reopening the open
  folder changed nothing; `tree_folder` matched `d:/.../PHOTOS/` to the row
  `D:\...\photos`, collapsed and expanded it (repeats unchanged), ran
  `expand_all`, `refresh` and `collapse_all`, refused `expand_all` on `C:\`
  and an undrawn row; `reveal` opened Explorer on a folder and a strip file
  (`\\?\` path) and refused a file outside the open folder and a missing path.

## Step 4: Settings, and the `mcpAllowDestructive` switch

- **One `apply_setting` for the modal and the tool.** The `settingsview::Change`
  match moved out of the `subscribe_in` callback into `Riffle::apply_setting`
  (taking the `WindowAppearance` the theme applies under); the callback keeps
  only `ClearCache` and `Closed`, which need the window or are not settings.
  `set_settings` answers `Action::Settings(Vec<Change>)`, so a tool's change
  and the modal's are the same values through the same method. `Change` now
  derives `Debug, PartialEq` for the companion's tests.
- **The theme over MCP has no `Window`.** `mcp_request` reads the appearance
  through the stored `AnyWindowHandle` (`self.window.update(..)`), falling
  back to `cx.window_appearance()` should the window be borrowed. The modal's
  path still passes `window.appearance()`, unchanged on Linux, where the two
  can differ.
- **Rules the modal never needed.** The modal's keys come from a real
  keystroke, so it never sees a malformed key; a client can send `Ctrl+K` or
  `alt+ctrl+k`, which the backend's `Keymap::add` accepts and which would
  never fire. `companion::named_key` requires the `keys::key_name` round trip
  and one key after the modifiers. The label names are refused while the
  format (after the call's own switch) writes no XMP, as the modal hides them.
  A call is atomic: one refused field refuses all of it.
- **`mcp_allow_destructive` is refused twice**: `SettingsArgs` is
  `deny_unknown_fields` (rmcp reports "unknown field" as the tool error) and
  the companion refuses the key should it ever arrive. `get_settings` is a
  read (`READS`), answered while the modal is open; `set_settings` is refused
  then (Busy) and while a folder operation runs (`FolderOps::busy`).
- `mcpEnabled` has no tool (read only at startup; a client would cut its own
  connection): Step 8's exclusion list takes it. The Cache page's `Clear
  Cache` (landed in #1128 after the plan's D6 listed it as missing) is an
  action, not a setting, so Step 4 left it alone; Step 8 decides its tool.
- **End to end** (`--mcp-port 41970`, a fresh `--settings` seeded with
  `mcpEnabled`, no folder open): `get_settings` gave the defaults; one
  `set_settings` with a light theme, auto-advance off, `k` added to `pick`,
  the pattern `%Y/%m`, a root and the `ja` preset answered the new values and
  logged the label names switch; repeating part of it changed nothing; `dop`
  with label names, `q`, `Ctrl+K`, `%H`, a relative root,
  `mcp_allow_destructive` and an empty call were refused with their messages;
  the store then held `labelNames`, `autoAdvance`, `shortcuts`
  (`{"pick":["p","k"]}`), `importRoot`, `importPattern`, `theme` and
  `sidecarFormat` `both`.
- Python run with `-I` ignores `PYTHONIOENCODING`; `-I -X utf8` prints the
  Japanese preset names on a cp932 console.

## Step 5: Move to Trash, Delete Permanently, and undo / redo of trash runs

- **The confirmation mechanism for Steps 6 and 7.** `companion::Op` is the
  destructive operation (today `Op::Trash(trash::Target)`), with `tool()` and
  `content()` (the dialog's title, line, run label and what it acts on). The
  companion answers `Action::Preview(Op)` (the view stores it as
  `companion::Pending { id, op }` under `mcp_next_id`, one at a time, a new
  preview replacing it, and replies `companion::previewed`), `Action::Ask(Op)`
  (the view opens the app's dialog and replies `companion::awaiting`) or
  `Action::Run(Op)` (a call with `id` and `confirm: true` matching
  `View.pending`; the view forgets the `Pending`, runs it and replies with
  `get_status`). A later step adds an `Op` variant, its arm in `tool` /
  `content`, and its arms in `mcp_request`'s `Ask` / `Run`.
- **An MCP-confirmed run does not drive `TrashFlow`.** The plan suggested
  driving the flow's own transitions; `trash.start` + `confirm_run` would
  leave the flow `Running` with no dialog, and the view draws
  `modal::backdrop` and `key_down` drops every key while `trash.busy()`, so
  the window would dim and go deaf with nothing on it. `confirm_run`'s tail
  became `request_run(target, own)`; the MCP run calls it with `own` false,
  the state a held run drained at a scan's end is already in (the flow idle,
  `trashing`, the gate's in-flight count and `resync.hold` set by
  `run_trash`). With no dialog to refuse by, the companion refuses a second
  `trash_photos` on `View.trashing`.
- **`get_status.operation`** comes from `operation_of`: the dialog asking,
  the gate's held label, `running_op` (set by `run_trash`, `undo_trash`,
  `redo_trash`) with `trashing`'s progress, else `last_op`, which
  `record_op` fills from the step note in `trashed` / `restored` only when
  `running_op` was set (a Delete Sidecars run goes through `trashed` too and
  is not reported until Step 6 sets its label). Asking and held count as not
  idle, so `wait_for_idle` after an `Ask` returns once the user answers.
- **A confirm re-checks the preview**: refused when the open folder changed or
  a previewed photo is no longer shown (a re-listing or a filter change in
  between), "preview again".
- **End to end** (`--mcp-port 41971`, a fresh `--settings` seeded with
  `mcpEnabled` and `mcpAllowDestructive` true, five ARW copies under
  `D:\handpic-mcp-e2e\step5\photos`): `tools/list` gave 25 tools; a preview
  of two paths answered id 1 with the dialog's text; `id` without `confirm`,
  an unknown id, `confirm` with `mode: delete` and a path outside the folder
  were refused; the confirm answered `running 0/2`, `wait_for_idle` returned
  `finished` "Moved 2 files to the Trash" and both files were gone; reusing
  id 1 was refused; `undo` restored both ("Restored 2 files from the
  Trash"), `redo` moved them again as `Redo Move to Trash`, a second `undo`
  brought them back; `trash_photos` with `mode: delete` answered
  `awaiting_confirmation`, a screenshot showed the Delete Permanently dialog,
  `get_status` was not idle, `navigate` was refused as busy, and the app was
  killed without a click: no file was deleted.
- Git Bash plus `json.loads` of a command-line argument mangles the verbatim-prefixed
  paths again; the test client substituted a `%P%` placeholder with the
  JSON-escaped folder prefix instead.

## Step 6: Folder operations I: Rewrite Sidecars, Delete Sidecars, Sequence JPEG Timestamps

- **A folder operation's preview is asynchronous, so the call replies at
  once (D2).** Unlike the trash dialog's text, the sidecar and sequencing
  previews are background reads (a sequencing preview of a large folder can
  outlast `BRIDGE_TIMEOUT`), and the sidecar request may be held by the gate
  for the scan. The preview call is `Action::Folder`, which runs `folder_op`
  (the menu item's own entry, gate and all) and replies with `get_status`
  (`previewing` / `held_for_scan`); the content reaches the client through
  `get_status`: `operation.dialog` while the app's dialog asks (grant off),
  or the new top-level `pending` (grant on), which `wait_for_idle` then
  returns with.
- **The flow does the preview in both modes; a marker reroutes it.**
  `mcp_folder: Option<FolderOp>` is set after `folder_op` only while
  `mcpAllowDestructive` is on; `sidecars_previewed` / `sequence_previewed`
  build the same `Dialog` as always and, when the flow's op equals the
  marker, end the flow and keep the `Dialog` as `Op::Folder(op,
  Some(dialog))` in the `Pending` instead of opening it. So the preview a
  client confirms is byte for byte the dialog's content, and no dialog-less
  flow stays busy waiting for a confirm that may never come. `folder_op`
  clears the marker (a menu click always gets its dialog), as does switching
  the grant off.
- **The confirm restarts the flow with no dialog** (`run_confirmed`:
  `start` → `previewed` → `run_sidecars` / `run_sequence`), so the UI's run
  rules hold: a sidecar run with a listing or scan under way is refused with
  `SCAN_RUNNING` (detected as the flow ending inside `run_sidecars`; the
  `Pending` is put back and the reply points to `wait_for_idle` and the same
  id), Delete Sidecars goes through `trashed` and is recorded for undo. The
  confirm's `HELD_REFUSAL` is not applied (these runs never enter the gate),
  but a sidecar *start* goes through `gate_request` and would drop a held
  operation, so it is refused with `HELD_REFUSAL` while `view.held`.
- A sequencing run's state is read from the flow (`sequence_running`) when an
  overlapping trash run owns `running_op`, and `sequence_ran` records the
  result whether or not the slot still names it.
- `trash_label` became `held_name` with the sidecar arm, so a held sidecar
  request dropped by an open, a later request or the drain is reported
  `failed` too. `record_dropped` now goes through a new `record_failed`.
- A sidecar `Pending` is dropped on a sidecar format switch (its counts and
  the files a Delete Sidecars run would move are of the old format); a
  folder open now drops only a trash `Pending`, since a folder operation's
  folder need not be the open one.
- **`cancel_operation` passes the busy guard** (the dialog it closes is what
  makes the view drop keys). `cancel_of` in the view decides: the trash
  dialog asking, a folder dialog asking or a sequencing run (dialog or none:
  `dismiss_folder_dialog`'s sequence arm sets the cancel flag without a
  dialog), else the `Pending`, else a refusal naming the operation under way.
  An import run (Step 7) is today reported as "the folder operation under
  way" cannot be canceled when its dialog is aside.
- **End to end** (`--mcp-port 41972`, fresh `--settings` seeded with
  `mcpEnabled` (and `mcpAllowDestructive` for the first run), three ARW
  copies and JPEGs extracted from them with `exiftool -b -PreviewImage`, one
  `DateTimeOriginal` stamped on all, under `D:\handpic-mcp-e2e\step6`):
  grant on: `rewrite_sidecars` replied `previewing`, `wait_for_idle` gave
  the `pending` rows and id, the confirm `running` then `finished` "Rewrote
  the sidecars of 2 files in photos", reusing the id was refused;
  `delete_sidecars` previewed, confirmed, moved both `.xmp` ("Moved 2
  sidecars to the Trash") and `undo` restored them; `sequence_timestamps`
  previewed the three rows, `cancel_operation` forgot the pending, a second
  `cancel_operation` answered "nothing to cancel"; the confirmed run wrote
  `jpegs-sequenced` with :58 / :59 / :00; with 403 JPEGs a
  `cancel_operation` mid-run reported `running 152/403` and the run ended
  "canceled, 176 of 403 written"; `delete_sidecars` right after an open
  (files touched to force a re-scan) answered `held_for_scan` during
  `analyzing`, and the preview landed as `pending` id 6 after the scan.
  Grant off: `delete_sidecars` opened the dialog (screenshot), `get_status`
  gave `awaiting_confirmation` with the dialog's rows and notes, a confirm
  and `navigate` were refused as busy, `cancel_operation` closed it with
  nothing moved; `sequence_timestamps` opened its dialog likewise.
- Git Bash mangles `\\` in a JSON argument even inside single quotes; the
  test client took `%photos%` / `%jpegs%` placeholders it expanded itself.

## Deferred issues (todo candidates)

- **Pending manual check: `open_folder` deferred by a run and refused during
  a rename.** Both go through the UI's own code (`open_from_tree` →
  `open`'s `deferred_open`, and `refuse_renaming`), but neither state could
  be reached end to end without the user's input. On Windows, run the native
  app with MCP on a test port as `docs/agents/native-app.md` prescribes, with
  a folder of test copies open: (1) start a Move to Trash of a few files and,
  while its run is in flight, call `open_folder` on another folder: expect
  `opening` set to it and `idle: false` in the reply, then `wait_for_idle`
  returning with that folder open; (2) rename a folder in the tree and, while
  the rename is in flight (a slow share helps), call `open_folder` on it:
  expect `A folder is being renamed; try again in a moment.` Step 3's
  checkbox was ticked on the automated criteria.
- **Pending manual check: the busy guard's wiring in the running app.** The
  refusal rules are unit-tested in `crates/native/src/companion.rs`
  (`a_busy_view_refuses_every_request_but_get_view`), but the `busy` value
  `mcp_request` builds in `crates/native/src/main.rs` was not exercised end to
  end, since opening the settings, a trash dialog, a folder dialog or an inline
  rename needs the user's input. On Windows (or any platform), run the native
  app with MCP on a test port as `docs/agents/native-app.md` prescribes, then:
  open the settings (`ctrl+,`) and call `navigate {"to":"next"}`: expect
  `Handpic's settings are open; try again once the user closes them`, while
  `get_view` still answers; repeat with the strip's `Move to Trash…` dialog
  open, a folder menu dialog (e.g. `Delete Sidecars…`) open, and a folder's
  inline rename open, each expecting its own message. Step 1's checkbox was
  ticked on the automated criteria.
- **An empty lens label in the filter's EXIF groups.** Found by Step 2's end
  to end check: `get_view.filter_offers.exif.lens` listed `""` for a folder
  where a compact camera's ARW (`DSC-HX95` / `DSC-HX99`) reports an empty
  lens string, so the Filter menu presumably draws a blank `Lens` item too.
  `exif::exif_key` (`crates/native/src/exif.rs`) takes `Exif.lens` as is;
  check whether the backend's `exif` reader should give `None` for an empty
  string, or the key skip it, and whether the Tauri `exif.ts` behaves the
  same.
- **Pending manual check: the MCP settings page and a live theme over MCP.**
  The switch's rule and store key are unit-tested (`settings.rs`,
  `settingsmodal.rs`'s tab order) and the `set_settings` writes were checked
  end to end, but the page and the repaint need eyes. On Windows, run the
  native app with MCP on a test port as `docs/agents/native-app.md`
  prescribes: (1) open the settings (`ctrl+,`), pick the MCP page between
  Import and Appearance, flip "Let MCP clients run destructive operations
  without asking" on, close, and call `get_settings`: expect
  `mcp_allow_destructive: true` and `mcpAllowDestructive` `true` in the store;
  reopen the page: the switch shows on; (2) with the settings closed, call
  `set_settings {"theme":{"mode":"light","light":"Ayu Light"}}`: expect the
  window to repaint in Ayu Light at once; (3) open the settings and call
  `set_settings {"auto_advance":false}`: expect `Handpic's settings are open;
  try again once the user closes them`, while `get_settings` still answers.
  Step 4's checkbox was ticked on the automated criteria.
- **Pending manual check: Move to Trash over MCP with
  `mcpAllowDestructive` off, and the redo refusal.** Unit-tested in
  `crates/native/src/companion.rs`, and the dialog path was seen end to end
  for a delete only. On Windows, run the native app with MCP on a test port
  as `docs/agents/native-app.md` prescribes, with `mcpAllowDestructive`
  off and a folder of test copies open: (1) call `trash_photos` on one photo:
  expect `awaiting_confirmation` and the Move to Trash dialog with its run
  button focused; click Move to Trash: `wait_for_idle` returns `finished`
  "Moved 1 file to the Trash"; (2) call `undo`: the file comes back; call
  `redo`: expect the refusal starting "the next redo moves files to the Trash
  again without a dialog"; (3) open the delete dialog over MCP, click
  Delete Permanently once: the file is deleted and `get_status` reports
  `Delete Permanently` `finished`; open it again and press Cancel:
  `wait_for_idle` returns idle with nothing deleted. Step 5's checkbox was
  ticked on the automated criteria.
- **Pending manual check: a folder operation confirmed in the app's dialog
  after an MCP call.** The dialog path was opened and canceled over MCP end
  to end, but a click on its run button needs the user. On Windows, run the
  native app with MCP on a test port as `docs/agents/native-app.md`
  prescribes, `mcpAllowDestructive` off, with a folder of test copies holding
  judged photos and their `.xmp`: call `delete_sidecars {"path": <folder>}`,
  wait for the Delete Sidecars dialog, click Move to Trash: expect
  `wait_for_idle` to return `finished` "Moved N sidecars to the Trash" and the
  `.xmp` files gone; repeat with `rewrite_sidecars` (click Rewrite: `finished`
  "Rewrote the sidecars of N files in <folder>") and `sequence_timestamps`
  (click Run: the `-sequenced` folder written and revealed). Step 6's
  checkbox was ticked on the automated criteria.
