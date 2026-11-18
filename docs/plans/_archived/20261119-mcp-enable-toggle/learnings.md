# Learnings

## Step 1

- `--mcp-port 0` works: the backend binds `127.0.0.1:0` and the running
  state's `port` is the one it got, so the UI test opens the view with
  `Args { mcp_port: Some(0), .. }` and reads the port from the reported
  state instead of pre-checking a free port. The page's URL shows the running
  state's port, else the configured one (`settingsmodal::mcp_port`).
- `Bridge::reply` with an id nobody waits for is a no-op (the pending map
  has no entry), so a reply after the server stopped needs no guard. The
  server's `Mcp` stays alive in `Control` for the session, so replies always
  have a bridge to go to. `idle_waiters` are left as they are on a disable:
  they are answered at idle or at their timeout, harmlessly.
- The settings view's URL `Input` is synced in `SettingsView::render`
  (`show` sets the value only when it differs), since the main view's state
  handler (`scanned`) has no `Window` to set an `InputState` with.
- Python's text mode on Windows writes CRLF; edit files with
  `newline=''` (or the Edit tool) to keep the repository's LF endings.
