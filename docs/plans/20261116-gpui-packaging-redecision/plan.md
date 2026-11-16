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

# Re-decide the GPUI port's packaging and updater along gpui-kit's guides

## Purpose

Step 10 of [the gpui-port plan](../20261109-gpui-port/plan.md) packages the
native app, gives it an updater and switches the releases over.
[`packaging-updater.md`](../20261109-gpui-port/packaging-updater.md) decided
on 2026-11-14 to package with `cargo-packager` and to write an updater that
keeps the Tauri `latest.json` / minisign contract, so that installs in the
field would update into the native app.

On 2026-11-16 the user decided otherwise: auto-update for existing Tauri
installs stops at the last Tauri release, and Tauri users reinstall by hand.
The compatibility contract with shipped Tauri installs (`latest.json`,
minisign signatures, NSIS `/P /UPDATE`, the MSI upgrade code, an rpm for rpm
installs) is dropped, and the native app's packaging and updating follow
gpui-kit's standard guides (<https://gpui-kit.com/docs/packaging/> and
<https://gpui-kit.com/docs/auto-update/>).

This plan rewrites the decision record and Step 10's sub-milestones to match.
It is documentation only: no packaging or updater code, no workflow change.
Once it is merged, each remaining sub-milestone of Step 10 can be planned
against a record that says what is decided, what is unverified and what is
still open.

## Steps

- [x] Step 1: Rewrite `packaging-updater.md` and Step 10 of the gpui-port plan for the gpui-kit-aligned decision
  - Done when:
    - `docs/plans/20261109-gpui-port/packaging-updater.md` no longer
      recommends `cargo-packager` or keeping the `latest.json` contract. It
      states the user's 2026-11-16 decision, the decision per platform
      (artifacts, tools, who owns an update), what is unverified and the
      open questions listed below. Every claim keeps the **Source** /
      **Unverified** tagging.
    - Step 10 of `docs/plans/20261109-gpui-port/plan.md` has sub-milestones
      consistent with the record: no `cargo-packager`, no `latest.json`
      produced or read by the native app, no "real-device update from the
      last Tauri release". Its Progress section has a 2026-11-16 entry for
      the re-decision. Step 10 stays unchecked.
    - No file other than those two (and this plan's own files) changes.
    - `mise run ci:local` passes locally (for a Markdown-only change it runs
      lychee) and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **Scope.** Only the two files above. Leave these alone; they are the
      later sub-milestones' work and the new sub-milestone 4 names them:
      `crates/**`, `.github/workflows/**`, `mise.toml`, `README*.md`,
      `docs/humans/**`, `docs/agents/**`, `CONTRIBUTING.md`, `CLAUDE.md`,
      the `release` skill and `release-please-config.json`. The Tauri app is
      frozen (auto memory: GPUI only).
    - **Form of `packaging-updater.md`.** Rewrite it in place and keep the
      title. The lead says: it is the decision record for Step 10, first
      written on 2026-11-14 by the gpui-ci-packaging plan (now at
      `../_archived/20261114-gpui-ci-packaging/plan.md`; write that path in
      backticks as today, and fix the stale non-archived path), and
      re-decided on 2026-11-16 by this plan. The earlier text
      (`cargo-packager`, the full `latest.json` contract) stays in git
      history only; one sentence says so. Tag each claim **Source** (read
      from the named source on 2026-11-16) or **Unverified** (checked in the
      sub-milestone that needs it). State that nothing comes from running a
      packaging tool, `self_update` or an update. English, wrapped near 80
      columns like the current file. `vite.config.ts` ignores `**/*.md` for
      `vp fmt`, so only lychee checks the file. Write the two gpui-kit URLs
      as plain URLs (lychee runs `--offline`).
    - **Section: the user's decision (2026-11-16).** Auto-update for Tauri
      installs stops at the last Tauri release; Tauri users reinstall by
      hand. Dropped with it: `latest.json`, the minisign key pair
      (`TAURI_SIGNING_PRIVATE_KEY`, `plugins.updater.pubkey`), NSIS
      `/P /UPDATE`, the MSI upgrade code, an rpm for shipped rpm installs,
      and the "real-device update from the last Tauri release" gate.
    - **Section: how Tauri installs stop updating.** Facts read at planning
      time; write them as Source with these sources:
      - The endpoint is
        `https://github.com/minodisk/handpic/releases/latest/download/latest.json`
        (`crates/app/tauri.conf.json`).
      - In `tauri-plugin-updater` 2.12.0's `updater.rs`, a response without
        a success status leaves no release and `check` returns
        `Error::ReleaseNotFound`.
      - `crates/app/src/update.rs` logs `update failed: …` on the startup
        check and shows an error dialog only on the menu check.
      - So the first published release without a `latest.json` asset ends
        auto-update for every Tauri install, with no code change.
      - Rule to record: no native release attaches an asset named
        `latest.json` in the Tauri shape with native artifacts, since
        shipped installs would try to install it.
      - `gh release list` on 2026-11-16 shows v1.10.0 as the latest
        published release. The earlier record says its assets are named
        `Riffle`. `main` already carries the Handpic rename
        (`tauri.conf.json` `productName`), unreleased.
    - **Section: what gpui-kit's guides say.** Source: the two pages, read
      on 2026-11-16 through a summarizing fetch. Say that the exact scripts
      (`install.sh`, the `.iss`, the deb `control`) were not read line by
      line and are to be read in the packaging sub-milestone.
      - Packaging:
        - No single packager. `cargo build --release --locked` per target
          and architecture, on that platform.
        - macOS: a `.app` (`Info.plist`, `.icns`); Developer ID with
          hardened runtime (`codesign --options runtime --timestamp`),
          nested code first, not `--deep`; DMG by `hdiutil create … -format
          UDZO`, signed; `xcrun notarytool submit --wait` with the result
          checked as Accepted; `xcrun stapler staple` / `validate`. A ZIP or
          tarball of the `.app` is the portable form.
        - Windows: a portable ZIP, or an Inno Setup `.exe` installer
          (`ISCC.exe`, a stable `AppId`). The exe is signed before
          packaging, then the setup; the uninstaller needs Inno's
          signed-uninstaller workflow.
        - Linux: a `.tar.gz` with an `install.sh` that verifies a SHA-256
          before a per-user install under `~/.local`, and a `.deb` by
          `dpkg-deb --root-owner-group --build` with `Depends` derived from
          the built binary. rpm needs a separate recipe. AppImage and MSI
          are not mentioned.
        - SHA-256 checksums are published; install, upgrade N to N+1 and
          uninstall are tested on clean machines.
      - Auto update:
        - The install's owner updates it.
        - `self_update` 1.3 is an optional dependency: GitHub backend,
          `checksum_from_asset("SHA256SUMS")`, `is_update_available()` /
          `update()`, blocking, so it runs on a background worker. It is
          only for a writable single-binary portable install.
        - macOS `.app` from a DMG: a bundle-aware updater, or a new signed
          and notarized app replacing the whole `.app`, never the executable
          alone.
        - Inno Setup: the installer owns files, shortcuts and the uninstall
          record.
        - deb / rpm: the package manager.
        - A multi-file portable archive is updated as a whole.
        - Restart to finish; report the new version only after the relaunch.
        - Stage outside the live install.
        - Windows file locks and elevation are handled by a helper or the
          installer.
        - Do not silently switch an install from one owner to another.
        - `SHA256SUMS` proves integrity, not who published; use a signing
          key or signed metadata where that matters.
        - Pin the selected version between check and install.
      - Source (docs.rs feature list, 2026-11-16): `self_update` is at
        1.3.3 and has a `signatures` feature depending on `zipsign-api`
        0.2.2. Unverified: how it signs and verifies, and where the key
        lives.
    - **Section: the decision.** Write it as the gpui-kit-aligned default
      per platform. Mark each line that depends on an open question with a
      reference to it.
      - Tooling: platform-native tools driven from mise tasks and the
        release workflow; no `cargo-packager`, `cargo-bundle` or Tauri
        bundler. It grows from what exists (Source: `mise.toml`,
        `crates/native/packaging/`, `crates/native/build.rs`,
        `docs/agents/native-app.md`):
        - `native:bundle:macos` lays out an unsigned `Handpic.app` with no
          DMG.
        - `native:install:linux` installs into the same `~/.local` layout
          gpui-kit's `install.sh` uses.
        - `build.rs` already embeds the Windows icon.
        - The license notices (`gpui-component`, the `gpui-pre-*` crates,
          Lucide; the models' licenses in `crates/core/models/`) are due at
          packaging.
      - macOS, arm64 and x86_64 separately: `Handpic.app` in a DMG. An
        update replaces the whole `.app`.
      - Windows x64: an Inno Setup setup `.exe` and a portable ZIP. The
        installer owns an installed copy's update; `self_update` may update
        the portable copy. No MSI and no NSIS.
      - Linux x64: a `.tar.gz` with `install.sh`, and a `.deb`. `apt` /
        `dpkg` owns a deb install's update; `self_update` may update the
        tarball install.
      - Every release carries `SHA256SUMS` and no `latest.json`.
      - Update flow in the app: a launch check and a manual check; the
        install's owner does the install; restart to finish; the install or
        relaunch only after the sidecar writer is drained on quit, as
        `crates/app/src/update.rs` does today.
      - Not taken, with reasons: `cargo-packager` plus the self-written
        `latest.json` updater (its deciding fact, Windows NSIS `/P`
        compatibility with shipped installs, is gone with the user's
        decision); `cargo-packager-updater`; `cargo-bundle`.
    - **Section: unverified.** At least:
      - How the running app knows its install type (portable, Inno Setup,
        DMG, deb), which "do not switch owners" requires.
      - `self_update`'s asset-name and target matching, the binary's path in
        the archive, and whether the portable archives count as
        single-binary once the license notices ship beside the binary.
      - Whether replacing the running exe works on Windows for the portable
        copy.
      - The executable's name in a release (`riffle-native` today in
        `Info.plist`, the `.desktop` file and `datadir.rs`'s tests).
      - The version source: `crates/native/Cargo.toml` and `Info.plist` are
        `0.0.0`, and `release-please-config.json` bumps neither.
      - The `.desktop` `Exec` (relative today; gpui-kit's `install.sh`
        writes an absolute path).
      - The deb's `Depends` and the Vulkan driver requirement at run time.
      - What a hand reinstall over a Tauri install does on each platform:
        the Inno Setup install next to the NSIS / MSI one and its uninstall
        entry and shortcuts; the deb's `Package` name against the Tauri
        deb's, so `apt` upgrades in place; a leftover `Riffle.app`.
      - That the native app picks up the Tauri install's data. `settings.rs`
        seeds its store from the Tauri `settings.json`, and the index is the
        native app's own (Source: `CLAUDE.md`); unverified on a real
        install.
    - **Section: open questions.** Each with its options and consequences
      and no choice made unless the caller supplies one (see Trade-offs and
      risks):
      1. Who performs the update of installer-owned installs.
      2. Publisher authenticity beyond `SHA256SUMS`.
      3. OS code signing.
      4. rpm and AppImage.
      5. What the last Tauri release tells users, and which release that is.
      6. Whether the portable Windows ZIP ships at all.
    - **Step 10 of the gpui-port plan.** Rewrite the "Done when" lead: the
      packaging and the updater follow gpui-kit's guides as decided in
      `packaging-updater.md` (re-decided 2026-11-16), with what is
      unverified and open; the six sub-milestones in order, each re-planned
      as its own plan. The sub-milestones:
      1. Unchanged in substance and already done: the three-platform
         `native` CI job and the first decision. Add that the decision was
         superseded on 2026-11-16, and fix the plan path to
         `../_archived/20261114-gpui-ci-packaging/plan.md`.
      2. Package the native app with platform-native tools on a manual
         `workflow_dispatch` only, uploading workflow artifacts and touching
         no release: `cargo build --release --locked`; Windows Inno Setup
         setup `.exe` and portable ZIP; macOS arm64 and x86_64 `Handpic.app`
         in a DMG; Linux `.tar.gz` with `install.sh` and `.deb`;
         `SHA256SUMS`; the license notices; the version wired from the
         release version; signing as open question 3 is settled; rpm and
         AppImage as question 4 is settled.
      3. The native app's update flow: the launch check, the menu item,
         `self_update` for the portable installs, the owner's path for the
         installer, DMG and deb installs as question 1 is settled,
         authenticity as question 2 is settled, restart to finish after the
         sidecar drain. It replaces `tauri-plugin-updater` and reads no
         `latest.json`.
      4. `release.yml` builds and attaches the native artifacts and
         `SHA256SUMS` and no `latest.json`. With it:
         `.claude/skills/release/scripts/verify-release.sh` (which requires
         `latest.json`, `.msi`, `.rpm` and `.AppImage` today), the `release`
         skill's text, `CONTRIBUTING.md`'s stats note and
         `release-please-config.json`'s version files. Before the first
         native release is published: a real-device install, an update from
         N to N+1 and an uninstall per format, and the notice to Tauri users
         as question 5 is settled.
      5. Unchanged (the workspace merge and the removal of `crates/app`).
      6. Unchanged (the docs rewrite), plus the reinstall instructions for
         Tauri users in `README.md` / `README.ja.md` and `docs/humans/**`.
    - Add to the gpui-port plan's Progress: `(2026-11-16) Step 10: the
      packaging / updater decision redone along gpui-kit's guides after the
      user dropped the update path from Tauri installs, through` this plan's
      path in backticks `(Step 10 stays open)`.
    - Check that the "Trade-offs and risks" bullets of the gpui-port plan
      that mention Step 10 (the workspace merge, NASM, "Two apps for a long
      time") still read true. None refers to `cargo-packager` or
      `latest.json`, so no edit is expected.

## Trade-offs and risks

The step can be done without settling any of these: the record lists them as
open. Where the caller (or the user) does decide one before the step runs,
move it from "open questions" to "the decision" in the record and adjust the
matching sub-milestone.

- **Who updates installer-owned installs (Inno Setup, DMG, deb).**
  - (a) Notify only: the app says a newer version exists and opens the
    release page; the user runs the installer, drags the new `.app` or
    installs the deb. Least code, no elevation or file-lock handling, and no
    authenticity question for these installs beyond the OS signature. It is
    a regression from today's silent updates on Windows and macOS.
  - (b) The app drives the owner: it downloads the setup `.exe` and runs it
    at quit; stages and swaps the whole `.app`; a deb stays with `apt` or a
    hand install, as today. Close to today's behavior, but it is the
    per-platform install code the earlier record priced, and it needs its
    own verification of what it downloads.
  - gpui-kit allows both and forbids only replacing the binary inside such
    an install.
- **Publisher authenticity.** Today's updates are minisign-verified;
  `SHA256SUMS` from the same GitHub release only proves integrity.
  - OS signatures (Developer ID plus notarization, Authenticode) checked on
    the staged artifact; nothing on Linux.
  - `self_update`'s `signatures` feature (`zipsign-api`): a verifying key
    compiled into the app and a new signing secret in CI; covers the
    portable archives only; unverified in detail.
  - Accept checksum-only over GitHub HTTPS, recorded as a step down.
  - With "notify only" above, the question shrinks to the portable installs.
- **OS code signing.** gpui-kit's standard is a Developer ID signed and
  notarized DMG and a signed exe, setup and uninstaller. Today nothing is
  OS-signed (`README.md` tells users how to pass Gatekeeper and
  SmartScreen). Following the guide fully needs paid certificates and CI
  secrets, which is the user's call. Staying unsigned (ad hoc on macOS)
  deviates from the guide and removes OS signatures as an authenticity
  answer. The record must not claim signing is decided either way.
- **rpm and AppImage.**
  - rpm: drop it (rpm-distribution users take the `.tar.gz` with
    `install.sh`), or add a separate recipe (`cargo-generate-rpm`,
    `rpmbuild`).
  - AppImage: it is the Linux download `README.md` leads with and today's
    only self-updating Linux format. Either the `.tar.gz` + `install.sh`
    install with `self_update` replaces it, or it is kept with a tool
    outside gpui-kit's guide.
  - MSI: the plan writes it as dropped, since the guide offers Inno Setup or
    a ZIP. Say so if it should be an open question instead.
- **The last Tauri release and its message.**
  - (a) v1.10.0, still named Riffle, stays the last one. No install is told
    anything in-app; the notice lives in the first native release's notes,
    the README and the usage guide; a menu check on a Tauri install shows an
    update error from then on.
  - (b) One final Tauri release from `main` (it would also ship the Handpic
    rename) whose release notes announce the reinstall. Installs update to
    it silently, so its notes are seen only on GitHub.
  - (c) As (b), with an in-app one-time notice. It needs a change in the
    frozen `crates/app`, an exception only the user can grant.
  - Independent of these: attach to later native releases a frozen copy of
    the last Tauri release's `latest.json`, pointing at that release's own
    assets. Installs older than it could still reach it, and a Tauri check
    would report "up to date" instead of an error. The cost is one legacy
    asset on every release and a rule that it never names a native artifact.
- **Whether to ship the Windows portable ZIP.** The guide says to choose
  only the formats users need. Without it, `self_update` has no Windows
  target and Windows updates rest wholly on the first question.
- **Risk: guide facts came through a summarizing fetch.** The exact
  `install.sh`, `.iss` and `control` contents were not read. The record tags
  that, and the packaging sub-milestone reads the pages itself.
- **Risk: files outside this step keep describing the Tauri contract**
  (`docs/humans/usage.md`'s Updating section, the `release` skill,
  `verify-release.sh`, `CONTRIBUTING.md`). That is correct while the Tauri
  app is what ships. Sub-milestones 4 and 6 own them, and this step must not
  edit them.

## Progress
