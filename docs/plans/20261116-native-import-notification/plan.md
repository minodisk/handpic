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

# Native Import Notification

## Purpose

An `Import…` run from an SD card can take minutes, and the user often switches
to another application while it copies. Today the only sign that it ended is
the status line inside the Handpic window. This work makes the GPUI app
(`crates/native`) post one OS system notification when an import run ends
while its window is not active, so the user learns the result (copied,
already there, failed, or canceled) without switching back to look.

The scope is the import only: not the scan, `Sequence JPEG Timestamps…`, or
the moves to the Trash. The Tauri app (`crates/app`) is frozen and is not
touched.

## Steps

- [x] Step 1: Post an OS notification when an import run ends while the window is inactive
  - Done when:
    - An import run that ends while the main window is not active posts
      exactly one OS notification whose title names the outcome and whose
      body is the same summary the status line shows (`import::done_status`:
      copied count and size, `, K already there`, `, J failed`, or the
      canceled wording). A run that ends while the window is active posts
      none.
    - The decision and the text live in one pure function in
      `crates/native/src/import.rs`, with unit tests covering: active window
      gives nothing; inactive gives a title and body for a plain success, for
      a run with skipped and failed files, and for a canceled run; the body
      equals `done_status` of the same `Ran`.
    - The app sets its process identity at startup, so an unpackaged Windows
      build can deliver the toast.
    - `CLAUDE.md`, `docs/agents/native-app.md`, `docs/humans/usage.md` and
      `docs/humans/usage.ja.md` describe the notification.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Manual check on Windows (by the user if the agent cannot observe it):
      start an import, switch to another application, and see the toast
      appear at the end; clicking it brings the Handpic window forward; an
      import that ends with the window active shows no toast. The PR
      description says whether this was done or is pending on the user.
  - Implementation approach (as far as it is known; omit if unknown):
    - No new dependency. The pinned `gpui-component =0.9.1` and
      `gpui-pre =0.3.12` provide it: `window.push_notification(note, cx)`
      (`WindowExt`) with a `Notification` built with `.title(..)`, a message
      and `.system()`. System-only delivery: no in-app toast, since the
      status line already shows the result when the user comes back. The
      component takes the title and the message as the toast's title and
      body, and a click on the toast activates the app and this window.
    - Do not call `cx.on_system_notification_response`: `gpui_component::init`
      installs the one global response handler, and a second call would
      replace it.
    - Give the notification a fixed id (`Notification::id::<T>()` with a
      private unit type), so a later import's notification replaces an
      earlier one that is still in the notification center instead of
      stacking.
    - `crates/native/src/import.rs`: add the pure function next to
      `done_status`, for example
      `done_notification(ran: &Ran, window_active: bool) -> Option<(String, String)>`
      returning the title and the body, `None` when the window is active.
      The body is `done_status(ran)` so the two cannot drift. Titles:
      `Import finished`, and `Import canceled` when `ran.canceled`. Add its
      tests to the module's existing `tests`, which already build `Ran`
      values for `done_status`.
    - `crates/native/src/folderops.rs`: post from `import_ran`
      (around line 840), after `self.folderops.import.done(run)` accepted the
      run, so a stale run posts nothing. `import_ran` has only a
      `Context<Self>` and is called from a `cx.spawn` task, so it has no
      `Window`; both the active check (`window.is_window_active()`) and
      `push_notification` need one. `crates/native/src/modal.rs` shows the
      existing pattern for this: an `AnyWindowHandle` kept from
      `window.window_handle()` and a `cx.defer` that runs
      `handle.update(cx, |_, window, cx| ..)` after the current update. The
      step decides between reusing a stored handle that way and passing a
      window into the run's task (`cx.spawn_in`); either way the check and
      the post happen in the same window update. The synthesized
      "the import stopped unexpectedly" `Ran` also goes through `import_ran`
      and so is covered without extra code.
    - `crates/native/src/main.rs`: call
      `cx.set_app_identity(datadir::IDENTIFIER, "Handpic")` in the `run`
      closure before `cx.open_window` (gpui documents it as "once, early in
      startup, before opening windows or posting notifications"). Verified
      in `gpui-pre-windows-0.3.12`: the window's `app_id` does not provide
      this; without the call the platform logs
      `cannot show a system notification without an app identity` and drops
      the toast. On Windows the call sets the process AppUserModelID and, on
      the first toast, writes
      `HKCU\Software\Classes\AppUserModelId\dev.minodisk.handpic` with
      `DisplayName`; on Linux it only gives the notification its app name.
    - What to verify during implementation: that the toast really appears on
      Windows from a `mise run native:dev` / `native:release` build (the
      manual check above); that the taskbar icon and grouping of the running
      app look unchanged after the AppUserModelID is set; and that
      `Handpic.log` shows no `failed to register AppUserModelID` or
      `cannot show a system notification` warning.
    - No gating on `--auto` and no setting to turn the notification off:
      neither was asked for.
    - `crates/native` has no `gpui::test` harness (no `test-support`
      feature), so the post itself is not unit-tested; do not add the
      feature for this.
    - Docs: in `CLAUDE.md`, extend the `crates/native` entry for
      `src/import.rs` ("the port of `import.ts`") with the end-of-run
      notification's decision and text, and mention the app identity set at
      startup where the window's `app_id` is described. In
      `docs/agents/native-app.md`, add a section in the file's existing
      `### … (Measured)` / `(Inferred)` style recording the app identity
      requirement, the system-only delivery, the inactive-window rule and
      the platform caveats (macOS posts only from the `.app` bundle in a
      trusted location and asks for authorization on the first post; Linux
      needs a notification daemon and cannot retract or replace). In
      `docs/humans/usage.md`, add the notification to the `Import…` section
      where the end of a run is described (around "At the end the status
      line shows"), and make the same change in `docs/humans/usage.ja.md`
      in Japanese in the same PR.

## Trade-offs and risks

- **A canceled run**: decided with the user: notify on every end while the
  window is inactive, a canceled run included, under the title
  `Import canceled`. This covers a cancel followed by switching away.
- **Title wording**: `Import finished` / `Import canceled` with
  `done_status` as the body. Failed files then appear only as `, J failed`
  in the body. A third title for a run with failures is possible but was not
  asked for.
- **System-only delivery through the component**: `.in_app_and_system()`
  would also leave an in-app toast that duplicates the status line when the
  user returns. Raw `cx.show_system_notification` needs no `Window` but a
  click on its toast does nothing, since the component's response handler
  ignores tags that are not its own.
- **Setting the app identity changes Windows behaviour beyond toasts**: the
  process gets the explicit AppUserModelID `dev.minodisk.handpic`, which the
  taskbar uses for grouping and pins, so an existing pin of the exe may no
  longer group with the running window. The first toast writes one HKCU
  registry key that nothing removes. The toast shows the name `Handpic` with
  a generic icon, since no icon is registered; registering one is out of
  scope.
- **"Not active" is read once, at the run's end**: a window in the
  background or minimized gets the notification; a user who returns a moment
  later still finds it in the notification center, and it is not retracted
  on activation.
- **Delivery is not guaranteed and not automatable**: Windows Focus Assist /
  Do Not Disturb hides the toast (it is posted silent in any case); macOS
  posts only from `Handpic.app` in a trusted location, after the user allows
  it; Linux needs a notification daemon. The acceptance therefore rests on
  the unit tests of the decision and text plus a manual check on Windows.

## Progress
