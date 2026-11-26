# Learnings

## Step 1

- The datadir test builds the expected `.iss` string with a Rust raw string
  inside `format!` (`r#""{{localappdata}}\{IDENTIFIER}\update""#`), so the
  backslashes stay literal and only `{IDENTIFIER}` is interpolated.
- No PR check runs the uninstaller (`Package` runs on dispatch or from
  `release.yml` only), so the `datadir.rs` test is the only automated
  evidence; the uninstall itself stays a manual check.

## Deferred issues (todo candidates)

- Pending manual check, on Windows (already recorded as the one item of the
  `todo.md` section "App: the Inno Setup uninstaller leaves the update staging
  folder (native-release)", which this step rewrote): with an Inno Setup
  install made by a setup built from this change and an update staged by it,
  uninstall from Settings → Apps; expect
  `%LOCALAPPDATA%\dev.minodisk.handpic\update` gone and `index.sqlite`,
  `settings.sqlite` and the logs kept. Then record the result in
  `docs/plans/20261109-gpui-port/packaging-updater.md` and replace the
  **Unverified** note in `docs/agents/app.md`. Basis: Step 1's Done-when;
  files `crates/app/packaging/windows/handpic.iss`, `docs/agents/app.md`.
  The step's checkbox was ticked on the automated criteria.
