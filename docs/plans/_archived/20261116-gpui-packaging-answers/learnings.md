# Learnings

- (Step 1) The usage guide's "Updating" is a paragraph lead in
  `docs/humans/usage.md` (around line 1151), not a heading; the record cites
  it by that word. Its text names the AppImage as the only self-updating
  Linux format and a `.deb` / `.rpm` install as updated by hand, while
  `tauri-plugin-updater` 2.12.0's `install_deb` runs `pkexec dpkg -i`
  (confirmed in the local cargo registry copy). Both are recorded and the
  real behavior is tagged unverified.
- (Step 1) A long Python heredoc through the Bash tool failed with
  "unexpected EOF while looking for matching `''"; plain Edit calls were used
  instead.

## Deferred issues (todo candidates)

- The gpui-port plan's Progress entry for 2026-11-14 sub-milestone 1
  (`../20261114-gpui-ci-packaging/plan.md`) and the 2026-11-16 re-decision
  entry (`../20261116-gpui-packaging-redecision/plan.md`) point at plan
  folders that are archived (or will be) under `../_archived/`. Left as
  history by this plan's "Trade-offs and risks". Basis: this plan's
  implementation of Step 1. File:
  `docs/plans/20261109-gpui-port/plan.md` (Progress section).
