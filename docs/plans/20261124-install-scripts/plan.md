<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# One-line install scripts

## Purpose

Releases are not Developer ID signed or notarized (the macOS app is signed ad
hoc in `tools/package/macos.sh`) and carry no Authenticode signature, so a
copy downloaded with a browser is blocked by Gatekeeper on macOS and warned
about by SmartScreen on Windows. Gatekeeper only blocks files carrying
`com.apple.quarantine`, which browsers set and `curl` does not; files fetched
by PowerShell's `Invoke-WebRequest` get no Mark of the Web. The app's own
updater already installs this way.

Once this is done, a user installs Handpic with one line and no first-launch
permission step:

- macOS / Linux: `curl -fsSL https://raw.githubusercontent.com/minodisk/handpic/main/install.sh | sh`
- Windows: `irm https://raw.githubusercontent.com/minodisk/handpic/main/install.ps1 | iex`

The install lands where the app's updater expects it, so it keeps updating
itself afterwards.

## Steps

- [x] Step 1: Add `install.sh` and `install.ps1` at the repository root, make
      them the primary install method in the docs, and shellcheck `install.sh`
  - Done when:
    - `install.sh` (POSIX sh, `#!/bin/sh`) at the repository root:
      - macOS (`arm64`, `x86_64`): resolves the latest release, downloads
        `Handpic-<version>-macos-<arch>.tar.gz` and `SHA256SUMS` with `curl`,
        refuses a digest mismatch before anything is installed, and installs
        `/Applications/Handpic.app`. The installed bundle has no
        `com.apple.quarantine` attribute
        (`xattr -p com.apple.quarantine /Applications/Handpic.app` fails),
        passes `codesign --verify --strict`, and opens without the Gatekeeper
        dialog. Running it again over an existing install replaces it.
      - Linux (`x86_64`): downloads `Handpic-<version>-linux-x64.tar.gz`,
        the release's own `install.sh` asset and `SHA256SUMS`, and installs
        through that asset to `~/.local/bin/handpic` (no root).
      - Any other OS or architecture exits non-zero with a message naming
        what `uname` reported; a missing required tool (`curl`, a SHA-256
        tool, `python3` on Linux) exits non-zero with a message.
      - A digest mismatch exits non-zero and leaves an existing install
        untouched (checked by hand with a stub, see the approach).
    - `install.ps1` at the repository root: resolves the latest release,
      downloads `Handpic-<version>-windows-x64-setup.exe` and `SHA256SUMS`,
      refuses a digest mismatch (`Get-FileHash`), and runs the setup silently,
      leaving `%LOCALAPPDATA%\Programs\Handpic\Handpic.exe`, the Start menu
      shortcut and the `HKCU` uninstall entry. Neither the setup nor the
      first launch shows a SmartScreen warning. It works when piped to `iex`
      in Windows PowerShell 5.1 and in PowerShell 7, and a failure does not
      close the user's shell. A 32-bit Windows is refused with a message.
    - `README.md` and `README.ja.md` (in sync): the Install section leads
      with the two one-liners; the per-OS file table and the first-launch
      permission notes stay as the alternative (manual download), with
      `xattr -d com.apple.quarantine /Applications/Handpic.app` named for
      macOS.
    - `docs/humans/usage.md` and `docs/humans/usage.ja.md` (in sync): the
      Installing section describes the one-liners first (what each installs
      and where, that the download is checked against `SHA256SUMS`), keeps
      the per-file instructions, and tells the repository-root `install.sh`
      apart from the release's `install.sh` asset.
    - `CLAUDE.md` names the two root scripts in its layout description.
    - `install.sh` is shellchecked by `mise run lint` and selected by
      `mise run ci:local` when it changes, with a case in
      `tools/ci/local.test.sh`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Location: the repository root, so the raw URLs are
      `https://raw.githubusercontent.com/minodisk/handpic/main/install.sh`
      and `.../install.ps1`. The checked-in Linux installer stays at
      `crates/app/packaging/linux/install.sh`; do not move or rename it (the
      release asset `install.sh`, `tools/package/assets.sh` and
      `package.yml`'s checks depend on it).
    - Asset names are fixed by `tools/package/assets.sh`. The macOS
      architecture in the name is what `uname -m` prints (`arm64`,
      `x86_64`), as `tools/package/macos.sh` names it. `SHA256SUMS` is the
      output of `sha256sum -- *`: `<hex digest>  <file name>` per line; pick
      the line by exact file name (`awk '$2 == name'`, as `package.yml`
      does).
    - Latest release: only a final release is "latest" (release candidates
      are published with `--latest=false`). Decide between following the
      redirect of `https://github.com/minodisk/handpic/releases/latest` to
      read the tag (no API rate limit, no JSON parsing in sh) and the API's
      `releases/latest` (60 unauthenticated requests per hour per IP; easy in
      PowerShell with `Invoke-RestMethod`). Download assets from
      `https://github.com/minodisk/handpic/releases/download/v<version>/<name>`.
    - `install.sh`:
      - `set -eu`; put the whole body in a function called on the last line,
        so a download cut short runs nothing. Work in a `mktemp -d` folder
        removed by a trap.
      - SHA-256: `shasum -a 256` on macOS, `sha256sum` on Linux.
      - macOS: extract with the system `tar` into the temporary folder,
        run `codesign --verify --strict` on the extracted `Handpic.app`, and
        only then replace `/Applications/Handpic.app`. The tarball holds
        `Handpic.app/` only (`package.yml` checks this and that the
        extracted bundle keeps its ad hoc signature). Do not use `sudo`: the
        updater swaps the bundle in place and needs its parent folder
        writable by the user (`misplaced` in `crates/engine/src/update.rs`).
        If `/Applications` is not writable, fail with a message.
      - Linux: after checking the tarball's digest, run the downloaded
        release asset as `sh install.sh <tarball> <digest>`; it re-checks the
        digest and installs the binary, desktop entry and icon under
        `~/.local`, which is what the updater's `Tarball` install kind is
        (`detect`: the executable in a `.local/bin` folder). Check the
        downloaded `install.sh` asset against `SHA256SUMS` too before running
        it. Do not reimplement its logic and do not extract the signed
        tarball with GNU `tar` here (the asset reads it with Python's
        `tarfile`).
      - Do not launch the app; print where it was installed.
    - `install.ps1`:
      - Use the setup exe, not the portable ZIP: it is the per-user Inno
        Setup install (`crates/app/packaging/windows/handpic.iss`,
        `PrivilegesRequired=lowest`, no UAC prompt) the updater detects as
        `Inno` by `unins000.exe` beside the exe. Run it with
        `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, the switches
        `package.yml`'s check and the updater's `setup_args` use, and wait
        for its exit code.
      - Written for `irm | iex`: no `exit` (it would close the user's
        shell; `throw` instead), the body in a function or script block so
        `$ErrorActionPreference = 'Stop'` and `$ProgressPreference` do not
        leak into the session, TLS 1.2 enabled for Windows PowerShell 5.1,
        ASCII only without a BOM (`.gitattributes` checks every text file
        out with LF).
      - Architecture: the setup allows `x64compatible` (x64 and Windows 11
        on Arm); refuse only what the setup would refuse.
    - shellcheck wiring, three places kept in step: the `shellcheck -x` line
      of `[tasks.lint]` in `mise.toml`, and in `tools/ci/local.sh` the
      `shellcheck_patterns` array (add `'^install\.sh$'`) and the
      `shellcheck-all` line of `run_check`; add an `expect none "install.sh"
      "shellcheck install.sh"` case to `tools/ci/local.test.sh`. A change to
      `mise.toml` makes `mise run ci:local` select every check, so the local
      run is the long one. There is no PowerShell linter in the repository;
      do not add one.
    - Docs: `README.ja.md` and `docs/humans/usage.ja.md` are the Japanese
      translations and change in the same PR. Keep the heading anchors
      `#installing` / `#インストール` (README links to them; lychee checks
      fragments). Leave
      `.claude/skills/release/references/reinstall-notice.md` as it is.
    - To verify during implementation, and record in `learnings.md`:
      - On this Mac, from the branch (`sh install.sh`, and the one-liner
        against the branch's raw URL): the three macOS checks above
        (`xattr`, `codesign`, a first launch with no dialog), over a missing
        and over an existing `/Applications/Handpic.app`.
      - Digest mismatch and unsupported platform, with a stub `curl` /
        `uname` first on `PATH` (a stub `curl` that serves a wrong
        `SHA256SUMS`; a stub `uname` printing e.g. `FreeBSD` or `Linux` +
        `aarch64`): non-zero exit, a message, nothing installed or replaced.
      - Linux, in a container or VM with a scratch `HOME`: the install
        lands in `~/.local/bin/handpic` (the app need not be launched).
      - Windows needs a real Windows machine: the install, the absence of a
        SmartScreen warning on the setup and on the first launch, and the
        `iex` behavior on Windows PowerShell 5.1. This cannot be checked on
        the macOS development host or in CI; see the risks.

## Trade-offs and risks

- **Two files named `install.sh`.** A release already carries an asset
  called `install.sh` (the Linux per-user installer,
  `crates/app/packaging/linux/install.sh`, run as
  `sh install.sh ARCHIVE EXPECTED_SHA256`). The plan keeps the requested name
  for the repository-root script and has the docs tell the two apart. The
  alternative is another name for the root script (e.g. `get.sh`), which
  avoids the confusion but changes the requested one-liner URL. Renaming the
  release asset instead would touch `assets.sh`, `package.yml` and the
  documented manual install, and is not proposed.
- **Linux through the release's `install.sh` asset vs. reimplemented in sh.**
  Delegating reuses the tested installer (unsafe-entry checks, the desktop
  entry's `Exec` rewrite, staged replaces) and always matches the release's
  tarball layout, at the cost of needing Python 3. A pure-sh install would
  drop that dependency but duplicate logic that `package.yml` tests only for
  the asset. The plan delegates.
- **Windows: the setup exe vs. the portable ZIP.** The setup gives the Start
  menu shortcut, the uninstall entry and the `Inno` update path. The ZIP
  would need the script to choose a folder and create a shortcut itself. The
  plan uses the setup.
- **`/Applications` not writable (a non-admin macOS user).** The plan fails
  with a message. The alternative is a fallback to `~/Applications`, which
  the updater also accepts (any writable parent), at the cost of an install
  location the docs do not otherwise mention. `sudo` is not an option: a
  root-owned bundle could not be swapped by the updater.
- **An automated test of `install.sh`.** The plan wires shellcheck only and
  checks digest mismatch and unsupported platforms by hand with stubs. A
  small `install.test.sh` (stub `curl` and `uname` on `PATH`, added to
  `bash_tests` in `tools/ci/local.sh`, `[tasks.lint]` and `local.test.sh`'s
  `every` list) would pin those two acceptance criteria in CI, for more
  wiring and a test that never exercises a real install. Left out as
  over-engineering unless the caller wants the criteria covered in CI.
- **Launching after the install.** The plan does not launch the app (the
  setup's `/RELAUNCH` and `open` are left unused) and prints the location.
  Launching would be friendlier but is an extra behavior nobody asked for.
- **Windows cannot be verified here.** The SmartScreen criterion needs a
  manual run on a real Windows machine. Before merge that means the branch's
  raw URL or a local copy of `install.ps1`; the `main` one-liner works only
  after the merge. If no Windows run happens before merge, the criterion is
  open at merge time. SmartScreen's behavior for an unsigned exe without
  Mark of the Web is expected to be silent, but it has not been measured for
  this setup.
- **What the checksum proves.** `SHA256SUMS` comes from the same release as
  the asset, so the check catches a corrupted or truncated download, not a
  compromised release; trust rests on HTTPS to github.com. The updater
  additionally checks a zipsign signature, which the scripts cannot do
  without the tool. The docs should not claim more than the digest check.
- **Quarantine depends on how the script is run.** `curl` from Terminal sets
  no quarantine attribute. The plan does not strip the attribute defensively
  (`xattr -dr`); if a measured case shows a quarantined result, add it then.
- **A running app during a reinstall.** Replacing `/Applications/Handpic.app`
  while Handpic runs leaves the old process running until it quits. On
  Windows the silent setup falls back to the Restart Manager for a running
  `Handpic.exe`. The plan does not detect or quit a running app.

## Progress
