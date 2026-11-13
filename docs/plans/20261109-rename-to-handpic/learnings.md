# Learnings: rename-to-handpic

## Step 1: installers and the data-dir migration

### Values read from the Riffle 1.10.0 release (published, not a draft)

Each asset was downloaded into its own empty directory and read without being
executed: the MSI through the WindowsInstaller COM object opened read-only, the
`.deb` and `.rpm` with a small Python parser of their headers.

| Asset | Field | Value |
| --- | --- | --- |
| `Riffle_1.10.0_x64_en-US.msi` | `UpgradeCode` | `{15DF1A23-08B6-56BE-B9DF-B38D92F7DD75}` |
| `Riffle_1.10.0_amd64.deb` | `Package` | `riffle` |
| `Riffle-1.10.0-1.x86_64.rpm` | `Name` | `riffle` |

The UpgradeCode also equals `uuid5(NAMESPACE_DNS, "Riffle.exe.app.x64")`, the
default the schema describes, so pinning it changes nothing for a Riffle-named
build. A debug `tauri build --bundles msi` with `--config` naming the product
Handpic produced an MSI that carries the same UpgradeCode.

### Does `/P /UPDATE` keep the app data? Yes, by the template

The `tauri-bundler` crate is not in the cargo registry here: it is not a
dependency of this workspace, and the CLI comes from npm as a prebuilt binary.
The template was therefore read as the text embedded in
`@tauri-apps/cli-win32-x64-msvc` 2.11.5, the version `pnpm-lock.yaml` pins.
The `v1.10.0` tag pins the same CLI version, so this is the source of the released uninstaller.

In `Section Uninstall` the only removal of `$APPDATA\${BUNDLEID}` and
`$LOCALAPPDATA\${BUNDLEID}` is inside
`${If} $DeleteAppDataCheckboxState = 1` / `${AndIf} $UpdateMode <> 1`.
`un.onInit` sets `$UpdateMode` from `/UPDATE`, so the branch is skipped.
Independently, `$DeleteAppDataCheckboxState` is only set by `un.ConfirmLeave`,
on a page `/P` skips. This is the template, not a run of the real 1.10.0
uninstaller; that run is in the pending manual checks below.

`CheckIfAppIsRunning` kills a running app without prompting when `$PassiveMode`
is 1, so the hook's `/P` run should not stop on a dialog there.

### What the plan did not anticipate

- `/UPDATE` skips shortcut handling in both directions. The old uninstaller
  keeps `Riffle.lnk`, and `CreateOrUpdateStartMenuShortcut` /
  `CreateOrUpdateDesktopShortcut` return early in update mode. An updater run
  would have left a dead `Riffle` shortcut and no `Handpic` one, so the app
  could not be started from the Start Menu after the relaunch. The hook removes
  the Riffle shortcuts that target the old binary (read from the old entry's
  `MainBinaryName`) and `NSIS_HOOK_POSTINSTALL` creates the new ones, only for
  the shortcuts that existed.
- The hook has to be inert while the product is still called Riffle, because
  the key it looks for would then be the app's own, and a same-name update
  would uninstall itself. Tauri's template includes `hooks.nsh` before it
  defines `PRODUCTNAME`, so a file-level `!if` on it is always true; each
  macro body holds the `!if "${PRODUCTNAME}" != "Riffle"` instead, evaluated
  when `!insertmacro` expands it.
- The macOS leftovers must not be removed when the old identifier equals the
  new one, so `run` filters that identifier out before anything else, and the
  deletion sits behind the filter.
- A failed pair does not let the app start: `main` shows an error dialog and
  exits, so nothing is created in the new folder next to the only real copy
  (an empty `index.sqlite`, a WAL, `settings.json`). The next launch resumes
  and an existing destination file always wins, which is right because it is
  either a complete file from the earlier run or Handpic's own (decision 9).
  Accepted risk (decision 9): if `new/index.sqlite` already exists without a
  `-wal`, the old `index.sqlite` is discarded but its `-wal` / `-shm` still move
  next to the other database, and SQLite may replay those frames over it. The
  scenario is rare and ordering the companions before their main file was judged
  not worth the code.
- The migration also removes a stale `<dest>.migrating` before moving, so a
  resumed run whose rename succeeds cannot leave one behind.

Compile checks, done with `pnpm tauri build --debug` and `--config`, without
installing anything: a probe script (the hook included before `PRODUCTNAME` is defined, as
in the template) with Tauri's makensis showed the hook's code present under
the name Handpic and absent under Riffle (its `Var`s were reported unused), and the WiX build succeeded with the pinned
code. The debug installers stayed in `target/debug/bundle` and were not run.

### Choices worth knowing

- A real "read-only source" is OS-specific: on Unix a `0o555` directory, on
  Windows a file held open without `FILE_SHARE_DELETE` (the read-only attribute
  does not stop a rename there). Both tests assert the failure line and that the
  source remains.
- `run` returns `Vec<Result<String, String>>` rather than the plan's
  `Vec<String>`, because `setup` logs failures and successes at different
  levels.
- `dirs` 6 is the version Tauri already resolves.

## Step 2: the product rename

- The rename was scripted over the tracked files and then checked with a grep
  of what still says riffle. Left on purpose: `riffle-core` / `-app` / `-cli`,
  `riffle_*`, `RIFFLE_*` and the tests' temp dir prefixes (Step 3);
  `LEGACY_IDENTIFIERS`, `hooks.nsh`, the `deb` / `rpm` migration values and the
  MSI `upgradeCode`; the `Upgrading from Riffle` text; `CHANGELOG.md`,
  `docs/plans/**` and `todo.md`.
- The plan's Docs bullet said `app-log.md`'s table paths stay because the
  identifier is kept. That was stale (the identifier changes), so the path
  table did change and the bullet was corrected. Its `data migration` row
  already existed from Step 1.
- `lychee` runs with `--offline`, so links to `github.com/minodisk/handpic` are
  not fetched and pass although the repository is not renamed yet.

## Deferred issues (todo candidates)

- **Pending manual check, Step 1 (a), Windows NSIS** (Step 1's checkbox stays
  unticked until (a) is done). On a machine with Riffle 1.10.0 installed by
  `Riffle_1.10.0_x64-setup.exe` and run at least once, with a Start Menu and a
  desktop shortcut: build with
  `pnpm tauri build --config '{"productName":"Handpic","identifier":"com.minodisk.handpic","bundle":{"createUpdaterArtifacts":false}}'`
  (the config file stays as it is), then run the installer as the updater does:
  `Handpic_1.10.0_x64-setup.exe /P /UPDATE`. Expected: no prompt; Settings > Apps
  lists only Handpic; `%LOCALAPPDATA%\Riffle` is gone; `Handpic.lnk` replaces
  `Riffle.lnk` in the Start Menu and on the desktop;
  `%APPDATA%\com.minodisk.riffle` and `%LOCALAPPDATA%\com.minodisk.riffle` still
  exist until Handpic's first launch, after which they are gone and the data is
  under `com.minodisk.handpic`. This also confirms that the real 1.10.0
  uninstaller keeps the app data and does not prompt (the plan's risks "Windows
  hook correctness" and "Riffle's uninstaller and the old data on Windows").
  Repeat with a double-click install for the interactive path. Do this before
  Step 1 is merged. Automated criteria (b), (c) and (d) are met.
- **Pending manual check, Step 1 (a), Windows MSI.** With Riffle 1.10.0 installed
  from `Riffle_1.10.0_x64_en-US.msi`, install the MSI from the same Handpic build
  (`--bundles msi`). Expected: Settings > Apps lists only Handpic and the Riffle
  entry is gone (same UpgradeCode, so a major upgrade).
- **Pending manual check, Step 1 (a), Linux deb and rpm.** In a VM or container
  with Riffle 1.10.0's package installed, install the Handpic package built on
  Linux with the same `--config`, by `dpkg -i` and `rpm -U` as the updater does.
  Expected: `dpkg -l | grep -E 'riffle|handpic'` (or `rpm -qa`) shows only
  `handpic`, and no file conflict stops the install. The package name `handpic`
  is inferred from `productName`; confirm it.
- **Pending manual check, Step 1 (a), Linux AppImage and macOS.** AppImage:
  replace the file as the updater does and run it; one app, and the file name
  stays `Riffle_1.10.0_amd64.AppImage` (documented, not fixed). macOS: install
  the Handpic bundle over `/Applications/Riffle.app` as the updater does; one app
  in `/Applications`, and on first launch the old folders and the decision-8
  leftovers under `~/Library` are gone.
- **Pending manual check, Linux packages still named Riffle.** While `productName`
  is Riffle the packages declare `Replaces` / `Conflicts` / `Obsoletes` against
  their own name `riffle`. Package managers are expected to ignore a
  self-reference, but this has not been run. If a release is cut between Step 1
  and Step 2, check that `dpkg -i` and `rpm -U` still update a Riffle install;
  otherwise this goes away with the rename. Basis: the plan's "Step 1 is inert"
  assumption, `crates/app/tauri.conf.json`.
- An updater run on Windows leaves `HKCU\Software\minodisk\Riffle` behind, since
  `/UPDATE` skips the template's cleanup of it. Invisible and harmless; one line
  in `crates/app/nsis/hooks.nsh` would remove it if wanted. Basis: Tauri's
  `installer.nsi`, `Section Uninstall`.
- The migration runs before `tauri_plugin_single_instance`, so two processes
  started together on the very first launch could both migrate. Neither can
  replace a file (an existing destination wins; a source gone missing is
  reported and retried), so it was left alone.
  Basis: `crates/app/src/migrate.rs`, `crates/app/src/main.rs`.
- **Pending manual check, Step 2, first launch on real Riffle data** (Step 2's
  checkbox stays unticked until this is done). On a machine with Riffle 1.10.0's
  data, run this build. Expected: the data is under `dev.minodisk.handpic` and no
  `com.minodisk.riffle` folder remains, with the open folder, ratings, settings,
  ranking, model and viewed history intact, and no data-migration error dialog.
  The automated criteria are met; `mise run ci` passed on this step.
- **Pending manual check, Step 2, macOS first-launch text.** On a Mac with
  macOS 15 or later, launch an unnotarized Handpic build. Expected: the launch is
  blocked and System Settings > Privacy & Security offers Open Anyway, as
  `README.md` and `docs/humans/usage.md` now say; on macOS 14 or earlier,
  right-click > Open works. The text was not verified by the agent, and the
  landing page (Step 4) copies it.
- **Pending manual, Step 2, before merging.** The trademark gate, Step 1's
  criterion (a) checks above, and renaming the GitHub repository (then
  `curl -sI https://github.com/minodisk/riffle/releases/latest/download/latest.json`
  must show a `301`).
- The app icon still reads `RIFFLE 60FPS CULLER`, drawn into the film border by
  `tools/macos/app-icon/gen.py` and `icon.svg`. The artwork is name-bearing; the
  icons and the shipped PNGs are kept for now because regenerating needs macOS
  (DIN Condensed Bold, Chrome), so the old name is visible at large icon sizes.
  Basis: `tools/macos/app-icon/gen.py` lines 113 and 119, `crates/app/icons/`.
- `crates/native` (the GPUI port's spike) keeps `struct Riffle` and the
  `riffle-native` crate name; its window title already says Handpic. Step 3 of
  `plan.md` renames the crate and its `riffle-core` dependency. Basis:
  `crates/native/src/main.rs`.
