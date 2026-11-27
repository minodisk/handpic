# Learnings

## Step 1

- The fix is a `min_w_0` child `div` around the string in `empty_message`
  (`crates/app/src/main.rs`); `min_w_0` alone was enough in the UI test,
  `max_w_full` was not tried. The real-font wrap is the main agent's
  screenshot check.
- Under the test platform's `NoopTextSystem` every glyph advances
  `0.6 * font_size`, 8.64 px at 14.4 px, so the no-folder text
  (`Drop a folder onto the window, or click here to choose one. Or press
  Ctrl+O.`) is 657 px: wider than the narrow preview's 208 px content box
  and narrower than the default test window's preview, so the plan's widths
  work as given.
- Without the fix the new test failed on the containment assert with the
  text at `x 271.5, 657 px x 23.5 px`, one line spilling left of the
  preview's 480 px edge; with it the test passes.
- The test calls `run_until_parked` after `simulate_resize`; whether the
  resize alone re-renders was not checked separately.
- The first attempt to edit through a `python3` heredoc in Git Bash hung
  (as `CLAUDE.md` warns: use `mise x -- python`, or the Edit tool).
- A cold `cargo test -j 2 -p handpic-app` build took about 6 minutes.
- `mise run ci:local` was refused by the session's permission classifier
  (the user's standing rule leaves local CI to the PR's GitHub Actions), so
  only `mise run fmt` and the new test ran locally; clippy, the full app
  suite and the link check are left to the PR's checks.
