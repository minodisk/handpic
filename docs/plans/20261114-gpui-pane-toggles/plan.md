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

# GPUI pane toggles and Open Folder

## Purpose

The native app (`crates/native`) lists the keymap actions `open`,
`toggleRight`, `toggleStrip` and `toggleSides` in its Keyboard Shortcuts tab
(`settingsmodal.rs`) and lets them through the folder tree's gate
(`treekeys.rs`'s `TREE_PASSTHROUGH`), but `key_down` in
`crates/native/src/main.rs` has an arm only for `toggleLeft`: the others fall
through to `key: {action} is not handled yet` and do nothing. The Tauri app
runs all of them (`crates/app/ui/src/main.ts`: `openFolder`, `changePanels`
over `panels.ts`) and remembers which panes are hidden in the `panels` key of
its settings store (`panels` / `set_panels` / `panels_setting` in
`crates/app/src/commands.rs`).

Once this is done, in the native app:

- `open` (`ctrl+o`, `meta+o` on macOS) shows the OS folder picker and opens
  the chosen folder.
- `toggleRight` and `toggleStrip` hide and show the meta pane and the
  thumbnail strip, as `toggleLeft` already does for the tree; `toggleSides`
  (`tab`) hides or shows both side panes.
- Which panes are hidden is remembered across restarts, as in the Tauri app,
  `toggleLeft`'s included.

### Decisions (user, 2026-11-14)

- Persist all three panes under `panels`, in the Tauri key and shape,
  `toggleLeft` included.
- Include `toggleSides`.
- Under `--auto`, ignore the stored panes (start with every pane shown) and
  write nothing to `panels`, matching the session-restore plan's handling of
  `--auto`.
- Hiding the strip keeps the status row (status line and `Sort` / `Filter`).
- The stored-value rule lives in a new native `panels.rs`, not in the backend.

### What the investigation found (2026-11-14, `main` at `f942b4dd`)

- **Native today.** `Riffle` holds `left_pane: bool` (initialised `true`,
  never stored). The `toggleLeft` arm flips it, hands the keyboard back from
  the tree when the pane hides (`set_tree_keys(false, cx)`), calls
  `zoom_resized(cx)` and `cx.notify()`. `render` draws the tree with
  `.children(self.left_pane.then(|| self.tree_pane(cx)))`, the meta pane
  with an unconditional `.child(self.meta_pane())` and the strip as an
  unconditional `div().h(px(strip::STRIP_HEIGHT))`. The status row
  (`status_row`: the status line and the `Sort` / `Filter` buttons) is a
  separate row below the strip.
- **Tauri's rule.** `Panels = { left, strip, right }`; `toggle` flips one,
  `toggleSides` hides both side panes when either is shown, else shows both.
  `changePanels` applies them, blurs the tree when the left pane is hidden,
  and stores them with `set_panels`. `panels_setting` reads the stored value
  as three booleans, a missing or non-boolean entry meaning shown. The
  panels are applied once at launch from the store.
- **Tauri's `openFolder`.** It takes a token, awaits `pick_folder`, and drops
  the answer when the picker was canceled or another open took a newer token
  meanwhile; an error goes to the status line. The key does not run while a
  modal is open, which the native `key_down` already covers by returning
  early (settings open, `trash.busy()`, `folderops.dialog_open()`).
- **Native picker precedent.** `settingsview.rs`'s `choose_root`:
  `cx.prompt_for_paths(PathPromptOptions { files: false, directories: true,
  multiple: false, prompt: None })`, awaited in a `cx.spawn`, matching
  `Ok(Ok(Some(paths)))` / `Ok(Ok(None)) | Err(_)` / `Ok(Err(e))`.
- **Native open.** `open_from_tree` calls
  `self.open(PathBuf::from(path), None, None, cx).detach()`; `open` already
  defers under a run in flight (`deferred_open`), drops a held operation and
  reveals the folder in the tree. `self.opens()` is a counter bumped by every
  open.
- **Settings.** `settings.rs`'s `from_values` reads each key into `Settings`;
  writes go through `Settings::writes` (`Writes::set(key, value)`), one
  ordered writer thread drained on quit. A new native store is seeded
  verbatim from a Tauri `settings.json`, so it may already hold `panels`.
- **Guide.** `../../agents/native-app.md` states "The toggle lasts for the
  session only" for `toggleLeft`, gives the canvas sizes with the tree shown
  and hidden, names `toggleLeft` as the only action the tree gate lets
  through "today", and records how `cx.prompt_for_paths` behaved when the
  launch still used it.

## Steps

- [x] Step 1: Run `open`, `toggleRight`, `toggleStrip` and `toggleSides` in
      the native app and remember the panes
  - Done when:
    - `open` shows the OS folder picker; choosing a folder opens it as a
      click in the tree does (whole, without `--size` / `--max`, revealed in
      the tree); canceling does nothing; a folder opened another way while
      the picker was up wins over the picker's answer; a picker error
      reaches the status line.
    - `toggleRight` hides and shows the meta pane, `toggleStrip` the
      thumbnail strip, `toggleSides` both side panes (hiding both when
      either is shown, else showing both). The preview canvas takes the
      freed space, and a 1:1 crop is re-cut (`zoom_resized`).
    - A hidden left pane does not keep the keyboard, by `toggleLeft` and by
      `toggleSides` alike.
    - All four toggles and `open` run while the tree has the keyboard (they
      are already in `TREE_PASSTHROUGH`) and do nothing while the settings,
      the trash dialog or a folder dialog is open.
    - The panes are stored under `panels` as
      `{"left": bool, "strip": bool, "right": bool}` on every change and
      applied at launch; a missing or non-boolean entry means shown. A store
      seeded from the Tauri app's `settings.json` comes up with the Tauri
      app's panes.
    - Under `--auto`, every pane starts shown whatever the store holds, and
      nothing is written to `panels`.
    - Unit tests cover the pure rules: `toggle`, `toggle_sides`, the stored
      value's parse (missing, not an object, non-boolean entries, extra
      keys) and its round trip, and `settings.rs`'s load of `panels`.
    - `mise run ci` and `mise run native:check` pass.
    - `../../agents/native-app.md` and the `crates/native` part of
      `CLAUDE.md` describe the new module and the rules; the guide's
      statements that the toggle "lasts for the session only", that
      `toggleLeft` is the only action the gate lets through today, and that
      the picker is no longer used are corrected.
    - The hand check below is done on Windows and its results are in
      `learnings.md`, or, where it cannot be run by the agent, recorded there
      as pending on the user.
  - Implementation approach:
    - **A new pure module `crates/native/src/panels.rs`**, a port of
      `crates/app/ui/src/panels.ts` plus the stored-value rule of the Tauri
      `panels_setting`: a `Panels { left, strip, right }` value (all shown
      by default), `toggle(which)`, `toggle_sides()`,
      `from_setting(Option<&Value>) -> Panels` and `setting() -> Value`, in
      the style of `SortKey::from_setting` / `Stored::from_setting`. Port
      the cases of `panels.test.ts` if it exists and of
      `panels_setting_falls_back_to_shown` in `crates/app/src/commands.rs`.
      No GPUI types.
    - **`crates/native/src/settings.rs`**: `from_values` reads `panels` into
      a new `Settings.panels`. Writes use the existing `Writes::set("panels",
      ...)`; no new writer.
    - **`crates/native/src/main.rs`**:
      - Replace `left_pane: bool` with the `Panels` value, initialised from
        `settings.panels`, or all shown under `--auto` (reuse an existing
        `--auto` suppression flag if session-restore has landed one).
      - One helper for a pane change (the port of `changePanels`): set the
        panes, `set_tree_keys(false, cx)` when the left pane is now hidden,
        `zoom_resized(cx)`, write `panels` (not under `--auto`),
        `cx.notify()`. The four arms (`toggleLeft`, `toggleRight`,
        `toggleStrip`, `toggleSides`) call it; the existing `toggleLeft`
        arm's behavior must not change otherwise.
      - `render`: draw the meta pane and the strip `div` only while shown,
        as the tree already is. The status row stays.
      - The `open` arm: capture `self.opens()`, call `cx.prompt_for_paths`
        with the options `choose_root` uses, and in a `cx.spawn` open the
        first path through the same call `open_from_tree` makes, only when
        `self.opens()` is still the captured value. `Ok(Err(e))` goes to
        the status line through the existing `fail`.
    - Do not add anything to `crates/backend` or `crates/app`.
    - **Verify during implementation:**
      - Whether a second `open` press while the picker is up shows a second
        dialog. If it does, ignore the press while one is out with a flag
        on the view; if it does not, add nothing.
      - Whether the window's focus returns to the photo (`self.focus`)
        after the picker closes, so the culling keys work without a click.
      - With the strip hidden: paging, judging and the `Sort` / `Filter`
        menus still work, and showing the strip again draws it around the
        current file with its thumbnails. Gate the strip's bookkeeping only
        if the check shows a problem.
      - The perf overlay (`d`) still sits along the bottom of the canvas
        with panes hidden.
    - **Hand check (Windows), each run with `--settings <fresh test store>`
      and `--log <test file>`, never the real store:**
      1. `ctrl+o`, pick a folder: it opens and the tree reveals it. Again,
         cancel: nothing changes. Again, with the tree holding the keyboard.
      2. `ctrl+alt+arrowright`, `ctrl+alt+arrowdown`, `ctrl+alt+arrowleft`
         and `tab`, each twice, with a photo shown and again at 1:1 (`z`)
         and with peaking on (`e`): the canvas resizes, the crop is re-cut.
      3. Give the tree the keyboard, hide the left pane with `tab`: the
         arrow keys page the photos again.
      4. Hide the right pane and the strip, quit with `q`, relaunch: both
         still hidden. Repeat quitting by the title bar's close button.
      5. Put `{"panels": {"left": false}}` in a Tauri-style `settings.json`
         and launch with `--import-settings` on a new store: the tree is
         hidden, the other two shown.
      6. Rebind one of the actions in the Keyboard Shortcuts tab: the new
         key runs it.
    - macOS and Linux are covered by the CI build and the unit tests only;
      say so in the guide rather than claiming behavior there.

## Trade-offs and risks

- **The status row stays when the strip is hidden.** In the Tauri app the
  `Sort` / `Filter` menus sit in the strip's header and go with it. In the
  native app they share the status row with the status line (errors,
  progress), so only the strip's own `div` hides.
- **Two copies of the stored-value rule** (Tauri's private `panels_setting`
  and native `panels.rs`), held together by ported test cases.
- **No focus resync around the picker.** The native app has no window-focus
  rescan of the open folder, so Tauri's `settleIdle` hold is not ported.
- **Conflicts with in-flight work.** Session-restore edits `settings.rs` and
  `main.rs`; rebase before the PR and expect small textual conflicts.
- **Out of scope.** The application menu bar and the Tauri user guide.

## Progress

- (2026-11-14) Step 1 implemented; the Windows hand check is pending on the user (see learnings.md)
