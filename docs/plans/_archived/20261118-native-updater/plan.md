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

# The app's update flow

## Purpose

Sub-milestone 3 of Step 10 of [the gpui-port plan](../20261109-gpui-port/plan.md).
The app (`crates/app`, product "Handpic") cannot update itself:
`tauri-plugin-updater` left with the Tauri app. Once this plan is done, an
install of the app checks for a newer release at launch and on demand
and updates through the owner of its install, as
[`packaging-updater.md`](../20261109-gpui-port/packaging-updater.md) decided
and as gpui-kit's guide (<https://gpui-kit.com/docs/auto-update/>) explains:

- the portable ZIP and the `.tar.gz` install by `self_update`;
- an Inno Setup install by the setup `.exe`, downloaded in the background and
  run silently at quit;
- a `Handpic.app` by a staged swap of the whole bundle;
- a deb install by a notice with the release page, nothing installed.

Every download the app installs is verified with a project-owned key. A
restart finishes the update after the sidecar drain. No `latest.json` is read.

The plan also makes the packaging carry a prerelease version (`2.0.0-rc.1`)
and gives the updater a prerelease policy, so an rc.1 to rc.2 dry run on real
devices is possible once sub-milestone 4 has switched `release.yml`.

Out of scope (sub-milestone 4): `release.yml` attaching the artifacts and
signatures, the signing secret in release CI, `verify-release.sh`, and the
release-please version files. This plan publishes nothing.

The user approved the plan on 2026-11-18 with every recommended option in
"Trade-offs and risks" (zipsign everywhere, a user-held private key, the test
key and test feed, automatic install, binary-only portable updates, the
Updates page, an rc install following every newer version, numeric `x.y.z`
forms for an rc, an hourly launch check, state-only MCP exposure).

### What the guide prescribes and where this plan deviates

Followed as written:

- The owner table: `self_update` for a portable binary, the installer for
  Inno Setup, the whole `.app` for macOS, the package manager for a deb.
- `self_update` 1.3 with the guide's feature list.
- `checksum_from_asset("SHA256SUMS")`, `no_confirm(true)`,
  `show_output(false)`, `show_download_progress(false)`.
- The blocking work on a background worker.
- The phases `Idle`, `Checking`, `Available`, `Downloading`, `Verifying`,
  `ReadyToRestart`, `Failed`.
- The selected release pinned between the check and the install.
- Staging outside the live install.
- `Restart to finish`, with the version reported only by the new process.
- A cached last-check time and no simultaneous checks.

Deviations, each because the guide gives no answer or the recorded decision
overrides it:

1. **Publisher authenticity by zipsign.** The guide says `SHA256SUMS` does not
   prove the publisher and names "a trusted signing key" without a mechanism.
   This plan adds `self_update`'s `signatures` feature (`verifying_keys`) on
   top of the checksum.
2. **No OS code signing** (answer 3 of the record). The guide's "signed and
   notarized `.app`" and "publisher signature" on Windows become the project
   key plus an ad hoc `codesign --verify` of the staged bundle.
3. **The app drives the installer and the bundle swap itself** (answer 1 of
   the record). The guide only names the owner.
4. **Updates download and install without asking** (the record's flow, as the
   Tauri app did). The guide's snippet runs "after the user accepts" and asks
   for a stated policy; the policy is stated in the usage guide (Step 7).
5. **The check is not `is_update_available()`.** It would offer a prerelease to
   a stable install. The app filters `get_newer_releases()` by channel and
   pins the result with `release_tag`.
6. **Binary-only updates of the portable archives.** The guide says a
   multi-file archive is updated as a unit. The ZIP's `licenses/` and the
   tarball's icon and `.desktop` file are not needed to run the binary and are
   left as installed.

## Steps

- [x] Step 1: Map the version per package format and let a `Package` run take a version
  - Done when:
    - `tools/package/version.sh` prints the SemVer string as today and, by
      argument, the three mapped forms below. A version that is not `x.y.z`
      or `x.y.z-<prerelease>` is refused.

      | Form    | Stable    | Prerelease   | Used by                                              |
      | ------- | --------- | ------------ | ---------------------------------------------------- |
      | SemVer  | `2.0.0`   | `2.0.0-rc.1` | every asset file name, Inno `AppVersion`, the binary |
      | Numeric | `2.0.0`   | `2.0.0`      | `CFBundleShortVersionString`, `CFBundleVersion`      |
      | Windows | `2.0.0.0` | `2.0.0.0`    | Inno `VersionInfoVersion`                            |
      | Deb     | `2.0.0`   | `2.0.0~rc.1` | the control file's `Version`                         |

    - The deb's file name keeps the SemVer string
      (`handpic_2.0.0-rc.1_amd64.deb`); only the control field uses `~`.
    - `package.yml` has an optional `version` input. When given, a new
      `tools/package/set-version.sh <version>` writes it to
      `crates/app/Cargo.toml` and to the crate's entry in
      `crates/app/Cargo.lock` before the build, so `cargo build --release
      --locked` still passes and the binary reports that version.
    - `package.yml`'s checks assert each mapping:
      - the two plist fields are numeric;
      - the setup's `DisplayVersion` is the SemVer string and its file version
        is the Windows form;
      - `dpkg-deb --field … Version` is the Deb form;
      - `dpkg --compare-versions` puts `2.0.0~rc.2` before `2.0.0~rc.10`
        before `2.0.0`.
    - A `Package` run dispatched on the branch with `version=2.0.0-rc.1` is
      green, and one with no input is green at `0.0.0`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `tools/package/version.sh`, `tools/package/set-version.sh`,
      `macos.sh` and the `app:bundle:macos` task in `mise.toml` (whichever
      writes the plist's two fields), `windows.sh`,
      `crates/app/packaging/windows/handpic.iss` (a `VersionInfoVersion`
      define), `linux.sh`, `package.yml`, and the "Packaging" section of
      `docs/agents/app.md`.
    - `shellcheck` covers `tools/package/*.sh` through `mise run lint`.
    - `handpic.iss` sets no `VersionInfoVersion` today; Inno derives it from
      `AppVersion` only when that is numeric.
    - `build.rs` embeds a version resource through `winresource`. The
      prerelease `Package` run is what shows that a prerelease
      `CARGO_PKG_VERSION` builds on Windows.
    - `set-version.sh` is also what a tester uses for local N and N+1 builds.
      Sub-milestone 4 wires release-please to the same two files.
    - The executable is `riffle-native` today. The remove-tauri plan
      (`../20261118-remove-tauri/plan.md`, Step 2) renames it, so keep reading
      the name from each script's `binary=`.

- [x] Step 2: Add the updater's assets and the signing tooling to the packaging
  - Done when:
    - `Package` also builds the two assets the updater needs:
      - `Handpic-<version>-macos-<arch>.tar.gz`, holding `Handpic.app`;
      - `Handpic-<version>-windows-x64-setup.exe.zip`, holding the setup
        `.exe`.
    - `tools/package/sign.sh` signs the four updater archives with zipsign
      (pinned at 0.2.2) from a private key file: the portable ZIP, the Linux
      `.tar.gz`, the macOS `.tar.gz` and the setup ZIP.
    - `SHA256SUMS` is computed after signing.
    - The production verifying key is checked in at
      `crates/app/packaging/update.pub`. A test key pair is checked in
      under `crates/app/packaging/test/`, with a README saying no released
      build trusts it.
    - `package.yml` has an optional `update_test_repo` input (`owner/name`).
      - When given: the build gets `HANDPIC_UPDATE_TEST_REPO` and the archives
        are signed with the test key.
      - When not given: nothing is signed. Sub-milestone 4 signs with the
        production secret.
    - The run's checks prove, on the signed files:
      - `zipsign verify` passes with the test verifying key;
      - `Expand-Archive` still opens the signed portable ZIP;
      - `install.sh` still installs the signed tarball with the digest from
        `SHA256SUMS`;
      - the `.app` extracted from the macOS `.tar.gz` passes
        `codesign --verify --strict`.
    - A `mise` task generates a key pair (`zipsign gen-key`) for the user.
    - A `Package` run dispatched on the branch with `update_test_repo` set is
      green, and one without it is green.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged.
    - zipsign embeds its signature in the file (prepended to a ZIP, appended
      as a gzip member to a `.tar.gz`) and binds it to the file name. So sign
      before `SHA256SUMS`, and never rename a signed file.
    - The DMG, the setup `.exe`, `install.sh` and the deb are downloaded by
      people, not installed by the app. They stay unsigned and are covered by
      `SHA256SUMS` only.
    - Build the macOS tarball with `COPYFILE_DISABLE=1`, so no `._` entries
      land in it, from the ad hoc signed bundle `macos.sh` already has.
    - Decide where signing runs: in each platform job, or in the final job
      (today's `checksums`) over the downloaded artifacts. Either way the
      uploaded artifacts must be the signed bytes `SHA256SUMS` names.
    - The production private key is generated by the user and never enters
      the repository or this plan's workflows. See Trade-offs for who
      generates it and when.
    - Files: `tools/package/macos.sh`, `windows.sh`, `sign.sh`, `mise.toml`,
      `package.yml`, `crates/app/packaging/`, `CLAUDE.md`'s packaging
      paragraph, `docs/agents/app.md`.

- [x] Step 3: Add the updater to `crates/engine`
  - Done when:
    - `crates/engine/src/update.rs` detects the install kind from the
      executable's path: portable ZIP, Inno Setup, `.app`, tarball
      (`~/.local/bin`), deb (`/usr/lib/handpic`), or unknown.
    - It checks a feed (API base URL, owner, repository) for the release to
      offer, under the prerelease policy:
      - versions are compared as SemVer, so `2.0.0-rc.2` < `2.0.0-rc.10` <
        `2.0.0`;
      - a stable install is never offered a version with a prerelease part;
      - a prerelease install is offered any newer version, later rcs and
        finals alike;
      - the SemVer string decides, not GitHub's prerelease flag.
    - It installs a portable update with `self_update`, as the guide's
      snippet plus:
      - `verifying_keys` with the embedded key;
      - `release_tag` pinned to the checked release;
      - an exact asset name through `asset_matcher`;
      - `bin_path_in_archive` set to `Handpic/<binary>` for the ZIP and
        `Handpic/bin/<binary>` for the tarball.
    - A deb or unknown install gets the release and its page URL and installs
      nothing.
    - Tests run the updater against a local fake GitHub API (a loopback
      listener in the test) with archives signed by a test key made in the
      test, installing into a temp folder. They cover:
      - the policy table above;
      - a ZIP and a `.tar.gz` update;
      - up to date;
      - an unreachable feed;
      - a missing asset;
      - a wrong digest;
      - an unsigned archive and one signed by another key;
      - a release without `SHA256SUMS`.

      In every failure case nothing is installed.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 2 is merged (the key files and the asset names).
    - The updater lives in `crates/engine` like `ai.rs`: no GPUI, state handed
      to a callback, the feed and the keys as parameters so the tests inject
      theirs.
    - The app's version is a parameter; `cargo_crate_version!()` here would be
      the engine's.
    - Dependency, as the guide has it plus `signatures`:
      `self_update = { version = "1.3", default-features = false, features =
      ["ureq", "rustls", "github", "archive-tar", "compression-tar-gz",
      "archive-zip", "compression-zip-deflate", "checksums", "signatures"] }`.
    - Check with `cargo tree` what the dependency adds. The engine's
      `reqwest` deliberately uses rustls with `ring`; if `self_update` brings
      `aws-lc-sys` or a second TLS stack that breaks a platform build, record
      it and decide between the guide's `ureq` and `self_update`'s `reqwest`
      feature with the engine's client (`reqwest_client`).
    - The key and the feed: `include_bytes!` of
      `crates/app/packaging/update.pub`, and `minodisk/handpic`. With
      `HANDPIC_UPDATE_TEST_REPO` set at build time: the test verifying key and
      that repository. Log which one a build uses.
    - Release tags are `v<version>` (`include-component-in-tag` is false).
    - `self_update`'s github backend takes `api_base_url`, and plain `http` on
      loopback works in its own tests.
    - Asset names: `Handpic-<version>-windows-x64.zip`,
      `Handpic-<version>-linux-x64.tar.gz`. The macOS arch is `arm64` or
      `x86_64` (`uname -m`), not Rust's `aarch64`.
    - Decide the detection rule for Inno Setup against portable (the
      `unins000.exe` beside the exe, or the `HKCU` uninstall entry's install
      location) and record it. A debug build or a `cargo run` binary is
      `unknown`.
    - Files: `crates/engine/src/update.rs`, `crates/engine/src/lib.rs`,
      `crates/engine/Cargo.toml`, the root `Cargo.lock`, `CLAUDE.md`.

- [x] Step 4: The app's update flow: the launch check, the Updates page, the notice and the restart
  - Done when:
    - The app checks at launch on a background task. Not under
      `--auto`, not in a debug build, and not within one hour of the last
      automatic check.
    - For a portable or tarball install it downloads, verifies and installs
      without asking, then shows `Restart to finish`.
    - The settings modal has an `Updates` page showing the running version,
      the phase (`Idle`, `Checking`, `Available`, `Downloading`, `Verifying`,
      `ReadyToRestart`, `Failed` with its message) and buttons:
      - `Check for Updates`, never two checks at once;
      - `Restart Now`;
      - on a deb or unknown install, `Open Release Page`, which opens the
        release in the browser and installs nothing.
    - A failed check or download leaves the app usable and is retryable.
    - `Restart Now` quits through the existing quit handlers (scan stop,
      judgment stores, sidecar writer, settings writes) and relaunches the
      updated executable.
    - The pure parts (phase transitions, texts, which button shows per install
      kind) have unit tests, and the companion's coverage table names the new
      page.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged.
    - There is no menu bar in the app; the Updates page is "the menu
      item" of the plan's wording (see Trade-offs).
    - Follow the app's split: a pure model in `crates/app/src/update.rs`
      (as `cache.rs` is to the Cache page), the page in `settingsmodal.rs` /
      `settingsview.rs`, and the engine's events through the view's existing
      channel (`scan::Event`), as `Event::Perf` and `Event::Mcp` arrive.
    - Restart with GPUI's `cx.restart()`: it relaunches once the process has
      exited and reaches `App::on_app_quit`. Do not use
      `self_update::restart`, which `exec`s past the drain.
    - Measure that both drains log before the relaunch.
    - On Linux `current_exe()` names the replaced file with a ` (deleted)`
      suffix after the update. Read the executable's path before installing
      and hand it to `cx.set_restart_path`.
    - Remember the version this process installed, so a second check does not
      install it again (the Tauri `UpdateRun.installed` rule).
    - The last-check time: `self_update`'s `UpdateCheckGuard` with a stamp
      file beside the settings store, or a key in the store. Decide and
      record.
    - `companion.rs`'s `coverage` module is compiled for its tests and must
      map every page (`page_coverage`). The MCP companion exposes the update
      state only (in `get_settings` or `get_status`); the check and the
      restart stay the user's (see Trade-offs).
    - Add the page to `CLAUDE.md` and a section to
      `docs/agents/app.md`.

- [x] Step 5: Update an Inno Setup install: the setup downloaded in the background and run at quit
  - Done when:
    - On an Inno Setup install the app downloads
      `Handpic-<version>-windows-x64-setup.exe.zip` into an app-owned staging
      folder, checks its digest against `SHA256SUMS` and its zipsign signature
      with `self_update::verify_signature`, and extracts the setup. A failure
      at any of these runs nothing.
    - At quit, after the drains, it starts the setup with
      `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`.
    - `Restart Now` does the same and the app comes back as the new version.
    - A quit without `Restart Now` does not relaunch.
    - A stale staging folder is cleared at launch.
    - The updater's tests cover the download and both verifications against
      the fake feed, without running the setup.
    - `package.yml`'s Windows check installs version N, runs the setup of N
      again with the relaunch switch off and on, and asserts the upgrade's
      exit code and the installed files.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass, and
      a `Package` run dispatched on the branch is green.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged.
    - Measure and record how the setup, started while the app is still
      exiting, gets the exe unlocked: Inno's Restart Manager handling
      (`CloseApplications`), or a `[Code]` wait for the app's process id
      passed on the command line.
    - The relaunch belongs to the installer: a `[Run]` entry in `handpic.iss`
      that starts the app only when a command-line parameter asks for it. So a
      first install by hand and a quit with `Later` do not launch anything.
    - The setup is spawned detached and must outlive the app's process.
    - Decide where in the quit it is spawned so both `on_app_quit` drains have
      finished.
    - Staging folder: under the app's cache folder (`datadir.rs`).
    - `AppId` never changes; the per-user install needs no elevation.

- [x] Step 6: Update a `Handpic.app`: the whole bundle staged and swapped
  - Done when:
    - On a `.app` install the app updates through `self_update`'s bundle mode
      from `Handpic-<version>-macos-<arch>.tar.gz`
      (`bundle_path_in_archive("Handpic.app")`), with the same checksum,
      signature and pin as the portable path.
    - The staged bundle is checked with `codesign --verify --strict` in the
      `verify_binary` hook before the swap.
    - The executable inside the bundle is never replaced alone.
    - An app running translocated, or in a folder it cannot write, ends in
      `Failed` with a message that says to move the app to `/Applications`.
    - `Restart Now` reopens the swapped bundle.
    - A test swaps a bundle in a temp folder against the fake feed
      (`bundle_install_path`), including a rejected signature leaving the old
      bundle in place.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the
      macOS leg of the `app` job runs the bundle test).
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 4 is merged; independent of Step 5.
    - `self_update` stages inside the bundle's parent folder, restores the old
      bundle on a failed step and reports `Error::AppTranslocated` and
      `Error::InstallPathNotWritable` itself.
    - `bin_name` is still required in bundle mode; `bin_path_in_archive` must
      not be set.
    - GPUI's macOS restart runs `open` on the `.app` path, which is unchanged
      by the swap.
    - The ad hoc signature differs per build. Whether the folder-access
      grants survive the swap is a manual check of Step 7, not a gate here.

- [x] Step 7: Verify every path on a test feed, record the results and write the user docs
  - Done when:
    - `tools/package/test-release.sh` publishes the artifacts of a `Package`
      run as a release of a scratch repository (a prerelease when the version
      has a prerelease part), and its use is described in
      `docs/agents/app.md`.
    - Two test builds, `2.0.0-rc.1` and `2.0.0-rc.2`, were packaged with
      `update_test_repo` set and published there.
    - Each row below is recorded in `packaging-updater.md` as **Measured**
      with its date and machine, or left **Unverified** with the reason and
      deferred to the rc dry run:

      | Install     | Check                                                                                                                                                    |
      | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
      | Portable ZIP | rc.1 updates to rc.2 while running; `Restart Now` relaunches rc.2; the judgments made just before the restart are in the sidecars                       |
      | Inno Setup  | rc.1 downloads in the background; a plain quit installs rc.2 without a prompt; `Restart Now` relaunches rc.2; no SmartScreen or UAC prompt for a standard user |
      | `.app`      | rc.1 in `/Applications` swaps to rc.2 and reopens; from the DMG or Downloads it fails with the move message; Gatekeeper and the folder-access grants after the swap |
      | Tarball     | rc.1 from `install.sh` updates to rc.2 and relaunches                                                                                                    |
      | deb         | rc.1 shows the notice and opens the release page; nothing is installed                                                                                   |
      | Any         | a stable build on the same feed is not offered an rc; offline at launch leaves the app usable; a tampered asset installs nothing                         |

    - The record's "Unverified" items this plan answered are rewritten:
      - the install-kind detection;
      - `self_update`'s asset matching and archive paths;
      - what zipsign signs;
      - where the staged `.app` comes from;
      - the setup at quit;
      - the relaunch.
    - `docs/humans/usage.md` and `usage.ja.md` describe the app's update flow
      and its policy: automatic download and install, per install type, the
      deb notice, the project key, and that prereleases only reach
      prerelease installs.
    - The gpui-port plan's Step 10 marks sub-milestone 3 done through this
      plan and lists, for sub-milestone 4, what `release.yml` must do:
      - sign the four archives with the production key;
      - compute `SHA256SUMS` after signing;
      - attach the two new assets;
      - publish `rc.N` versions as GitHub prereleases;
      - run the end-to-end rc.1 to rc.2 dry run on real devices from the real
        repository.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 5 and 6 are merged.
    - The end-to-end rc.1 to rc.2 dry run with the production key and the
      real feed can only happen after sub-milestone 4. It is the deferred
      manual check that covers every update path, and this step's rows are
      its rehearsal with the test key.
    - Checks that need a machine nobody has at hand stay **Unverified** and
      are named in the hand-over to sub-milestone 4; do not mark them
      measured from CI output.
    - The remove-tauri plan (`../20261118-remove-tauri/plan.md`, Step 5)
      rewrites the same "Updating" section and says the app has no
      updater. Whichever lands second edits the other's text.
    - A fix found by a row goes into this step's PR only when it is small;
      otherwise add a step to this plan.

## Trade-offs and risks

- **The signing mechanism for the setup `.exe` and the `.app`.** zipsign signs
  only `.zip` and `.tar.gz`.
  - A (planned, chosen): one mechanism. The `.app` ships in a signed
    `.tar.gz`, which `self_update`'s bundle mode installs. The setup ships in
    a signed `…-setup.exe.zip`, which the app verifies with
    `self_update::verify_signature`. There is no crypto code of our own; the
    cost is two extra assets per release, the setup uploaded twice.
  - B: a detached ed25519 signature file beside the setup `.exe`, made by a
    signer of our own (zipsign's keys are raw ed25519) and verified with
    `ed25519-dalek`. No extra copy of the setup, but a signer and a verifier
    to write and review.
  - C: sign `SHA256SUMS` itself (signed metadata, which the guide names) with
    minisign and verify it before trusting any digest. It covers every asset
    and could reuse the Tauri minisign key pair, but it adds a second
    mechanism next to zipsign (the plan's item names `self_update` with
    signature verification for the archives) or replaces it with our own
    download-and-verify code.
- **The key.**
  - Planned: a new ed25519 pair in zipsign's format. The existing minisign
    key is password-protected and in another format, and nothing in the field
    needs it any more.
  - The verifying key is checked in. The private key is generated by the
    user, kept offline by the user, and added as a repository secret only in
    sub-milestone 4.
  - Open: who runs the key generation and when. Step 2 needs the public key.
    If the user prefers to generate it later, Step 2 ships the test key only
    and a non-test build refuses to update until the key lands. That adds a
    code path and delays the first real verification.
  - `verifying_keys` accepts several keys, so a rotation ships the old and
    the new key in one release.
  - A lost private key means a hand reinstall for everyone.
- **Test trust.**
  - Planned: a checked-in test key pair and a build-time
    `HANDPIC_UPDATE_TEST_REPO`. A build with it trusts a public key and
    reads a scratch repository, so it must never be distributed; the Updates
    page and the log say when a build is a test build.
  - Alternative: give `Package` the production secret. Then any dispatch on
    any branch can sign with the release key, and it moves the secret into
    this plan against its scope.
- **Automatic install against the guide's consent.**
  - Planned: download and install without asking, as the record decides and
    as the Tauri app did, with the policy stated in the usage guide.
  - Alternative, closer to the guide: stop at `Available` and install after a
    click. That is the deb path's behavior for every install, and a
    regression from today's silent updates.
- **Binary-only portable updates.**
  - Planned: `self_update` replaces the executable only, as the guide's
    snippet does. The ZIP's `licenses/` folder and the tarball's icon and
    `.desktop` file stay as first installed, so the notices can go stale.
  - Alternative: bundle mode over the ZIP's `Handpic/` folder, which would
    also delete anything the user put in that folder, and `MoveAll` over the
    tarball's three files.
- **Where "Check for Updates" lives.**
  - Planned: an `Updates` page in the settings modal, since the app
    has no menu bar.
  - Alternatives: an item in the title bar, or a keymap action. Neither
    exists as a pattern for a rarely used action.
- **The prerelease policy.**
  - Planned: an rc install follows every newer version, so rc.1 goes to rc.2
    and then to the final without a reinstall, and is a stable install from
    then on.
  - Alternative: an rc install follows only its own `x.y.z` line.
  - The SemVer string decides the channel. A final version published by
    mistake as a GitHub prerelease would still reach stable installs.
- **The numeric versions of an rc.**
  - Planned: `x.y.z.0` on Windows and `x.y.z` on macOS for every rc and the
    final. Nothing compares them; the updater compares the binary's SemVer.
  - Alternative: encode the rc number in the fourth field or the build
    number. That makes rc.1 (`2.0.0.1`) newer than the final (`2.0.0.0`)
    unless the final gets a high fourth field.
  - The deb's file name keeps `2.0.0-rc.1` so the asset names stay SemVer,
    against Debian's convention of the control version in the name.
- **The launch check's throttle.**
  - Planned: at most one automatic check an hour. The guide asks for a cached
    last-check time, and GitHub allows 60 anonymous requests an hour per
    address. The button always checks.
  - Alternative: every launch, as the Tauri app did.
- **The MCP companion.** The coverage table forces a decision for the new
  page.
  - Planned: the state only, with the check and the restart left to the user.
  - Alternative: a `check_for_updates` tool plus the state in `get_settings`
    or `get_status`.
  - A restart from an MCP client should stay the user's, as
    `mcpAllowDestructive` is.
- **The usage guide.** Step 7 and the remove-tauri plan's Step 5 edit the same
  section; see Step 7.
- **Risks to watch.**
  - `self_update` may pull a second HTTP client and TLS stack into
    `crates/engine` (Step 3 measures).
  - The Windows quit may race the setup for the exe's lock (Step 5 measures).
  - An ad hoc signed bundle may lose the folder-access grants on every update
    (Step 7 records).
  - A signed ZIP has data before its first entry, which some unzip tools
    reject (Step 2 checks Windows' own).
  - The remove-tauri plan renames the binary while this plan adds paths that
    name it.

## Progress

- (2026-11-18) Step 1 complete
- (2026-11-18) Step 2 complete
- (2026-11-18) Step 3 complete
- (2026-11-19) Step 4 complete
- (2026-11-20) Step 5 complete
- (2026-11-20) Step 6 complete
- (2026-11-21) Step 7 complete
