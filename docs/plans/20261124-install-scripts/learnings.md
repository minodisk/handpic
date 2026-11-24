# Learnings: one-line install scripts

## Step 1

- Latest release: `install.sh` follows the redirect of
  `https://github.com/minodisk/handpic/releases/latest`
  (`curl -w '%{url_effective}'`) and takes the tag from the URL, so it needs
  no API call and no JSON parsing; `install.ps1` uses the API's
  `releases/latest` through `Invoke-RestMethod`.
- macOS replace: the extracted bundle is moved to
  `/Applications/.Handpic.app.install` first, then the old bundle is removed
  and the staged one renamed into place; the exit trap removes a leftover
  staged bundle.
- Verified on this Mac (macOS, arm64) against the real release v2.0.1:
  - `sh install.sh` over an existing `/Applications/Handpic.app` that a
    browser DMG install had left with `com.apple.quarantine`: replaced it;
    `xattr -p com.apple.quarantine /Applications/Handpic.app` fails (no
    quarantine on any file in the bundle), `codesign --verify --strict`
    passes. `spctl -a` still says "rejected" (ad hoc signature), which does
    not matter because Gatekeeper only assesses quarantined files.
  - `open /Applications/Handpic.app` started the app with no Gatekeeper
    dialog (CoreServicesUIAgent had no window); it was quit with osascript.
  - `cat install.sh | sh` (the piped form) over a missing
    `/Applications/Handpic.app`: installed, same `xattr` / `codesign` results.
    The one-liner against the branch's raw URL was not run (the branch was not
    pushed at the time); the piped local copy is the same code path.
  - A stub `curl` that zeroes the digests of `SHA256SUMS`: exit 1,
    "checksum mismatch ... nothing installed", the existing bundle's mtime
    unchanged, no staged bundle left, the temporary folder removed.
  - A stub `uname` printing `FreeBSD amd64` and `Linux aarch64`: exit 1 with
    "unsupported platform: <os> <arch>".
  - A `PATH` without `curl`: exit 1, "curl is required but was not found".
- Verified on Linux in a `debian:stable-slim` container
  (`--platform linux/amd64`, emulated on the Mac) as a non-root user:
  installs `~/.local/bin/handpic`, the desktop entry and the icon through the
  release's `install.sh` asset. Without `python3` it exits 1 with
  "python3 is required but was not found".
- `pwsh` is not installed on this Mac, so `install.ps1` was not even
  parsed here; it was checked only to be ASCII without a BOM.
- `mise run ci:local`: the `mise.toml` change selects every check. The first
  run stopped at `clippy app` because NASM was not on this Mac's `PATH`
  (environmental); after `brew install nasm` the second run passed every
  selected check.

## Deferred issues (todo candidates)

- **Pending manual check (Windows, `install.ps1`)**: on a real Windows
  machine (x64, ideally also Windows 11 on Arm), run
  `irm https://raw.githubusercontent.com/minodisk/handpic/main/install.ps1 | iex`
  (before merge: the branch's raw URL or a local copy piped to `iex`) in
  Windows PowerShell 5.1 and in PowerShell 7. Expected: it prints the
  download and install lines, leaves
  `%LOCALAPPDATA%\Programs\Handpic\Handpic.exe` (with `unins000.exe` beside
  it), the Start menu shortcut and the `HKCU` uninstall entry; no SmartScreen
  warning on the setup or on the first launch of `Handpic.exe`; a forced
  failure (e.g. a wrong digest in a local copy) prints the error and leaves
  the shell open, and `$ErrorActionPreference` / `$ProgressPreference` of the
  session are unchanged afterwards. Basis: the plan's Done-when for
  `install.ps1`, not checkable on the macOS host or in CI. The step's
  checkbox was ticked on the automated criteria and the macOS / Linux checks
  above. Files: `install.ps1`.
