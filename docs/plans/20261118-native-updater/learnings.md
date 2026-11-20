# Learnings: the app's update flow

## Step 1: version mapping and the `Package` version input

- `version.sh [form] [version]` takes an optional version as its second
  argument, so `set-version.sh` validates its input through the same rule
  and the workflow's deb ordering check maps `2.0.0-rc.2` / `2.0.0-rc.10` /
  `2.0.0` through the script instead of writing the `~` forms by hand.
- The prerelease is restricted to dot-separated alphanumeric identifiers
  (no `-`, though SemVer allows one): a `-` in a deb upstream version is
  read as the revision separator, so `2.0.0~rc-1` would not be the version
  the control file meant. Leading zeros in `x.y.z` are refused as SemVer
  does. Build metadata (`+...`) is refused.
- `winresource` 0.1.34 writes `CARGO_PKG_VERSION` as the exe's
  `FileVersion` / `ProductVersion` strings and only the numeric
  major / minor / patch into the fixed `FILEVERSION`, so a prerelease
  version builds; the Windows leg checks the installed exe's
  `ProductVersion` against the SemVer string. Nothing in the binary reads
  `CARGO_PKG_VERSION` yet, so macOS and Linux have nothing to check until
  the updater reads it.
- The setup's file version is read as
  `FileMajorPart.FileMinorPart.FileBuildPart.FilePrivatePart`, not the
  `FileVersion` string, which Inno may write differently.
- `set-version.sh` writes the files back with `cat >` rather than `mv`, so
  `mktemp`'s `0600` mode does not replace the files' own.
- shellcheck's SC2016 fires on an awk program passed in single quotes that
  contains `$0`; passing the selectors as `awk -v` variables keeps the
  program a fixed single-quoted string.
- The "Set the version" step runs after `actions/cache`, so the cache key is
  the checked-in `Cargo.lock`'s hash and a versioned run still hits the
  cache; only `handpic-app` itself rebuilds.
- Inno Setup is not installed on this machine, so the `.iss` change
  (`VersionInfoVersion={#FileVersion}`) was first compiled on the `Package`
  runs.
- A change to `mise.toml` makes `mise run ci:local` select every check
  (all crates' clippy and tests); it took about 4 minutes here.
- `Package` runs on the branch at 2709745e, both green on the first try:
  - 44194426548 (`version=2.0.0-rc.1`): `CFBundleShortVersionString` and
    `CFBundleVersion` `2.0.0`, setup file version `2.0.0.0`,
    `DisplayVersion` and the exe's `ProductVersion` `2.0.0-rc.1`, deb
    `Version: 2.0.0~rc.1` in `handpic_2.0.0-rc.1_amd64.deb`, every asset
    named `Handpic-2.0.0-rc.1-...`. A prerelease `CARGO_PKG_VERSION` builds
    on Windows (`winresource`).
  - 44194431118 (no input): every asset at `0.0.0`.
  - A cold-ish release build plus the checks took about 75 minutes per run,
    the `macos-15-intel` leg queued longest.

## Step 2: the updater's assets and the signing tooling

- The production key pair is not generated here: the user runs `mise run
  app:keygen <private-key>` and checks in the
  `crates/app/packaging/update.pub` it writes, so that Done-when item and
  the Step 2 checkbox stay open until then. Nothing in this step reads
  `update.pub` (the test key signs); Step 3's `include_bytes!` will.
- zipsign 0.2.2 (`mise exec cargo:zipsign@0.2.2`, about 10 s to build here)
  writes a 64-byte private key (secret and public halves) and a 32-byte raw
  verifying key. `sign zip` / `sign tar` without `-o` sign in place, the
  `--context` defaulting to the input's file name: a renamed copy of a
  signed ZIP fails `verify` with "no matching key/signature pair found".
  Signing an already signed file again still verifies.
- Measured locally: Windows PowerShell 5.1's `Expand-Archive` opens a ZIP
  zipsign prepended its signature to (bsdtar-made, as `windows.sh` makes
  them), and GNU tar (`tar -tzvf`) and Python's `tarfile` (what `install.sh`
  uses) read a signed `.tar.gz` without a warning.
- The key files need `-text` in `.gitattributes`: under `* text=auto
  eol=lf`, 32 random bytes hold no NUL about 88 % of the time and git would
  treat them as text.
- zipsign is a task tool (`tools = { "cargo:zipsign" = "0.2.2" }` on
  `app:sign` and `app:keygen`), not a `[tools]` entry, so
  `mise-action` and every developer's `mise install` do not build it; it is
  built on the first `mise run app:sign` with the `[tools]` Rust.
  `sign.sh` itself refuses another `zipsign --version`, so the pin holds when
  it is run outside mise too.
- `mise run app:sign -- a b` hands `a b` to the task's `"$@"` (the `--`
  is dropped); a task's relative paths resolve against the repository root.
- Signing runs in each platform leg right after packaging and before the
  checks, so the checks and the uploads see the signed bytes; `sign.sh`
  verifies each file with the given verifying key right after signing, so
  `zipsign verify` with the test verifying key is proven in every leg. The
  `checksums` job installs the tarball with `install.sh` and the digest
  `SHA256SUMS` names.
- `HANDPIC_UPDATE_TEST_REPO` goes through `$GITHUB_ENV` from a conditional
  step, so a build without the input does not see the variable at all
  (rather than an empty one). Nothing reads it before Step 3.
- `Package` runs on the branch at 7e557cce:
  - 44296484243 (`update_test_repo=minodisk/handpic-update-test`), green on
    the first try: every leg built with `HANDPIC_UPDATE_TEST_REPO` set,
    `sign.sh` printed zipsign's `OK` for the Linux tarball, both macOS
    tarballs and both Windows ZIPs (building zipsign took 10 to 30 s per
    leg), the runner's `pwsh` `Expand-Archive` opened both signed ZIPs (the
    setup ZIP one entry, the setup's digest), the `.app` extracted from each
    signed macOS tarball passed `codesign --verify --strict` (no `._`
    entries), `install.sh` installed the signed tarball in the `linux` leg
    and again in `checksums` with the digest from `SHA256SUMS`, which lists
    the ten assets including the two new ones.
  - 44296501086 (no input): green after one rerun. The first attempt's
    `macos-latest` leg failed in `hdiutil create` with "No space left on
    device" while writing the DMG (before the new tarball step; the same leg
    of the other run passed), so it was taken for a runner flake and the
    failed job rerun; it passed. No signing step ran.

## Step 3: the updater in `crates/engine`

- `self_update` resolved to 1.3.3 (edition 2024, `rust-version` 1.88). With
  the guide's features plus `signatures` it brings `ureq` 3.4 on its `_ring`
  rustls feature, the same `rustls` 0.23 the engine's `reqwest` uses, and
  no `aws-lc-sys` on any target (`cargo tree --target all -i aws-lc-sys`
  prints nothing, in the root workspace and in `crates/app`). So the
  guide's `ureq` stays; `reqwest_client` was not needed. It does add a
  second HTTP client (`ureq` next to `reqwest`), about 350 lock entries in
  `crates/app/Cargo.lock` and 700 lines in the root one.
- `crates/app/Cargo.lock` must be updated with the engine's new
  dependencies too (`cargo fetch` in `crates/app`), or `Package`'s
  `cargo build --release --locked` fails; the plan's file list named only
  the root lock.
- 1.3.3 differs from older `self_update` docs: `build()` returns the
  concrete `github::Update` with inherent `update()` /
  `get_newer_releases()`, `checksum_from_asset` fails the update
  (`ChecksumSourceInvalid`, naming the asset) when the release has no such
  asset, `verify_release_digest` is on by default and also checks GitHub's
  per-asset `digest` field when present, and `bin_install_path` other than
  the running exe takes a plain move instead of `self_replace`, which is
  what lets the tests install into a temp folder. An `asset_matcher`
  replaces the target substring match entirely, so `target` is irrelevant.
- The order of checks in `self_update` is: the `SHA256SUMS` entry fetched
  before the archive download, then digest, release digest, zipsign
  signature (context: the downloaded file's name, which `self_update` keeps
  as the asset name), extraction, replace. Every failure case in the tests
  leaves the exe byte for byte as it was.
- `get_newer_releases()` already drops releases not newer than the current
  version and non-SemVer tags; `offered` applies the channel rule on top
  and takes the SemVer maximum, so the listing's order does not matter.
- Inno Setup against portable: an `unins000.exe` beside the exe is Inno
  Setup; else a `licenses/` folder beside it is the portable ZIP (the
  installer also writes `licenses/`, so the uninstaller is checked first);
  else unknown. That makes a `cargo run` or `target/release` binary
  unknown without a debug-build check here. The `HKCU` uninstall entry was
  not used: it needs the registry and would not tell a moved install
  apart. `.app` is `<x>.app/Contents/MacOS/<exe>`, the deb
  `/usr/lib/handpic/<exe>`, the tarball `<…>/.local/bin/<exe>`; these are
  path rules only, so the tests run on every platform.
- The binary's name is read from the exe path (`Handpic/<name>` in the
  ZIP, `Handpic/bin/<name>` in the tarball), so `install` needs no binary
  name parameter and follows a rename.
- Deviation from the plan's approach: the updater has no state callback like
  `ai.rs`. `self_update`'s `update()` downloads, verifies and installs in
  one blocking call, so the only phases the updater could report are the
  ones its caller already knows (before and after `check` / `install`).
  Step 4's pure model sets the phases around the two calls; a
  `progress_callback` for a download percentage can be added then if the
  page shows one.
- Deviation from the guide's snippet beyond the plan's list: none new. The
  version is a parameter rather than `cargo_crate_version!()` (that would
  be the engine's), as the plan said. No request timeout is set (the
  guide sets none); see the deferred issue below.
- `source()` logs `update: feed <owner>/<repo>, test|production key`;
  `HANDPIC_UPDATE_TEST_REPO` is read with `option_env!` while compiling
  the engine, which `Package` does in the same build as the app.
- The fake GitHub API is a `TcpListener` thread per test serving the
  listing, `/releases/tags/v<version>` and each asset with
  `Connection: close`; `ureq` reads plain `http` on loopback without any
  extra setting. The test key is `SigningKey::from_bytes(&[seed; 32])`
  through `zipsign-api` 0.2.2's `sign-zip` / `sign-tar` (a dev-dependency,
  with `zip` 9, `tar` and `flate2` to build the archives).

## Step 4: the app's update flow and the Updates page

- The phases are set around the updater's two calls, as Step 3 left it: the
  model goes `Checking` before `check`, `Downloading` before `install`,
  `Verifying` when `self_update`'s `progress_callback` reports the archive's
  full size (the digest, signature, extraction and replace follow inside the
  same `update()`), and `ReadyToRestart` or `Failed` on `install`'s answer.
  So `install` gained a `progress` parameter; the guide's "percentage only
  when the total is trustworthy" holds, a download without a size showing
  no percent.
- Deferred issue from Step 3 decided: the check's requests are bounded by
  `CHECK_TIMEOUT` (30 s) and the install's by `INSTALL_TIMEOUT` (10 min), set
  with the builder's `timeout` in `crates/engine/src/update.rs`. In
  `self_update` 1.3.3 that timeout is `ureq`'s `timeout_global` per request,
  the body included, so one bound for the download must cover the whole
  archive; 10 min is about 50 KB/s for today's archives. Not exercised by a
  test (a stalled-server test would take the timeout's length).
- The last-check time is a settings store key (`updateCheckedAt`, Unix
  seconds), not `UpdateCheckGuard`: the store already keeps session state,
  follows `--settings`, and needs no new path. It is written before the
  launch's check runs, so an offline launch is not repeated within the hour.
- The check and the install run on `std::thread`s rather than
  `background_spawn`: a download of minutes would hold one of the executor's
  pool threads.
- Inno Setup and `.app` installs stop at `Available` with `Open Release
  Page` until Steps 5 and 6 give them their owners; `update::installs`
  is the one place to change.
- Measured on Windows 11 (debug build, a temporary patch making the `--auto`
  end call `cx.set_restart_path(whoami.exe)` and `cx.restart()`, reverted):
  all three quit lines (`scan stopped`, `judgment stores drained=true
  sidecar writer drained=true`, `settings writes drained=true`) were logged
  after the restart request and the process exited 0. GPUI's Windows
  restart posts `PostQuitMessage` once its PowerShell waiter is spawned, so
  the shutdown and its handlers run as on a plain quit.
- Deviations from the gpui-kit guide recorded here beyond the plan's list:
  none of substance. The guide's "let the user defer the restart" holds
  (nothing restarts without `Restart Now`); its "tell the user when a new
  version is ready" is the pane's note naming `Restart Now` on the Updates
  page, as the app has no notification for it; the reported version
  is the running one (`CARGO_PKG_VERSION`) until the new process starts.
- The MCP companion exposes the state in `get_status` (`update`); the
  coverage table excludes the Updates page with that reason.

## Step 5: the Inno Setup install's update

- The engine's `stage_setup` does not go through `self_update`'s
  `update()` (which replaces an executable); it uses the pieces
  `self_update` 1.3.3 exports: `get_release_version("v<version>")` for the
  pinned release, `Download` (with `Accept: application/octet-stream`, the
  API asset URL being `url`, and its default User-Agent) for `SHA256SUMS`
  and the ZIP, `Checksum::from_sums_file` for the entry (its `verify` is
  crate-private, so the SHA-256 is compared here with `sha2`, now used in
  the module body, not only the tests), `verify_signature` and
  `Extract::extract_file`. The ZIP keeps its asset name in the staging
  folder, since zipsign's context is the file name.
- `self_update::verify_signature` returns `Ok` for an empty key list, so
  `stage_setup` refuses one itself (tested).
- How the setup gets the exe unlocked: neither the Restart Manager nor a
  retry, but a `[Code]` wait. The app passes `/WAITPID=<its pid>` and
  `InitializeSetup` waits on that process handle (`OpenProcess` with
  `SYNCHRONIZE`, `WaitForSingleObject`, at most 60 s) before anything is
  installed. The Restart Manager (`CloseApplications`, Inno's default `yes`,
  with `RestartApplications` `yes`) was not chosen: in a silent run it
  closes and then restarts the app itself, which would relaunch on a plain
  quit too. It stays at its defaults for a setup run by hand while the app
  runs; after a 60 s wait for a hung app it would apply there as well.
- The relaunch belongs to the setup: `[Run]` starts `{app}\<exe>` `nowait`
  only when `/RELAUNCH` is on the command line (`RelaunchAsked`, a
  `ParamStr` scan). So `Restart Now` on an Inno install sets `relaunch` and
  calls `cx.quit()`; `cx.restart()` would start the old exe while the setup
  replaces it.
- Where the setup is spawned: a third `on_app_quit` handler registered in
  `Handpic::new` after the judgment / sidecar one and the settings one.
  GPUI 0.3.12's `shutdown` calls the quit observers synchronously in
  registration order (a `BTreeMap` keyed by subscription id) before awaiting
  their futures, and both drains run in their handlers' bodies, so the
  setup is started after both have finished. It is spawned with
  `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` and the staging folder as
  its working directory; the `/WAITPID` wait covers whatever the process
  still does after the handlers. `CREATE_BREAKAWAY_FROM_JOB` was not added:
  a launch from Explorer or the Start menu is in no job.
- The staging folder (`<cache dir>/dev.minodisk.handpic/update`,
  `datadir::update_staging`) is cleared at launch only for an Inno install,
  so a test or `cargo run` binary never touches the real cache. After
  `/RELAUNCH` the relaunched app may find the setup still running from that
  folder (Inno's `setup.exe` waits for its `setup.tmp`), so the removal can
  fail; it is logged and the next launch clears it.
- Inno Setup is still not installed on this machine, so the `[Code]` was
  not compiled locally; the `Package` run compiles it. The `windows` leg's
  check now: installs N; upgrades with `/WAITPID` of a `pwsh` stand-in that
  holds the installed exe open (`FileShare.Read`, as a running image) for
  8 s, asserting exit code 0, the holder's own exit code 0 (not closed by
  the Restart Manager), the setup not finishing before the hold ends, the
  installed files, and no launch within 10 s; then upgrades with
  `/RELAUNCH` and asserts a launch within 30 s (a `handpic.exe` process from
  the install folder, or `Handpic.log` written since), stopping the app
  before the uninstall.
- Against gpui-kit's guide (<https://gpui-kit.com/docs/auto-update/>): it
  names the installer as the owner of an Inno Setup install, staging in an
  app-owned location outside the install, never running a partial payload,
  and the installer handling the file lock; all followed. Its "verify the
  publisher signature" on Windows is the project's zipsign key (the plan's
  deviations 1 and 2) and running the setup at quit without asking is the
  plan's deviation 4. No new deviation.
- The `ReadyToRestart` phase of an Inno install means "staged"; its texts
  say the update installs when Handpic quits.
- Tripped up locally: Python's text-mode writes on Windows turn each newline
  into CRLF, and a Rust string continuation (`\` at a line's end) before a CRLF
  kept the next line's indent in the string, so a text test failed on the
  first `mise run ci:local`. Edit with LF endings.
- The first `Package` run (44536402810, `version=2.0.0-rc.1`,
  `update_test_repo=minodisk/handpic`) compiled the `[Code]` and passed the
  `/WAITPID` upgrade (8.9 s against the 8 s hold, the holder's exit 0) and
  the no-launch check, then hung for 40 minutes until cancelled: pwsh 7's
  `Start-Process -Wait` waits for the whole process tree, so the `/RELAUNCH`
  upgrade never returned while the relaunched Handpic ran. `Invoke-Silent`
  now starts the setup with `-PassThru` and waits on its own handle
  (`WaitForExit(300000)`, throwing past it), the holder's wait is bounded at
  60 s, and the step has `timeout-minutes: 15`. Package run 44547625174 on
  the fix passed every job (the `/WAITPID` upgrade 9.1 s, the relaunch, the
  uninstall and the ZIP checks).

## Step 6: the `.app` install's bundle swap

- `self_update` 1.3.3's bundle mode as used: `bundle_path_in_archive("Handpic.app")`
  plus an explicit `bundle_install_path` (the `.app` taken from the exe path,
  `app_bundle`), `bin_name` set and `bin_path_in_archive` / `bin_install_path`
  left unset (either is a `ConflictingConfig` from `build()`). It downloads
  and checks the archive as in single-file mode (`SHA256SUMS`, release
  digest, zipsign), extracts the whole archive into a `TempDir` inside the
  bundle's parent, refuses an archive without the bundle folder or without a
  file at the running exe's relative path, runs `verify_binary` on the
  staged bundle root, then renames: the running exe aside, the old bundle
  into a stash `TempDir`, the staged bundle into place, reversing them on a
  failure. Both temp folders are dropped, so the parent is left holding only
  `Handpic.app` (tested).
- `self_update` reports `AppTranslocated` only while resolving its *default*
  bundle path (macOS, no `bundle_install_path`), and its
  `check_install_path_writable` probe fails only on `PermissionDenied`, so a
  read-only disk image (`EROFS`) would pass it and fail mid-install. The
  engine passes the path explicitly and checks first itself (`misplaced`):
  an `AppTranslocation` path component, or a probe file it cannot create in
  the bundle's parent, either way a `Failed` message ending "move
  Handpic.app to /Applications and open it from there, then check again";
  `self_update`'s own `AppTranslocated` / `InstallPathNotWritable` get the
  same suffix.
- `codesign_check` compiles on every target (`cfg!`, not `#[cfg]`) and
  refuses on anything but macOS, where an `App` install cannot occur. The
  verifier is a `fn` parameter of the private `install_app`, so the swap
  tests run on every platform with stub verifiers (accepting, and rejecting
  as a failed codesign would), and a `#[cfg(target_os = "macos")]` test runs
  the real `codesign_check`: it asserts that a bundle of `/usr/bin/true`
  with a minimal `Info.plist`, `codesign --force --sign -`, archived with
  `tar`'s `append_dir_all`, swaps in and still verifies, and that the same
  bundle with a byte appended to its `Info.plist` after signing is refused
  and the old bundle stays. It is compiled and run only on macOS, so it was
  not run locally (Windows) and first runs on the `test` job's
  `macos-latest` leg of the PR's checks.
- Deviation from the plan's Done-when wording: it says "the macOS leg of the
  `app` job runs the bundle test", but the engine and its tests are in
  `crates/engine`, a root-workspace member, so `cargo test` of the `test`
  job's `macos-latest` leg runs them (the `app` job tests `crates/app`'s own
  crate only). No CI change was needed.
- The restart: GPUI 0.3.12's macOS `restart` runs `open <path>` after the
  process exits, so the restart path set for an `App` install is now the
  bundle, not the executable inside it (`open` on the bare executable would
  start it through Terminal). Without a path GPUI would use
  `NSBundle.bundlePath`, the same folder; the explicit path keeps one rule.
- Against gpui-kit's guide (<https://gpui-kit.com/docs/auto-update/>): it
  asks for "a bundle-aware updater", to replace and validate the complete
  `.app` after staging and never one executable inside it, and to fail
  safely without partially replacing files; all followed. Its "signed and
  notarized" replacement is the ad hoc `codesign --verify --strict` plus the
  zipsign key (the plan's deviation 2). No new deviation.

## Deferred issues (todo candidates)

- No request timeout on the update check or download
  (`crates/engine/src/update.rs`, `configure`): `self_update`'s `timeout`
  is unset, as in the gpui-kit guide's snippet, so a stalled connection
  could keep a check in `Checking`. Step 4 ("never two checks at once")
  should decide whether to set `.timeout(..)` on the check (and a separate,
  longer one on the download). Basis: Step 3 implementation. Resolved in
  Step 4 (`CHECK_TIMEOUT` and `INSTALL_TIMEOUT`, see its section); not a
  todo.
- Pending manual check (Step 4): on a portable ZIP install (Windows) and a
  tarball install (Linux) of a release build packaged with
  `update_test_repo`, an older version launched with a newer release on the
  test feed should, without any click, go `Checking` → `Downloading` →
  `Verifying` → `ReadyToRestart` on the Updates page, with the pane's note;
  `Restart Now` should log the three `quit:` drain lines and come back as
  the newer version (on Linux through the path read at launch, not the
  ` (deleted)` one). A second launch within the hour should not check
  (`updateCheckedAt`). A deb install should show `Open Release Page` and
  install nothing. These are Step 7's rows; the Step 4 checkbox was ticked on
  the unit tests, the local checks and the Windows restart-drain
  measurement. Basis: Step 4 implementation; files
  `crates/app/src/update.rs`, `crates/app/src/main.rs`.
- The page's click paths (the three buttons, `Check for Updates` disabled
  while busy) were not driven headless; the hand check belongs with the
  pending manual check above. Basis: Step 4 implementation;
  `crates/app/src/settingsview.rs`.
- Pending manual check (Step 5, Windows, an Inno Setup install of a test
  build packaged with `update_test_repo`): with a newer release on the test
  feed, launch the older install; the Updates page should go `Checking` to
  `Downloading` to `Verifying` to `ReadyToRestart` ("installs when Handpic
  quits"), the setup staged under
  `%LOCALAPPDATA%\dev.minodisk.handpic\update`. A plain quit should log
  `quit: update setup ... relaunch=false`, install the newer version without
  a prompt (no UAC or SmartScreen) and start nothing; `Restart Now` should
  log `relaunch=true` and come back as the newer version. A tampered setup
  ZIP on the feed should end in `Failed` with nothing staged. This is Step
  7's Inno Setup row; the checkbox was ticked on the automated criteria.
  Basis: Step 5 implementation; files `crates/app/src/main.rs`,
  `crates/engine/src/update.rs`.
- Pending manual check (Step 6, macOS, a `Handpic.app` of a test build
  packaged with `update_test_repo`): with a newer release on the test feed,
  an older `Handpic.app` in `/Applications` should go `Checking` to
  `Downloading` to `Verifying` to `ReadyToRestart` without a click, leave
  only `Handpic.app` in `/Applications`, and `Restart Now` should log the
  three `quit:` drain lines and reopen the bundle as the newer version. The
  same build opened from the mounted DMG or from `~/Downloads` (translocated)
  should end in `Failed` with the move-to-`/Applications` message and change
  nothing. Also record whether Gatekeeper prompts after the swap and whether
  the folder-access grants survive it. This is Step 7's `.app` row; the Step
  6 checkbox was ticked on the automated criteria (the swap tests on every
  platform; the real `codesign` test runs on the PR's checks, the `test`
  job's `macos-latest` leg). Basis: Step 6
  implementation; files `crates/engine/src/update.rs`,
  `crates/app/src/main.rs`.
