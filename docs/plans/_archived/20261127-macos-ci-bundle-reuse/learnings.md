# Learnings

## Step 1

- mise's usage spec takes `flag "--features <features>"` next to the
  existing `arg "[profile]"` and sets `usage_features`, unset when the flag is
  not given; `mise run app:bundle:macos debug --features screenshot` and the
  flag before the profile both parse (checked on a scratch task with mise
  2026.9.5 on Windows, and with `mise run app:bundle:macos --help`).
- The task passes the flag through positional parameters (`set --`, then
  `set -- --features ...`, `cargo build "$@"`) rather than a bash array: the
  macOS system bash is 3.2, where `"${arr[@]}"` of an empty array fails under
  `set -u`.
- Besides the sentences the plan named, one more became false: under "The
  main view opens on GPUI's test platform", the rule said a test-only GPUI
  API in non-test code fails only in a plain build, "the macOS job's bundle
  step". With `screenshot` that step has `test-support` on, so CI no longer
  catches it; only `Package`'s release build does. The sentence now says so.
- No local cargo build was made (the user's preference; GitHub Actions
  verifies). The PR's macOS job is the first build of the `screenshot`
  feature on macOS in CI.
- `mise run ci:local` selects the app's clippy and tests for a `mise.toml`
  change, so it ran the whole app test suite. The first run failed one test
  untouched by this change,
  `folderops::tests::canceling_a_folder_dialog_drains_a_held_clear` ("the
  quit handlers took 2.074111s", the bound in `crates/app/src/uitest.rs` at
  line 303), on a loaded host while the first run also compiled; the second
  run passed all 1165.

## Deferred issues (todo candidates)

- No CI job before `Package` now builds the app without `test-support`, so a
  test-only GPUI API used in non-test code (the rule under "The main view
  opens on GPUI's test platform" in `docs/agents/app.md`) surfaces only in the
  `Package` workflow. Basis: Step 1's switch of the bundle step in
  `.github/workflows/ci.yml` to `--features screenshot`. Possible follow-up: a
  cheap `cargo check` without features on one leg, if its cost (a second
  GPUI check variant) is acceptable. Files: `.github/workflows/ci.yml`,
  `docs/agents/app.md`.
