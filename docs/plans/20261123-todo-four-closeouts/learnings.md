# Learnings

## Step 1: Trim the trailing spaces of the displayed lens name

- The lens is trimmed at display in `crates/engine/src/exif.rs` (`trim_end`,
  then a now-empty lens dropped), so `EXTRACTOR_VERSION` stays 13 and the
  index keeps the string as read.
- No other consumer shows the index's raw `lens`: `crates/app/src/meta.rs`
  (the `Lens` row) and `crates/app/src/exif.rs` (`Group::Lens`) both read it
  through `exif()`, and `crates/engine/src/mcp.rs`'s `lens` is only the
  filter argument, matched against those labels.
- A bare `python` in Git Bash hangs on this host, as the plan warns; use
  `mise x -- python`.
