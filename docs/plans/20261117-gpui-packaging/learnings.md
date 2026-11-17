# Learnings

## Step 1

- The GPUI crates carry two copyright holders, not one: `gpui-component`,
  `gpui-component-macros`, `gpui-base`, `gpui-kit` and `gpui-kit-assets`
  0.9.1 are Longbridge's (`Copyright 2024 - 2026 Longbridge`), while the
  `gpui-pre-*` 0.3.12 crates' `LICENSE-APACHE` reads `Copyright 2022 - 2025 Zed
  Industries, Inc.`. `crates/native/packaging/NOTICE-gpui` names these lines
  and then carries the Apache-2.0 text of `gpui-component`'s
  `LICENSE-APACHE` (from "Apache License, Version 2.0, January 2004" on).
  `gpui-kit` itself ships no `LICENSE-APACHE` file in its crate, only the
  `license` field. `gpui-pre-reqwest` 0.12.15 is a fork of reqwest, `MIT OR Apache-2.0`, with
  `Copyright 2016 Sean McArthur`; it has its own entry in `NOTICE-gpui`, and
  the Apache text covers it.
- `crates/core/models/LICENSE` is YuNet's MIT license; `notices.sh` ships it
  as `LICENSE-yunet` so the names in `licenses/` do not collide with the
  themes' `LICENSE` (shipped as `LICENSE-themes`).
- The layout stays in `native:bundle:macos`, as the plan says. So the
  workflow runs `cargo build --release --locked` through `mise exec`, then
  `mise run native:bundle:macos` (whose own release `cargo build` finds the
  binary up to date), then `tools/package/macos.sh`, which only signs and
  makes the DMG. `macos.sh` stages the app with `ditto` rather than
  `cp -R`, so the ad hoc signature's extended attributes survive the copy.
- The DMG's `<arch>` comes from `uname -m` (`arm64` / `x86_64`), which is
  what the plan's file names use; the workflow's check step asserts the
  executable's `lipo -archs` equals the matrix's `arch`.
- A change to `mise.toml` selects every check in `mise run ci:local`, so
  this step's local run includes the root crates' and the native crate's
  clippy and tests.
- Nothing macOS-specific could run here (Windows host): `actionlint`,
  `shellcheck` and `local.test.sh` pass, `version.sh` prints `0.0.0` and
  `notices.sh` copies the six files, but `plutil`, `codesign`, `hdiutil` and
  the bundle and package tasks are only exercised by a GitHub run.
- The `Package` run on the branch: run 43986612196, green on the first
  attempt (no fix needed). It was triggered by a temporary
  `push: branches: [gpui-packaging-step-1]` on `package.yml` (a push trigger
  rather than the plan's `pull_request`, because the PR did not exist yet),
  removed again in the next commit. The arm64 leg took 11m44s, the
  `macos-15-intel` leg 27m8s (both cold caches), `checksums` 15s. Both legs'
  "Check the DMG" step passed: `hdiutil verify` VALID, `codesign --verify
  --strict` "valid on disk" and "satisfies its Designated Requirement", the
  six notices listed under `Contents/Resources/licenses`, plus the step's
  `lipo`, `plutil` version and `/Applications` symlink asserts. Downloaded
  with `gh run download`: `Handpic-0.0.0-macos-arm64.dmg` (27,338,311
  bytes), `Handpic-0.0.0-macos-x86_64.dmg` (30,919,229 bytes) and
  `SHA256SUMS` (193 bytes), which lists both DMGs, and `sha256sum -c` passed
  for both. The DMG contents were not inspected on Windows (no mount); the
  runner's check step stands for that. The run annotates that
  `actions/cache@v4`, `checkout@v4`, `upload-artifact@v4`,
  `ilammy/setup-nasm@v1` and `jdx/mise-action@v2` target Node.js 20 and are
  forced onto Node.js 24 (a warning only).

## Step 2

- `AppId` is `dev.minodisk.handpic` (the product identifier, `IDENTIFIER`),
  not a GUID: the uninstall key reads `dev.minodisk.handpic_is1` under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall`, and a plain
  string needs none of the `{{` escaping a braced GUID needs in an `.iss`.
  `native-app.md` records it as a never-change rule.
- The Inno Setup docs, read before relying on the planned directives:
  `{autopf}` maps to `{userpf}` in non-administrative mode (which
  `PrivilegesRequired=lowest` gives), the Uninstall key is the `AppId` plus
  `_is1`, and `x64compatible` is the value for an x64 binary. The runner
  confirmed the resulting path:
  `C:\Users\runneradmin\AppData\Local\Programs\Handpic\`.
- `windows-latest` ships Inno Setup 6.7.1 with `ISCC.exe` on `PATH` as
  Chocolatey's shim (`/c/ProgramData/Chocolatey/bin/iscc`), so the workflow
  installs nothing. The compile takes about 15 s. This host has no Inno
  Setup, and it was not installed (see the pending manual check below).
- Under Git Bash, ISCC's `/D` / `/O` / `/F` options are passed with
  `MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'` so MSYS does not rewrite them
  as paths, and every path is handed over through `cygpath -w`.
- The ZIP comes from `$SYSTEMROOT/System32/tar.exe -a` (bsdtar): Git Bash's
  own `tar` is GNU tar, which writes no ZIP. It was tried here on a dummy
  folder first. The ZIP holds one `Handpic/` folder with the exe and
  `licenses/`, like the Linux tarball's planned top-level folder.
- The workflow's check runs in `pwsh`. It uses `Start-Process -Wait
  -PassThru` for the exit codes, then checks the exe, the notices, the
  `HKCU` entry's `DisplayVersion`, the absence of an `HKLM` (or
  `WOW6432Node`) entry and the Start menu shortcut. It does this twice (the
  install, then the upgrade over it), then runs the silent uninstall. The
  uninstaller hands over to a copy of itself in `%TEMP%`, so the check waits
  up to 60 s for the exe and the key to go.
- Runs: 44077941693 was green on the first attempt (the Windows leg took 21
  min with a cold cache, 19m22s of it the release build). The second
  dispatch, 44081604694, came after `windows.sh` started printing the ISCC
  path and banner (it had run with `/Q`). Its Windows leg passed in 3m21s
  (warm cache), but the `macos-latest` arm64 leg failed in `macos.sh` with
  `hdiutil: create failed - No space left on device` while copying
  `riffle-native` into the image. That failure is environmental: no macOS
  file changed, and the same code passed in the run before. The failed job
  was re-run, and the run then went green on all four jobs. An accidental
  dispatch, 44081542762, ran on an unchanged
  commit and was cancelled.
- Artifacts of run 44077941693, downloaded with `gh run download`:
  - `Handpic-0.0.0-windows-x64-setup.exe` (18,714,068 bytes). Its version
    resource reads `Handpic Setup`, product version `0.0.0`, company
    `minodisk`.
  - `Handpic-0.0.0-windows-x64.zip` (25,030,326 bytes), holding
    `Handpic/riffle-native.exe` and the six notices under
    `Handpic/licenses/`.
  - The two DMGs.
  - `SHA256SUMS`, which lists all four files and passes `sha256sum -c`.

  The setup was not run on this machine.

## Deferred issues (todo candidates)

- Pending manual check (Step 1, macOS only, the user's): `mise run
  native:package:macos` on a Mac writes
  `crates/native/target/release/Handpic-0.0.0-macos-<arch>.dmg`; open it, drag
  `Handpic.app` to `Applications` and start it (Gatekeeper asks for the ad
  hoc signed app's first launch to be allowed under System Settings, Privacy
  & Security). Not runnable on the Windows host. Source: plan Step 1, "Done
  when" (`mise run native:package:macos`); files `mise.toml`,
  `tools/package/macos.sh`. Step 1's checkbox was ticked on the automated
  criteria (the green `Package` run 43986612196 and `mise run ci:local`).
- Pending manual check (Step 2, Windows, the user's): install Inno Setup 6
  (`winget install JRSoftware.InnoSetup`), then:
  1. Run `mise run native:package:windows`. It should write
     `crates/native/target/release/Handpic-0.0.0-windows-x64-setup.exe` and
     `Handpic-0.0.0-windows-x64.zip`.
  2. As a standard (non-admin) user, run the setup. Expect no UAC prompt,
     the app under `%LOCALAPPDATA%\Programs\Handpic`, and a Start menu entry
     `Handpic` that starts it.
  3. Run the setup again silently (`/VERYSILENT /SUPPRESSMSGBOXES
     /NORESTART`). Expect no prompt.
  4. Uninstall from Settings, Apps.

  Not run here: this host has no Inno Setup, which was not installed for
  the step, and the hosted runner is elevated, so it cannot prove there is
  no UAC prompt. Source: plan Step 2, "Done when" (`mise run
  native:package:windows`) and its approach (the UAC part left for a real
  machine). Files: `mise.toml`, `tools/package/windows.sh`,
  `crates/native/packaging/windows/handpic.iss`. Step 2's checkbox was
  ticked on the automated criteria (the green `Package` runs and `mise run
  ci:local`).
- `hdiutil create` can fail with `No space left on device` on the
  `macos-latest` runner (run 44081604694, arm64 leg, during Step 2). The
  same code passed in the runs before and after. `tools/package/macos.sh`
  lets `hdiutil` size the image from `-srcfolder`. Giving it headroom (for
  example `-size` from `du` plus a margin, or a retry) would make the
  `Package` run less flaky. Not done in Step 2, which touches no macOS
  file. Files: `tools/package/macos.sh`, `.github/workflows/package.yml`.
