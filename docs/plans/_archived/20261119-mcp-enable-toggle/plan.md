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

# MCP enable switch and server state on the settings' MCP page

## Purpose

A user reported "I enabled MCP in settings but no URL is shown". The native
app's MCP page holds only the `mcpAllowDestructive` switch (`Let MCP clients
run destructive operations without asking`); `mcpEnabled` has no UI and is
read only at startup, so the user took the permission switch for the enable
switch.

Once this is done, the MCP page has a switch for `mcpEnabled` that starts and
stops the server without a restart and is written to the settings store, and
it shows the server's URL and whether it is running, stopped or failed to
start (say, the port is in use). MCP clients still cannot change `mcpEnabled`.

## Steps

- [x] Step 1: Add the `mcpEnabled` switch, the server URL and the server state
      to the MCP page, switching the server at runtime
  - Done when:
    - On the MCP page, a switch labelled `Let MCP clients connect` above the
      existing destructive switch starts the server when turned on and stops
      it when turned off, with no restart, and each flip writes `mcpEnabled`
      to the settings store. A launch still starts the server when the stored
      value is on.
    - The page shows the endpoint URL, `http://127.0.0.1:<port>/mcp`, with the
      `--mcp-port` port when given, else `MCP_PORT`, also while the server is
      stopped, in a form the user can select and copy.
    - The page shows the state: running, stopped, or the bind error when the
      server could not start. After a failed start the switch stays on and
      the stored value stays on.
    - `mcpAllowDestructive` behaves as before.
    - A client cannot change `mcpEnabled`: `set_settings` with `mcp_enabled`
      is refused (a test pins it), `companion.rs`'s `Excluded` reason for
      `mcpEnabled` no longer says "read only at startup", and the backend's
      `get_settings` description no longer says Handpic reads it at startup.
    - Tests: unit tests of the page's pure texts (the URL, each state's
      text); a UI test in `crates/native/src/uitest.rs` that opens the view
      with a chosen port, turns the switch on, waits for the running state
      and connects to the port, turns it off and finds the port closed, and
      reads `mcpEnabled` back from the settings store; a test of the failed
      state against a port another listener holds.
    - `CLAUDE.md`, `docs/agents/native-app.md`, `README.md` with
      `README.ja.md`, and `docs/humans/usage.md` with `docs/humans/usage.ja.md`
      describe the switch and the page instead of "the app has no switch for
      it yet" and "read only at startup".
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Server control, `crates/native/src/mcp.rs`: today `start` returns
      `None` when off and otherwise spawns the `mcp` thread, which runs
      `Mcp::switch(true)` once and waits for the quit signal. Make it
      switchable: the thread takes switch commands over a channel and runs
      the backend's `Mcp::switch(enabled)` for each, in order. Keep "off at
      launch means no runtime and no thread": spawn the thread on the first
      enable and keep it for the session. Register the quit handler once, not
      per enable; it keeps the bounded `STOP_WAIT` stop and its
      `quit: MCP server stopped=...` log line.
    - State back to the view: after each switch the thread sends the returned
      `McpState` into the view's channel as a new `scan::Event` variant, next
      to `Event::Mcp`. `McpState`'s fields are private in
      `crates/backend/src/mcp.rs`; make them readable (public fields or
      accessors), nothing else in its shape changes. Keep the
      `mcp: {state:?}` log line per switch.
    - The view, `crates/native/src/main.rs`: a new
      `Change::McpEnabled(bool)` handled in `apply_setting` next to
      `Change::McpAllowDestructive`: log it, set `settings.mcp_enabled`,
      write `mcpEnabled` through `settings.writes.set`, send the switch to
      the server, and on a disable clear `mcp_pending` and `mcp_folder` as
      the destructive switch's off branch does. The view keeps the last
      reported state, drops it on every flip, and accepts a state only when
      its `enabled` equals the setting in force, so a late answer of an
      earlier flip is not shown. It hands the state to the settings view the
      way `show_index_size` and `show_clear` hand theirs.
    - `self.mcp` is `Option<Server>` and every reply site is
      `if let Some(server) = &self.mcp`. Check what a reply to a call whose
      server has stopped does (`Bridge::reply` with an id nobody waits for)
      and what becomes of `idle_waiters` on a disable.
    - The page, `crates/native/src/settingsview.rs` `fn mcp`: the new switch
      as the existing one is built (`SettingField::switch`, a
      `flip_*` method emitting the `Change`), `mcp_enabled` added to what
      `open` reads from the main view, the URL in a read-only `Input` like
      the Import page's `root_field`, and the state as text.
    - Pure texts in `crates/native/src/settingsmodal.rs`, next to
      `ALLOW_DESTRUCTIVE_LABEL`: the label and note, the URL of a port, and
      the state text as a function of the switch and the last state: off is
      stopped; on with a running state is running; on with an error is the
      error; on with no state yet is starting. The URL's port is the running
      state's when there is one, else `--mcp-port` or `MCP_PORT`.
    - A failed bind keeps the setting on (the backend's `Mcp::switch` rule:
      the next launch tries again); the user retries by flipping off and on.
    - Clients: `SettingsArgs` in the backend is `deny_unknown_fields` and has
      no `mcp_enabled`, so no refusal code is needed, only the test. Update
      the `Excluded` reason in `companion.rs`'s coverage table to the user's
      switch on the MCP page plus the client cutting its own connection, and
      the matching sentences in `docs/agents/native-app.md`. `get_settings`
      still does not report `mcp_enabled`; only its description's sentence
      changes.
    - UI test: `Ui::open` passes `Args::default()`, whose port would be
      41917, which an installed Handpic may hold. Give the harness a way to
      open with a port. Verify whether `--mcp-port 0` is accepted (the
      backend then reports the port it bound); if not, bind a free port
      first, release it and pass its number. For the failed state, hold a
      listener on the port passed.
    - Docs to change: `CLAUDE.md` (the `settingsview.rs` MCP page sentence,
      the `src/mcp.rs` "started only while `mcpEnabled` is on" sentence),
      `docs/agents/native-app.md` (the MCP companion section's "With it off
      or missing there is no runtime, no thread and no server", the
      `set_settings` paragraph and the excluded list), the four user docs'
      "no switch for it yet" passages and the endpoint bullet's "the log
      says why", which becomes the page.

## Trade-offs and risks

- One step instead of the usual three to seven: the server control, the page
  and the docs are not useful apart, and a first PR with a runtime switch
  nothing calls could not be checked by a user.
- Thread lifecycle. Taken: no thread until the first enable, then it stays
  for the session. Not taken: always spawning it (simpler, but an app with
  MCP off gains a runtime and a thread, against what
  `docs/agents/native-app.md` records), and tearing it down on each disable
  (more lifecycle code and a quit handler per enable).
- A failed start. Taken: the switch and the stored value stay on and the
  page shows the error. Not taken: flipping the switch back off, which would
  lose the retry at the next launch and differ from the backend's rule.
- The label. Taken: `Let MCP clients connect`, the removed Tauri app's,
  which the user docs already name as what an earlier install's value came
  from.
- `get_settings`. Taken: `mcp_enabled` stays out of the reply, since a client
  that can ask always sees it on. Not taken: reporting it, which changes the
  reply's shape and its tests for no information.
- Refusing `mcp_enabled` over MCP. Taken: rely on `deny_unknown_fields` and
  pin it with a test. Not taken: a companion-level refusal message like
  `ALLOW_DESTRUCTIVE_REFUSAL`, which is more code for a path the backend
  already closes.
- Disabling while a client's preview is held. Taken: clear `mcp_pending` and
  `mcp_folder`, as turning the destructive switch off does. Not taken:
  keeping them, which leaves a preview no connected client can confirm.
- Risk: turning the server off drops the calls in flight; their clients see
  the connection close. This is what the user asked for by flipping the
  switch.
- Risk: the UI test binds a real loopback port. An ephemeral or pre-checked
  free port keeps it from colliding with an installed Handpic on 41917.

## Progress

- (2026-11-19) Step 1 complete
