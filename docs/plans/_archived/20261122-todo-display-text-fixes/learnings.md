# Learnings: todo-display-text-fixes

## Step 1

- The failed-item count is `Restored::failed_items` (serde-skipped like
  `back`), filled by `restore`'s loop and by `Restored::none` through
  `items`, which groups `moved` the way `restore` does: a RAW or JPEG opens an
  item, a sidecar after one joins it, a sidecar with no media file before it
  is an item of its own. `restored_status` prints it; `restored_count` and
  the `What::Sidecars` arithmetic still read `failed.len()`, which equals the
  item count for a sidecar run.
- In `restore`, a sidecar after a RAW that did not come back now asks
  `in_trash` (on Windows that removes the Recycle Bin item from the
  candidate map; harmless, nothing restores it). The existing
  `an_emptied_trash_reports_every_file` test changed its expected sidecar
  reason to `NOT_IN_TRASH`; the two `RAW_NOT_RESTORED` tests whose sidecar
  stays in the Trash are unchanged.
- Writing Windows paths with backslashes through a Python heredoc mangled
  them (`\2026` became an octal escape); edit Rust raw strings holding
  `\?\` with the Edit tool instead.
