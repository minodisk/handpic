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

# Native CI on three platforms, and the packaging and updater decision for the GPUI switch-over

## Purpose

Step 10 of the GPUI port (`../20261109-gpui-port/plan.md`) is packaging, the
updater, the three-platform CI and the switch-over. Most of it has to wait for
the other ports (Steps 8 and 9) to finish. Two parts do not:

- The `native` CI job runs on `windows-latest` only. `crates/native` has never
  been compiled, linted or tested on Linux or macOS, so nobody knows what
  breaks there. Finding out now is cheap; finding out at the switch-over is
  not.
- Step 10 still says "`cargo-packager` or `cargo-bundle`; decided then" and
  "the same signed `latest.json` contract or a documented new one". Both can
  be decided from sources today, and the answer shapes the rest of Step 10.

Once this is done, every native PR is checked on all three platforms, and
Step 10 in the parent plan is a list of concrete sub-milestones with the
packager and the updater contract fixed.

Out of scope: deleting `crates/app` and `crates/app/ui`, merging the two
workspaces, rewriting `README.md` / `docs/**` wholesale, and the ports other
sessions own (pass 3, performance overlay, prefetch ring, settings modal, MCP,
folder tree, trash, folder dialogs). No packaging or updater code is written
here.

## Steps

- [x] Step 1: Run the `native` CI job on Windows, Linux and macOS, and record the packaging and updater decision
  - Done when:
    - `.github/workflows/ci.yml`'s `native` job is a matrix over
      `windows-latest`, `ubuntu-latest` and `macos-latest` with
      `fail-fast: false`, and its clippy (`-D warnings`) and `cargo test` pass
      on all three in the PR's checks. The `ci` aggregate job still requires
      `needs.native.result == success` (a matrix job's result covers every
      leg). The job's comment no longer says "Windows only until the
      switch-over".
    - Whatever made Linux or macOS fail is fixed in `crates/native` (or, if
      the cause is there, in `crates/backend` / `crates/core`), not skipped.
      `crates/app` and `crates/app/ui` have no diff.
    - `docs/agents/native-app.md` has an entry for each thing the Linux and
      macOS legs hit (tagged `Hit`), and its "Linux needs system libraries"
      entry lists the packages the job actually installs.
    - A new `packaging-updater.md` in the parent plan's folder
      (`../20261109-gpui-port/`) records the packager recommendation, the
      updater recommendation, the `latest.json` contract as the shipped Tauri
      installs consume it, and what is still unverified, each claim marked as
      read from source or unverified.
    - The parent plan's Step 10 is rewritten into concrete sub-milestones (see
      the approach below), names `packaging-updater.md`, and no longer says
      "decided then". The parent plan's Progress gets one line for this slice;
      Step 10 stays unticked.
    - `mise run lint` passes (actionlint for the workflow, lychee for the new
      links).
  - Implementation approach (as far as it is known; omit if unknown):
    - **The workflow change.** Files: `.github/workflows/ci.yml` only.
      - Add `strategy: { fail-fast: false, matrix: { os: [...] } }` and
        `runs-on: ${{ matrix.os }}`, like the `test` job above it.
      - Linux needs an apt step before the toolchain, guarded by
        `runner.os == 'Linux'`. Start from the parent plan's list:
        `libfontconfig-dev`, `libwayland-dev`, `libx11-xcb-dev`,
        `libxkbcommon-x11-dev`, `libvulkan1`. Add a package only when a build
        error names it. Zed's `script/linux` at the pinned rev also lists
        `libasound2-dev`, `libssl-dev`, `libzstd-dev`, `libsqlite3-dev`,
        `clang` and `cmake`, but those serve Zed, not `gpui`.
      - `ilammy/setup-nasm@v1` supports all three runners. `macos-latest` is
        arm64, where `crates/native/build.rs` returns before the NASM check,
        so the action is harmless there; guard it with the architecture only
        if it fails.
      - The run step's `cygpath` / `$USERPROFILE/nasm` `PATH` line is
        Windows-only; guard it with `[ "$RUNNER_OS" = Windows ]`. Keep the
        direct `mise exec rust@1.98.1 -- cargo clippy` / `cargo test` commands
        on all three legs rather than switching Linux and macOS to
        `mise run native:check`: one command path for the job, and the open
        learnings item about the task shell losing `PATH` stays a separate
        question.
      - The cache key already contains `runner.os`, so the three legs do not
        share a `target`. The cold build clones the whole Zed repository and
        compiles GPUI; raise `timeout-minutes` only if a cold leg exceeds 30,
        and record the cold durations in `learnings.md`.
    - **Iterating.** Linux and macOS can only be verified on GitHub. Open the
      PR as a draft right after the workflow change, read each failure with
      `gh run view --log-failed`, fix, push, repeat. `ci.yml` cancels
      superseded runs on a PR, so a push does not queue behind the last one.
      Pushing a workflow change needs the `workflow` scope
      (`docs/agents/github-actions-workflows.md`). Record each failure and fix
      in `learnings.md` as it happens.
    - **Failures to expect** (none confirmed; nothing here has run off
      Windows):
      - macOS: `gpui_apple`'s build script runs `xcrun metal` and
        `xcrun metallib` unless the `runtime_shaders` feature is on. A runner
        image whose Xcode lacks the Metal toolchain fails there. Default fix:
        download the toolchain in the job (`xcodebuild -downloadComponent
        MetalToolchain`), a CI-only change. Enabling `gpui_platform`'s
        `runtime_shaders` feature is the alternative. See "Trade-offs and
        risks".
      - Linux: link or `pkg-config` errors naming a missing library; add that
        package to the apt step.
      - Clippy on non-Windows: code only reached under `cfg(windows)` becomes
        dead code and fails `-D warnings`. `crates/native/src` has one
        `cfg` attribute today (`keys.rs`) and three `cfg!(target_os =
        "macos")` branches in `logger.rs`.
      - Tests: `tree.rs`'s tests feed `C:\` and UNC paths to functions that
        take the case rule as a flag; they fail on Unix if anything underneath
        goes through `std::path`. `datadir.rs`, `logger.rs` and `settings.rs`
        tests depend on `dirs`' per-platform folders. No test opens a GPUI
        window (`gpui::test` / `TestAppContext` are not used), so a headless
        runner is not a problem by itself.
      - A fix that touches `crates/backend` or `crates/core` must also pass
        `mise run ci` (the backend is compiled twice).
    - **`notify` 8.2.2 does not matter for this step.** `cargo test` in
      `crates/native` runs only `riffle-native`'s tests; the backend's watcher
      tests run in the root workspace on 8.2.0, on all three platforms
      already. The matrix does not change `crates/native/Cargo.lock`. The
      `todo.md` item (whether `configure` still queues behind `Unwatch` on
      8.2.2) is about the Windows rename path and stays open; it becomes
      forced at the workspace merge, where one lock picks one `notify`
      version. Write that into the parent plan's workspace-merge sub-milestone
      and leave `todo.md` as it is.
    - **`packaging-updater.md`: what was read from source.** Write these as
      findings, with the source named.
      - The contract today (`crates/app/tauri.conf.json`, `release.yml`, the
        `latest.json` of v1.10.0, `tauri-plugin-updater` 2.12.0's
        `updater.rs`): the endpoint is
        `https://github.com/minodisk/handpic/releases/latest/download/latest.json`.
        `tauri-action` writes that file and attaches it to the draft release.
        Its shape is `version`, `notes`, `pub_date` and `platforms`, a map of
        key to `{ signature, url }`. The keys in v1.10.0 are
        `windows-x86_64`, `windows-x86_64-nsis`, `windows-x86_64-msi`,
        `darwin-aarch64`, `darwin-aarch64-app`, `darwin-x86_64`,
        `darwin-x86_64-app`, `linux-x86_64`, `linux-x86_64-appimage`,
        `linux-x86_64-deb`, `linux-x86_64-rpm`. `signature` is the base64 of a
        minisign `.sig` file's text, made over the artifact's bytes with the
        key in `TAURI_SIGNING_PRIVATE_KEY`; the public key is the base64 of a
        minisign public key file in `plugins.updater.pubkey`. The trusted
        comment carries `timestamp`, `file` and `version`; the client checks
        `version` against the manifest only when a signature carries one
        (`require_signed_version` is off).
      - What a shipped Tauri install does with it, which is what the first
        native release must satisfy. It looks up `{os}-{arch}-{installer}`
        (the installer type is baked into the binary by the bundler), then
        `{os}-{arch}`; updates when `version` is greater; downloads `url`;
        verifies the signature; then:
        - Windows, NSIS install: writes the bytes to a temp `.exe` (a raw exe
          or a zip holding one) and launches it with `/P /UPDATE`, plus
          `/R /ARGS ...` on `Restart Now`.
        - Windows, MSI install: `msiexec /i <msi> /passive /promptrestart`. A
          major upgrade needs the same upgrade code
          (`15df1a23-08b6-56be-b9df-b38d92f7dd75` in `tauri.conf.json`).
        - macOS: untars the `.app.tar.gz`, drops the first path component and
          swaps the result into the existing bundle's path.
        - Linux, AppImage: writes the bytes (a raw AppImage, or a `.tar.gz`
          holding one) over the running AppImage's path.
        - Linux, deb / rpm: checks the bytes are a deb / rpm, then
          `pkexec dpkg -i` / `rpm -U`. An rpm install handed anything else
          fails with `InvalidUpdaterFormat`.
      - Nothing in that contract is private to Tauri: plain JSON, minisign,
        and per-platform artifact shapes. A native updater can read the same
        file with `reqwest` (already in `crates/backend`, rustls) and
        `minisign-verify` (already in the root lock, the crate the plugin
        uses), plus `tar` and `flate2` for macOS.
      - `cargo-packager` (crates.io 0.11.8 from 2025-11; the repository had
        commits on 2026-11-12): NSIS, WiX MSI, `.app`, DMG, deb, AppImage,
        pacman. No rpm. Its NSIS template descends from Tauri's: it reads `/P`
        and `/R`, and uses the same `Uninstall\${PRODUCTNAME}` and
        `Software\${MANUFACTURER}\${PRODUCTNAME}` registry keys. It does not
        read `/UPDATE`. It ships a minisign signer.
      - `cargo-packager-updater` (0.2.3 from 2025-07): the same manifest idea,
        but its macOS keys are `macos-<arch>`, not `darwin-<arch>`, and each
        platform entry needs a `format` field. The Tauri plugin's entry struct
        does not reject unknown fields, so one file could carry both sets.
      - `cargo-bundle` (0.12.0 from 2026-09): `osx`, `deb`, `rpm`, `appimage`,
        and `msi` / `wxsmsi`, with Windows support called experimental in its
        readme. No NSIS, no updater signatures.
    - **`packaging-updater.md`: the recommendation to record.**
      - Packager: `cargo-packager`. The deciding fact is Windows: shipped
        installs launch the downloaded file as an NSIS installer with `/P`,
        and `latest.json` prefers NSIS (`updaterJsonPreferNsis`).
        `cargo-bundle` cannot produce one. `cargo-packager` also covers every
        other format shipped today except rpm.
      - Updater: self-written, in `crates/backend`, reading the unchanged
        `latest.json` keys, with the same key pair. The same file then serves
        shipped Tauri installs and native installs with one set of keys, no
        second HTTP stack comes in, and the Windows install stays deferred to
        quit after the sidecar flush as `crates/app/src/update.rs` does it.
        `cargo-packager-updater` is the alternative; say why it was not taken.
      - So: keep the contract, do not write a new one.
    - **`packaging-updater.md`: what is unverified**, to be checked in the
      sub-milestone that needs it. Do not state these as facts.
      - Whether `cargo-packager`'s NSIS installer, launched with `/P /UPDATE`
        by a shipped install, upgrades in place: `/UPDATE` is ignored, so it
        takes its own reinstall path, which runs the old (Tauri) uninstaller
        with `/P`. Needs a real Windows install. Also the main binary name
        (`riffle-app.exe` today, `riffle-native` in the port) and the
        shortcuts that point at it.
      - Whether `crates/app/nsis/hooks.nsh` (the legacy `Riffle` uninstall)
        can be carried into a `cargo-packager` NSIS template.
      - Whether `cargo-packager`'s WiX config can pin the upgrade code.
      - Whether `cargo-packager`'s signer accepts the existing
        `TAURI_SIGNING_PRIVATE_KEY` and produces a signature the public key in
        `tauri.conf.json` verifies. Whatever signs, the release job must
        verify each signature with that public key before uploading.
      - rpm: see "Trade-offs and risks".
      - Whether the last shipped release is still named `Riffle`
        (v1.10.0's assets are) when the switch-over ships, which decides how
        many legacy names the installers must handle.
    - **The parent plan's Step 10.** Replace its one "Done when" paragraph
      with sub-milestones in this order, each re-planned as its own plan when
      its turn comes, as the parent plan already says of milestones:
      1. Native CI on three platforms and the packaging / updater decision
         (this plan).
      2. Package the native app with `cargo-packager` on a manual
         `workflow_dispatch` only, uploading workflow artifacts: Windows
         (NSIS, MSI), macOS arm64 and x86_64 (`.app`, DMG), Linux (deb,
         AppImage, rpm per the decision). No release is touched.
      3. The self-update in `crates/backend` with the native app's startup
         check, menu item and quit-time install, reading `latest.json`.
      4. `release.yml` builds, signs and attaches the native artifacts and a
         `latest.json` with today's keys, verified against the public key;
         a real-device update from the last Tauri release on each platform
         before the first native release is published.
      5. Merge `crates/native` into the root workspace: one `Cargo.lock`, the
         root toolchain bumped, the two patches moved to the root, one
         `notify` version (resolving the `todo.md` item on 8.2.2), and
         `crates/app` and `crates/app/ui` removed with their Vite+, pnpm and
         Tauri tooling.
      6. `README.md` / `README.ja.md`, `CONTRIBUTING.md`, `CLAUDE.md`,
         `docs/humans/**` and `docs/agents/**` rewritten for the native app,
         `tauri-app.md` archived.
      Keep the existing content of Step 10 inside these; add nothing new
      beyond the order and the two decisions.
    - Files: `.github/workflows/ci.yml`, `docs/agents/native-app.md`,
      `docs/plans/20261109-gpui-port/plan.md`,
      `docs/plans/20261109-gpui-port/packaging-updater.md` (new), and whatever
      in `crates/native/src` the two new legs require. `CONTRIBUTING.md`'s
      "Native app (GPUI)" section gains the Linux package list only if the
      job needed one beyond NASM. `CLAUDE.md` needs no change unless a source
      file is added.

## Trade-offs and risks

- **One PR or two.** This plan puts the workflow change and the documents in
  one PR, as asked. The documents do not depend on CI, and the CI leg may
  take many pushes. If the Linux or macOS fixes grow large, split: the
  documents and the parent plan edit first, the matrix second.
- **Where the decision record lives.** Taken here: `packaging-updater.md` in
  the parent plan's folder, where the plan guide says investigation results
  go. The plan folder is archived when the port ends, so the contract facts
  then move into the guide the native updater's code gets.
- **macOS shaders: download the Metal toolchain, or `runtime_shaders`.**
  Only relevant if the macOS leg fails at `xcrun metal`. Downloading the
  toolchain (the default) keeps the build as Zed ships it but adds a download
  to every cold run and depends on the runner's Xcode. `runtime_shaders`
  compiles shaders at startup on every macOS build unless made CI-only.
- **A failure that cannot be fixed inside `crates/native`.** If a leg fails
  in GPUI itself at the pinned rev, the fix is a rev bump, which the port
  treats as its own deliberate chore PR. In that case stop and report rather
  than marking the leg `continue-on-error`: the acceptance criterion is a
  passing job on all three.
- **rpm.** `cargo-packager` has no rpm target. A shipped rpm install looks up
  `linux-x86_64-rpm` and rejects anything that is not an rpm. Either keep an
  rpm in the release with a second tool (for example `cargo-generate-rpm`),
  or drop rpm and tell those users to reinstall by hand. Not decided here;
  record both in `packaging-updater.md` and decide in the packaging
  sub-milestone.
- **Self-written updater vs `cargo-packager-updater`.** Self-written means
  owning the per-platform install code (about what `update.rs` plus the
  plugin's install functions do today) and its security review. The crate
  means less code but a second manifest dialect (`macos-*` keys, `format`),
  its own HTTP stack, and a release from 2025-07.
- **Actions cache size.** Three native `target` directories join the three
  root ones and the four release ones under the repository's cache limit.
  Evictions would show up as cold native builds; record the cache sizes in
  `learnings.md` and act only if it happens.
- **Research limits.** The `cargo-packager` and `cargo-bundle` facts come
  from their public sources and crates.io metadata read on 2026-11-14, not
  from running either tool. Nothing was installed or built.

## Progress
