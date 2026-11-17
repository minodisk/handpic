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

# Package the native app on a manual workflow

## Purpose

Sub-milestone 2 of Step 10 of [the gpui-port plan](../20261109-gpui-port/plan.md):
package the GPUI app (`crates/native`) with platform-native tools, as decided
in [`packaging-updater.md`](../20261109-gpui-port/packaging-updater.md), and
build the distributables from a manually triggered GitHub Actions workflow
that uploads them as workflow artifacts. Once it is done a maintainer can
download a DMG per macOS architecture, a per-user Inno Setup setup `.exe`, a
Windows portable ZIP, a Linux `.tar.gz` with its `install.sh`, a `.deb` and a
`SHA256SUMS` from one run, and the same packaging runs locally through mise
tasks on the matching OS.

Out of scope, owned by later sub-milestones: any updater code, the signatures
of the project-owned key and any asset the updater needs (3); `release.yml`,
`release-please-config.json`, `verify-release.sh` and attaching anything to a
release (4); the user-facing docs (6). `crates/app` is frozen and is not
touched; its icons are only read.

## Steps

- [x] Step 1: The `Package` workflow, the version and notices plumbing, and the macOS DMG
  - Done when:
    - `.github/workflows/package.yml` exists, triggered by `workflow_dispatch`
      only, with `permissions: contents: read`, and touches no release.
    - A run of it builds `cargo build --release --locked` on `macos-latest`
      (arm64) and `macos-15-intel` (x86_64) and uploads
      `Handpic-<version>-macos-arm64.dmg` and
      `Handpic-<version>-macos-x86_64.dmg`; each DMG holds an ad hoc signed
      `Handpic.app` whose `Info.plist` carries `<version>` in both version
      fields and whose `Contents/Resources` holds the license notices.
    - A final job of the same run uploads a `SHA256SUMS` covering every
      artifact the run built.
    - `<version>` is read from `crates/native/Cargo.toml` by one shared script.
    - `mise run native:package:macos` produces the same DMG locally on a Mac.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
      (the `native` job's macOS leg still bundles and checks `Handpic.app`).
    - A `Package` run on the PR branch is green and its artifacts were
      downloaded and inspected (see the approach for how the first run is
      triggered).
    - `docs/agents/native-app.md` ("Packaging", and the "license notices are
      due at packaging" rule), `docs/agents/github-actions-workflows.md` and
      `CLAUDE.md` describe the workflow, the scripts and the new task.
  - Implementation approach (as far as it is known; omit if unknown):
    - Split build from packaging. `ci.yml`'s `native` job records that
      `mise run native:*` did not find NASM on Windows and therefore runs
      cargo through `mise exec rust@1.98.1 --`. Keep that shape on every
      platform: the workflow builds with
      `mise exec rust@1.98.1 -- cargo build --release --locked` in
      `crates/native`, then runs a packaging script that only packages an
      already-built `target/release` binary. The mise task is build plus the
      same script.
    - Scripts live in a new `tools/package/` (`version.sh`, `notices.sh`,
      `macos.sh`; Steps 2 and 3 add `windows.sh` and `linux.sh`). `mise.toml`'s
      `lint` task and `tools/ci/local.sh` list the shellcheck folders
      explicitly (`tools/git`, `tools/stats`, `tools/ci`), so add
      `tools/package/*.sh` to both and extend `tools/ci/local.test.sh`.
    - Keep the binary's name in one variable of the scripts (`riffle-native`
      today). The rename-to-handpic plan
      (`../20261109-rename-to-handpic/plan.md`, Step 3) renames it to
      `handpic-native`.
    - Version: `version.sh` prints `[package].version` of
      `crates/native/Cargo.toml`. It is `0.0.0` today and stays so here;
      adding the file (and `crates/native/Cargo.lock`) to release-please is
      sub-milestone 4's. Do not edit `release-please-config.json`.
    - `native:bundle:macos` (`mise.toml`) stays the one place that lays out
      `Handpic.app`; extend it to write the version into the copied
      `Info.plist` (`CFBundleShortVersionString` and `CFBundleVersion`, e.g.
      `plutil -replace`) and to copy the notices. The checked-in
      `crates/native/packaging/macos/Info.plist` keeps `0.0.0`;
      `datadir.rs`'s tests read it for the identifier and the executable.
    - DMG, after gpui-kit's packaging guide minus Developer ID and
      notarization: `codesign --force --sign - Handpic.app`, verify with
      `codesign --verify --strict`, stage a folder with the app (and an
      `/Applications` symlink), `hdiutil create -volname Handpic -srcfolder …
      -ov -format UDZO`. In the workflow, check the result (`hdiutil verify`,
      or attach and test for the executable).
    - License notices, the list `packaging-updater.md` and `native-app.md`
      name: `crates/native/LICENSE-lucide`, `crates/native/themes/LICENSE`,
      `crates/core/models/LICENSE`, `LICENSE-mediapipe` and `LICENSE-siglip2`,
      and an Apache-2.0 notice for `gpui-component` and the `gpui-pre-*`
      crates (a new checked-in `crates/native/packaging/NOTICE-gpui` with the
      copyright line and the license text, taken from the crates' own
      `LICENSE-APACHE`). `notices.sh` copies them into a given `licenses/`
      folder so every platform ships the same set. The repository has no
      project `LICENSE` file, so there is none to ship.
    - Workflow shape: copy the toolchain setup from `ci.yml`'s `native` job
      (`jdx/mise-action@v2` pinned to `2026.9.0`, `ilammy/setup-nasm@v1`, the
      cargo cache), with a cache key of its own keyed by `matrix.os` (the two
      macOS runners differ in architecture, as `release.yml` notes) and a
      longer timeout than CI's 30 minutes for the cold release build. One
      `actions/upload-artifact@v4` per leg with `if-no-files-found: error`;
      the checksum job downloads all of them and runs `sha256sum`.
    - A `workflow_dispatch` workflow can be dispatched only once its file is
      on the default branch, so this PR's own run needs another trigger: push
      a temporary commit adding a `pull_request` trigger, inspect the run,
      and remove the trigger before the merge. From Step 2 on,
      `gh workflow run package.yml --ref <branch>` runs the branch's version.
      Record this in `docs/agents/github-actions-workflows.md`.
    - Pushing a workflow change needs the `workflow` scope
      (`docs/agents/github-actions-workflows.md`).
- [ ] Step 2: The Windows setup `.exe` and portable ZIP
  - Done when:
    - The `Package` run also builds on `windows-latest` and uploads
      `Handpic-<version>-windows-x64-setup.exe` (Inno Setup, per user) and
      `Handpic-<version>-windows-x64.zip` (the exe and `licenses/`), both
      covered by `SHA256SUMS`.
    - The workflow's Windows leg installs the setup with
      `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, asserts the exe landed under
      `%LOCALAPPDATA%` and the uninstall entry is under `HKCU`, runs the setup
      a second time over the install (the silent upgrade) and uninstalls
      silently, each with exit code 0.
    - `mise run native:package:windows` produces both files locally on
      Windows with Inno Setup installed.
    - `packaging-updater.md`'s Unverified item on the Inno Setup directives is
      rewritten with what was measured and what remains.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass; a
      `Package` run dispatched on the branch is green.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the `.iss`, the
      script and the task.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (the workflow, `version.sh`, `notices.sh`, the
      shellcheck wiring).
    - New `crates/native/packaging/windows/handpic.iss`, after gpui-kit's
      example. Directives to verify against the Inno Setup documentation
      before relying on them (written from memory at planning time):
      `PrivilegesRequired=lowest`, a `DefaultDirName` that resolves under
      `%LOCALAPPDATA%\Programs` (`{autopf}` under `lowest`, or
      `{localappdata}\Programs\Handpic`), `ArchitecturesAllowed=x64compatible`,
      `SetupIconFile` pointing at `crates/app/icons/icon.ico` (read, not
      moved), a Start menu shortcut, and the version passed in as
      `ISCC /DAppVersion=<version>`.
    - `AppId` must be stable for ever (gpui-kit: upgrades depend on it).
      Decide it here and record it in `native-app.md` as a rule; the
      candidates are the product identifier `dev.minodisk.handpic` or a fixed
      GUID.
    - Nothing for the updater yet: no close-applications handling and no
      relaunch logic (sub-milestone 3).
    - Measure whether `windows-latest` ships Inno Setup (`ISCC.exe`); install
      it in the workflow only if it does not. Locally it comes from
      `winget install JRSoftware.InnoSetup`, and its install folder may not
      be on `PATH`; `windows.sh` looks for `ISCC.exe` and fails with the
      install hint, as `tools/ci/local.sh`'s `ensure_nasm` does for NASM.
    - The ZIP must be made with a tool present on both the runner and a stock
      Windows 11 under Git Bash (Windows' own `tar.exe -a -c -f x.zip`, or
      PowerShell's `Compress-Archive` as in gpui-kit's guide); Git Bash has
      no `zip`.
    - The exe's icon and version resource already come from `build.rs`
      (`winresource`); never add a manifest (`native-app.md`'s rule).
    - GitHub's hosted Windows runners run elevated, so the workflow check
      proves the per-user location, the `HKCU` entry and the silent run, not
      the absence of a UAC prompt for a standard user. Leave that part
      Unverified in the record for a real machine.
- [ ] Step 3: The Linux `.tar.gz` with `install.sh`, the `.deb`, and the records
  - Done when:
    - The `Package` run also builds on `ubuntu-latest` and uploads
      `Handpic-<version>-linux-x64.tar.gz`, `install.sh` and
      `handpic_<version>_amd64.deb`, all covered by `SHA256SUMS`; one run now
      yields every artifact of sub-milestone 2.
    - The workflow's Linux leg runs `install.sh` with the tarball and its
      digest under a throwaway `HOME` and asserts the binary, the `.desktop`
      file (absolute `Exec`) and the icon are in place, asserts a wrong
      digest installs nothing, prints `dpkg-deb --info` / `--contents`, and
      installs and removes the `.deb` with `apt`.
    - `mise run native:package:linux` produces the three files locally on
      Linux.
    - `packaging-updater.md`'s Unverified items this plan settled are
      rewritten with their result (the version source, the `.desktop` file's
      `Exec`, the deb's `Depends` and the Vulkan requirement, the deb package
      name against the Tauri deb's); the others stay.
    - The gpui-port plan's Step 10 item 2 names this plan and its Progress
      has a "Step 10, sub-milestone 2" line (Step 10 stays open).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass; a
      `Package` run dispatched on the branch is green.
    - `docs/agents/native-app.md` and `CLAUDE.md` describe the Linux
      packaging files, the script and the task.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 and 2 are merged.
    - The Linux leg installs the same apt list as `ci.yml`'s `native` job
      (`native-app.md`'s rule: keep the two lists equal).
    - Tarball layout after gpui-kit's guide: one top-level `Handpic/` with
      `bin/<binary>`, `share/icons/hicolor/256x256/apps/dev.minodisk.handpic.png`
      (from `crates/app/icons/128x128@2x.png`, 256 px, as
      `native:install:linux` uses) and `licenses/`.
    - `crates/native/packaging/linux/install.sh` is adapted from gpui-kit's
      guide (its code examples are Apache-2.0; say so in the file's header):
      `install.sh ARCHIVE EXPECTED_SHA256`, the whole archive verified before
      anything changes, unsafe entries refused, a per-user install under
      `~/.local`, Python 3 needed. Add it to the shellcheck globs.
    - The installed layout must equal `native:install:linux`'s
      (`~/.local/bin/<binary>`,
      `~/.local/share/applications/dev.minodisk.handpic.desktop`, the icon),
      and the desktop entry's base name, `Icon` and `StartupWMClass` stay
      `IDENTIFIER` (Wayland and X11 match the window by it).
    - One source for the desktop entry: the checked-in
      `dev.minodisk.handpic.desktop`, whose `Exec` stays the bare binary name
      (`datadir.rs` tests it). The packaged copies get an absolute `Exec`,
      rewritten at packaging or install time: `/usr/lib/handpic/<binary>` in
      the deb, the user's `~/.local/bin` path from `install.sh`.
    - deb after the guide: `usr/lib/handpic/<binary>`,
      `usr/share/applications/`, the hicolor icon, the notices under
      `usr/share/doc/handpic/`, a `DEBIAN/control` generated from a checked-in
      template with the version, built by
      `dpkg-deb --root-owner-group --build`.
    - `Package: handpic`. `crates/app/tauri.conf.json`'s `productName` is
      `Handpic` on `main` and v1.10.0 shipped as Riffle; read what package
      name the Tauri deb used and record it.
    - `Depends` is derived from the built binary (`dpkg-shlibdeps`; check the
      result against `objdump -p`), not copied. Measure whether `libvulkan1`
      appears among the binary's `NEEDED` entries; if GPUI loads it at run
      time it has to be added by hand. Record the result and the runner's
      glibc baseline in `native-app.md`.
    - No package is launched in the workflow: the runner has no display and
      no Vulkan driver.
    - Do not change `native:install:linux`; it stays the from-source install.

## Trade-offs and risks

- **Three steps, not one PR.** Each platform has its own toolchain and each
  leg needs trial runs on a runner no agent has locally; one PR of about 600
  lines across three OSes could not be reverted per platform. The cost is
  that the artifact set is complete only after Step 3. A single PR is
  feasible if preferred; it would use the temporary trigger below for all
  three legs.
- **Verifying a dispatch-only workflow before it is on `main`.** Taken: a
  temporary `pull_request` trigger in Step 1's PR, removed before the merge.
  Alternative: keep a `pull_request` trigger with a `paths:` filter on the
  packaging files for good, so packaging changes are always exercised; this
  contradicts "manual `workflow_dispatch` only" and adds a long release build
  to those PRs. The caller decides.
- **The version is `0.0.0` until sub-milestone 4.** Taken: read
  `crates/native/Cargo.toml` and leave it alone, so the plumbing is in place
  and the binary and the file names agree. Alternatives: set it to the
  release version by hand now (it must then be kept in step by hand until
  release-please owns it), or read `.release-please-manifest.json` (`1.10.0`),
  which names artifacts with a version the binary does not carry.
- **`install.sh` beside the tarball, not inside it.** gpui-kit's script takes
  the archive and its SHA-256 as arguments and refuses archive entries it
  does not expect, so it is a separate artifact. The other reading of
  "a `.tar.gz` with `install.sh`" puts the script inside the archive, where it
  cannot verify the archive it came from. Sub-milestone 3's `self_update`
  check (the binary's path in the archive) may also constrain this.
- **The executable's name.** Kept as `riffle-native` behind one variable. The
  rename-to-handpic plan changes it; a rename after the first native release
  would change installed paths (`~/.local/bin`, `/usr/lib/handpic`, the Start
  menu target), so it should land before sub-milestone 4. On Windows the
  process shows as `riffle-native.exe` until then.
- **Asset names.** Human-readable names (`…-windows-x64.zip`,
  `…-linux-x64.tar.gz`) are taken. `self_update`'s asset matching is
  Unverified in the record and may want Rust target triples in the names;
  sub-milestone 3 renames them if so, at no cost while nothing ships.
- **License notices: the named list only.** The decision record names the
  GPUI crates, Lucide, the themes and the models. A full third-party
  inventory of every Rust dependency (for example with `cargo-about`) is not
  taken; the caller may want it before the first public release.
- **Linux build baseline.** Building on `ubuntu-latest` ties the binary and
  the deb's `Depends` to that image's glibc; older distributions cannot run
  it. An older runner image widens that at the cost of a second image to
  track. `release.yml` builds the Tauri app on `ubuntu-latest` today.
- **The deb does not upgrade a Riffle deb in place.** v1.10.0's package is
  named after Riffle. `Conflicts` / `Replaces` against it is left to
  sub-milestone 4's real-device check.
- **What the workflow cannot prove.** No UAC prompt for a standard Windows
  user (the runners are elevated), Gatekeeper and SmartScreen behavior, and
  that the app starts from each package. These stay with sub-milestone 4's
  real-device gate.
- **`release.yml` also has `workflow_dispatch`.** It still builds the Tauri
  bundles on a manual run and is not touched; the new workflow is a separate
  file with a different name.

## Progress
