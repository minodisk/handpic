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

## Deferred issues (todo candidates)

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
