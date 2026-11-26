# Learnings: macOS real-device checks through MCP (2026-11-26)

## Step 1

- What MCP plus the shell closed on macOS: the gpui-kit text / shader check,
  the trash undo of a rejected file, `rejectRest`, the scan's QoS branch, the
  Dock / Finder icon and the bundle's launch and `codesign`, and most of the
  tree counts, themes, bursts and packaging checks. What it cannot close: a
  key press, a right-click, a dialog button, the settings box (no MCP tool
  opens it), Finder's "Put Back", and Gatekeeper on a local build, which
  carries no `com.apple.quarantine` attribute.
- The setup worth keeping: every launch with its own `--settings`, `--index`,
  `--log` and `--mcp-port`; the bundle's executable
  (`Handpic.app/Contents/MacOS/Handpic`) run directly to get its stdout; a
  release build for anything timed, since a debug build took about 5 s per
  file in pass 2 and is unusable for the QoS check.
- Side finding: the `screenshot` tool returns the last drawn frame, and a
  window fully covered by other windows does not draw, so bring the window to
  the front before a capture. No `todo.md` item was filed for it.
- Every page during the scan passes was `source=read`, every page after them
  `source=prefetch`.
- The local package's name carries the crate's version (2.1.0), not the
  `0.0.0` the `todo.md` packaging boxes still name; those boxes were left as
  they are.
- The Windows reference figure `18.4 / 25.0` in `results.md` comes from
  `docs/humans/performance.md` (the table after "While the folder scan's
  second pass (faces, focus, eyes) runs"), not from `docs/agents/app.md`,
  where the plan looked for it.
- `results.md` refers to other plans and docs by backticked paths only: a
  relative link written from this folder would break once the wrap-up moves
  it under `_archived/`.
- The pass-1 row of the QoS table has no max in the run's notes (only the
  first page's 271.7 ms); `results.md` says so rather than taking the first
  page as the max.
