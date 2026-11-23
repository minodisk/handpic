<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Capital executable name (`Handpic.exe`, `Contents/MacOS/Handpic`)

## Purpose

The 2.0.0 Windows packages install `handpic.exe` and the macOS bundle holds
`Contents/MacOS/handpic`. The product is named Handpic, and the user wants the
shipped executable to carry that name: `Handpic.exe` in the portable ZIP and
the Inno Setup install, `Contents/MacOS/Handpic` in `Handpic.app`. Linux keeps
the lowercase command name `handpic` (tarball, deb, `install.sh`).

The updater today looks its executable up in the release archive by the
running exe's own file name, which is why an already-shipped 2.0.0 portable
copy cannot update into a ZIP holding `Handpic.exe` (accepted by the user: a
manual reinstall, no workaround in the ZIP). This work also makes the engine's
updater independent of the running exe's file name, so no later rename or
renamed copy breaks an update again.

## Decisions (with the user, 2026-11-23)

- Scope: Windows and macOS only; Linux stays `handpic`.
- 2.0.0 portable copies fail to self-update into the new ZIP; accepted, they
  reinstall by hand. No workaround in the ZIP.
- An existing portable `handpic.exe` keeps its name after a self-update (no
  case-only rename in the engine).
- `.github/**` is not changed.
- `docs/humans/usage.md` and `usage.ja.md` gain one sentence: a 2.0.0
  portable copy must be replaced by a fresh download.

## Steps

- [x] Step 1: Ship `Handpic.exe` on Windows and `Contents/MacOS/Handpic` on
      macOS, and make the engine's updater find its executable by a fixed
      per-install-kind name
  - Done when:
    - `tools/package/windows.sh` produces a portable ZIP whose entries are
      `Handpic/Handpic.exe` and `Handpic/licenses/...`, and a setup that
      installs `{app}\Handpic.exe`, with the Start menu shortcut, the
      `[Run]` relaunch and `UninstallDisplayIcon` all naming `Handpic.exe`.
      Verified locally on Windows with `mise run app:package:windows` and a
      listing of the ZIP (`tar -tf`), recorded in `learnings.md`.
    - `mise run app:bundle:macos` lays out `Contents/MacOS/Handpic`,
      `crates/app/packaging/macos/Info.plist` has `CFBundleExecutable`
      `Handpic`, and `tools/package/macos.sh` checks that path.
    - `tools/package/linux.sh`, `crates/app/packaging/linux/**` and the
      `app:install:linux` task are unchanged; the Linux binary is `handpic`.
    - `crates/engine/src/update.rs` no longer derives any archive path or
      `bin_name` from the running exe's file name, and its tests cover:
      - the archive lookup by the fixed name: a ZIP holding only
        `Handpic/Handpic.exe` updates a portable install, and the tarball's
        `Handpic/bin/handpic` updates a tarball install;
      - a portable install whose exe is `handpic.exe` updated from that ZIP
        (new bytes, and the file keeps the name `handpic.exe`, asserted from a
        directory listing, not from `Path::exists`, which ignores case on
        Windows and macOS);
      - a portable install whose exe is `Handpic.exe` updated from that ZIP;
      - a ZIP holding only the old `Handpic/handpic.exe` installs nothing
        and leaves the exe as it was (the fixed name is what is looked up).
    - The `datadir.rs` packaging tests pass with `CFBundleExecutable`
      `Handpic` and the `.desktop` file's `Exec` still the Cargo bin name.
    - The docs name the shipped executable correctly (list below), with
      `docs/humans/usage.md` and its `.ja.md` pair in sync, including the
      sentence for 2.0.0 portable users.
    - The pending hand checks below are added to `todo.md` in its existing
      section format (Background, Files, TODO).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **Rename in packaging only; keep the Cargo bin name.**
      `crates/app/Cargo.toml` keeps `[[bin]] name = "handpic"`, so
      `cargo run`, `target/<profile>/handpic(.exe)`, the mise tasks, the
      Linux packages, `tools/package/assets.sh` and every dev-build mention
      in `docs/agents/app.md` stay as they are. The asset names do not
      change.
    - **`tools/package/windows.sh`**: keep `binary="handpic.exe"` as the
      name of cargo's output under `target/release`, and add a second
      variable for the shipped name `Handpic.exe`. Stage
      `Handpic/Handpic.exe` (the stage is a fresh `mktemp -d`, so bsdtar
      writes the entry with that casing). Pass the shipped name to ISCC as
      `Binary`, and the cargo output's name as a separate define for the
      `[Files]` source.
    - **`crates/app/packaging/windows/handpic.iss`**: the `[Files]` entry
      takes its source from the cargo output's name and sets
      `DestName: "{#Binary}"`. `[Icons]`, `[Run]` and `UninstallDisplayIcon`
      already use `{#Binary}`. Update the header comment's defines. `AppId`
      must not change.
    - **Measure Inno's upgrade over an existing lowercase file** before
      deciding on `[InstallDelete]`. Use a throwaway `.iss` with its own
      `AppId` and a `DefaultDirName` in a scratch folder (never the real
      `dev.minodisk.handpic` install): install a file as `x.exe`, then run a
      second setup installing it with `DestName` `X.exe`, and list the
      folder. If the old casing is kept, add
      `[InstallDelete] Type: files; Name: "{app}\handpic.exe"` to
      `handpic.iss`. If Inno already writes the new casing, add nothing.
      Record the measurement in `learnings.md`.
    - **macOS**: `tools/package/macos.sh` `binary="Handpic"`; the
      `app:bundle:macos` task in `mise.toml` copies
      `target/$profile/handpic` to `$app/Contents/MacOS/Handpic`;
      `crates/app/packaging/macos/Info.plist` `CFBundleExecutable`
      `Handpic`.
    - **`crates/app/src/datadir.rs` tests**:
      `the_macos_bundle_names_the_identifier_and_the_binary` compares
      `CFBundleExecutable` with the shipped name `Handpic` instead of
      `env!("CARGO_BIN_NAME")`. The Linux `.desktop` test keeps
      `env!("CARGO_BIN_NAME")`.
    - **`crates/engine/src/update.rs`**:
      - `portable_asset` drops its `exe_name` parameter and returns fixed
        paths by install kind (not by `cfg`, so the tests run on every
        platform): `Handpic/Handpic.exe` for `Portable`,
        `Handpic/bin/handpic` for `Tarball`.
      - `install` and `install_app` stop reading `exe.file_name()`.
        `self_update`'s required `bin_name` becomes the constant `"handpic"`
        `check` and `stage` already pass.
      - `bin_install_path` stays the `exe` the app passes (the real file,
        whatever its casing), never `exe.with_file_name(..)`.
      - `detect`, `setup_args` and `run_setup` need no change.
      - Update the test fixtures to the shipped names, keeping one portable
        install fixture on `handpic.exe`.
      - No change is expected in `crates/app/src/main.rs` or
        `crates/app/src/update.rs`; verify by reading.
    - **Docs**:
      - `docs/agents/app.md`: the `Handpic.app` section
        (`Contents/MacOS/handpic`), and the updater rules where they
        describe the archive path or `bin_name`. Add the rule that the
        shipped name differs from the Cargo bin name, where each lives, and
        that the updater looks up the fixed name. Leave every
        `target/debug/handpic.exe` / dev-build mention.
      - `docs/plans/20261109-gpui-port/packaging-updater.md`: only the
        statements of the current state, with a dated note of the rename.
        Leave the measured rc rows as history.
      - `docs/humans/usage.md` and `docs/humans/usage.ja.md`: the portable
        ZIP's `handpic.exe` in the install, SmartScreen and update sections,
        plus the sentence that a 2.0.0 portable copy is replaced by a fresh
        download.
      - `CLAUDE.md`: one clause where `windows.sh` / `macos.sh` or
        `update.rs`'s `install` is described, saying the packages ship
        `Handpic.exe` / `Contents/MacOS/Handpic` from the `handpic` bin.
      - `README.md`, `README.ja.md`, `.claude/skills/release/**`,
        `docs/plans/_archived/**` and `docs/plans/review-history/**`: leave.
    - **Pending hand checks to add to `todo.md`**:
      - Windows: over a real 2.0.0 Inno Setup install, run the new setup
        (by hand, then through the updater with `Restart Now`). Expect
        `Handpic.exe` in the install folder, the Start menu shortcut
        starting it, Settings → Apps showing Handpic with its icon, the
        relaunch starting the new version, and the uninstall removing the
        folder's files and the entry.
      - Windows: a portable copy whose exe is `handpic.exe`, running a build
        with this change, updates from a test feed
        (`tools/package/test-release.sh`) to a newer build; expect the new
        version after `Restart Now` and record the file's name afterwards.
        The same with a `Handpic.exe` copy.
      - Windows: a 2.0.0 portable copy offered the new release fails its
        install and stays on 2.0.0, as accepted; record the message shown.
      - macOS: `mise run app:package:macos`, open the DMG's `Handpic.app`;
        expect it to launch, `Contents/MacOS/Handpic` on disk and
        `codesign --verify --strict` passing. A 2.0.0 `Handpic.app` in
        `/Applications` updates to the new bundle and restarts into it.
      - A hand dispatch of the `Package` workflow on the branch: its logs'
        listings show `Handpic.exe`.
    - PR title suggestion: `fix(app): ship the executable as Handpic.exe
      and Contents/MacOS/Handpic`.

## Trade-offs and risks

- **An existing `handpic.exe` keeps its name after a portable update.**
  `self_replace` writes back onto the on-disk spelling; a rename would need
  extra engine code for a nearly empty population (2.0.0 portable copies
  cannot self-update at all and are reinstalled by hand).
- **`.github/**` unchanged.** The checks that spell `handpic.exe` /
  `Contents/MacOS/handpic` are case-insensitive where they run and keep
  passing, but no automated check asserts the capital; it is confirmed
  locally and from a `Package` run's listings.
- **2.0.0 portable copies (accepted).** They look for `Handpic/handpic.exe`
  in the ZIP by an exact match and fail.
- **2.0.0 `.app` on a case-sensitive volume.** `self_update`'s bundle mode
  requires the new bundle to hold a file at the running executable's
  relative path (`Contents/MacOS/handpic`). On the default case-insensitive
  APFS the update works; on a case-sensitive volume it fails and the user
  reinstalls from the DMG.
- **Inno's handling of the existing lowercase file is unmeasured** at
  planning time; the step measures it with a throwaway script. The real
  upgrade over 2.0.0 stays a hand check.
- **Shortcuts the user made by hand** keep pointing at `handpic.exe`;
  Windows resolves it to `Handpic.exe`.
- **Cargo bin rename not taken**: it would rename the Linux binary and every
  `target/` path, and likely trip the crate-name lint.

## Progress

- (2026-11-23) Step 1 complete (hand checks pending in todo.md)
