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
