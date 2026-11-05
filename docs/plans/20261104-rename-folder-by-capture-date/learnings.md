# Learnings

## Step 1: the planner and `date_folder_preview`

- The planner (`datefolder::plan`) takes dates, not Exif times: the command
  maps each listed file's capture time through `date_of` first (in `dated`),
  so the tests of the grouping pass `YYYY-MM-DD` strings directly.
- `date_of` refuses the all-zero `0000:00:00 00:00:00` some cameras write
  when the clock was never set, an out-of-range month or day, and any value
  that is not `YYYY:MM:DD` in its first ten bytes; such a file is undated.
- A file with an index row takes the row's `capture_time` even when it is
  `None` (a failed extraction would most likely fail again); only a listed
  file with no row is read with `reader::read_metadata`.
- A split target named like the folder itself (a folder `2026-10-31` holding
  photos of 10-31 and 11-01) is reported as a collision: the split creates
  siblings and merges nothing, so the folder itself is an existing entry by
  that name. Only the one-date case keeps `folder_target`'s case-only
  exemption (and a folder already named its one date is `Named`). Step 3's
  re-plan inherits this, so its split never moves a file into the folder it
  comes from.
- The one-date collision check reuses `rename::folder_target` and treats any
  error as the collision; its other refusals (a root, an invalid name) cannot
  happen there, since the root is refused first and a date is a valid name.
- `cargo` is not on the Git Bash `PATH` here; `mise exec -- cargo test -p
  riffle-app datefolder` runs the module's tests alone.
