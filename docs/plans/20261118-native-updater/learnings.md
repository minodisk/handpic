# Learnings: the native app's update flow

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
  cache; only `handpic-native` itself rebuilds.
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
  native:keygen <private-key>` and checks in the
  `crates/native/packaging/update.pub` it writes, so that Done-when item and
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
  `native:sign` and `native:keygen`), not a `[tools]` entry, so
  `mise-action` and every developer's `mise install` do not build it; it is
  built on the first `mise run native:sign` with the `[tools]` Rust.
  `sign.sh` itself refuses another `zipsign --version`, so the pin holds when
  it is run outside mise too.
- `mise run native:sign -- a b` hands `a b` to the task's `"$@"` (the `--`
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

## Step 3: the update engine in `crates/backend`

- `self_update` resolved to 1.3.3 (edition 2024, `rust-version` 1.88). With
  the guide's features plus `signatures` it brings `ureq` 3.4 on its `_ring`
  rustls feature, the same `rustls` 0.23 the backend's `reqwest` uses, and
  no `aws-lc-sys` on any target (`cargo tree --target all -i aws-lc-sys`
  prints nothing, in the root workspace and in `crates/native`). So the
  guide's `ureq` stays; `reqwest_client` was not needed. It does add a
  second HTTP client (`ureq` next to `reqwest`), about 350 lock entries in
  `crates/native/Cargo.lock` and 700 lines in the root one.
- `crates/native/Cargo.lock` must be updated with the backend's new
  dependencies too (`cargo fetch` in `crates/native`), or `Package`'s
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
- Deviation from the plan's approach: the engine has no state callback like
  `ai.rs`. `self_update`'s `update()` downloads, verifies and installs in
  one blocking call, so the only phases the engine could report are the
  ones its caller already knows (before and after `check` / `install`).
  Step 4's pure model sets the phases around the two calls; a
  `progress_callback` for a download percentage can be added then if the
  page shows one.
- Deviation from the guide's snippet beyond the plan's list: none new. The
  version is a parameter rather than `cargo_crate_version!()` (that would
  be the backend's), as the plan said. No request timeout is set (the
  guide sets none); see the deferred issue below.
- `source()` logs `update: feed <owner>/<repo>, test|production key`;
  `HANDPIC_UPDATE_TEST_REPO` is read with `option_env!` while compiling
  the backend, which `Package` does in the same build as the app.
- The fake GitHub API is a `TcpListener` thread per test serving the
  listing, `/releases/tags/v<version>` and each asset with
  `Connection: close`; `ureq` reads plain `http` on loopback without any
  extra setting. The test key is `SigningKey::from_bytes(&[seed; 32])`
  through `zipsign-api` 0.2.2's `sign-zip` / `sign-tar` (a dev-dependency,
  with `zip` 9, `tar` and `flate2` to build the archives).

## Step 4: the app's update flow and the Updates page

- The phases are set around the engine's two calls, as Step 3 left it: the
  model goes `Checking` before `check`, `Downloading` before `install`,
  `Verifying` when `self_update`'s `progress_callback` reports the archive's
  full size (the digest, signature, extraction and replace follow inside the
  same `update()`), and `ReadyToRestart` or `Failed` on `install`'s answer.
  So `install` gained a `progress` parameter; the guide's "percentage only
  when the total is trustworthy" holds, a download without a size showing
  no percent.
- Deferred issue from Step 3 decided: the check's requests are bounded by
  `CHECK_TIMEOUT` (30 s) and the install's by `INSTALL_TIMEOUT` (10 min), set
  with the builder's `timeout` in `crates/backend/src/update.rs`. In
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
  page, as the native app has no notification for it; the reported version
  is the running one (`CARGO_PKG_VERSION`) until the new process starts.
- The MCP companion exposes the state in `get_status` (`update`); the
  coverage table excludes the Updates page with that reason.

## Deferred issues (todo candidates)

- No request timeout on the update check or download
  (`crates/backend/src/update.rs`, `configure`): `self_update`'s `timeout`
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
  `crates/native/src/update.rs`, `crates/native/src/main.rs`.
- The page's click paths (the three buttons, `Check for Updates` disabled
  while busy) were not driven headless; the hand check belongs with the
  pending manual check above. Basis: Step 4 implementation;
  `crates/native/src/settingsview.rs`.
