# Learnings

## Step 1

- No path collision with the Tauri deb turned up: the app deb ships
  `/usr/lib/handpic/handpic`, `dev.minodisk.handpic.desktop`, the
  `dev.minodisk.handpic.png` icon and `/usr/share/doc/handpic/`, none of which
  a `Package: riffle` deb names, so no `Replaces` was needed.
- The PR's own checks do not build the deb; only a `Package` run dispatched on
  the branch exercises the `linux.sh` and "Check the .deb" changes. The caller
  dispatches it after the push.
- Local verification was limited to shellcheck on `tools/package/linux.sh` and
  the Markdown checks; no cargo build (CI verifies).
