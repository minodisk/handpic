# Learnings: capital-exe-name

## Step 1

- **Inno Setup upgrade over a lowercase file (Measured 2026-11-23, Windows
  11, Inno Setup 6.7.3 installed per user with `winget install
  JRSoftware.InnoSetup --scope user`).** A throwaway script with its own
  `AppId` (`dev.minodisk.handpic-casetest`), `PrivilegesRequired=lowest` and
  a `DefaultDirName` in the session's scratch folder installed a file as
  `x.exe` (bytes `one`); a second setup of the same `AppId` installed new
  bytes with `DestName: "X.exe"`. Both ran `/VERYSILENT /SUPPRESSMSGBOXES
  /NORESTART`. The folder then held only `X.exe` (bytes `two`), `unins000.exe`
  and `unins000.dat`: Inno writes the new casing, so `handpic.iss` gets no
  `[InstallDelete]`. The throwaway was uninstalled with its own
  `unins000.exe /VERYSILENT`; the folder and its `HKCU` uninstall key
  (`dev.minodisk.handpic-casetest_is1`) were gone afterwards. The real
  `dev.minodisk.handpic` install was never touched.
- **`mise run app:package:windows` (Measured 2026-11-23).** It built and
  compiled the setup and the ZIPs. The portable ZIP, listed with Windows'
  own `C:\Windows\System32\tar.exe -tf` (Git Bash's GNU `tar` cannot read a
  ZIP: "This does not look like a tar archive"), holds `Handpic/`,
  `Handpic/Handpic.exe`, `Handpic/licenses/` and the five notice files under
  it, no `handpic.exe`. The setup was not installed here (its `AppId` is the
  real one and its `[Icons]` would overwrite the user's Start menu
  shortcut); the `{app}\Handpic.exe` it installs is left to the hand checks.
- **`self_update`'s `bin_name` names no path.** In 1.3.3 it only fills the
  `{{ bin }}` placeholder of `bin_path_in_archive` (with the platform's exe
  suffix appended), so a fixed `bin_path_in_archive` with no placeholder
  makes the constant `handpic` harmless. The ZIP entry is matched exactly:
  a ZIP holding only `Handpic/handpic.exe` installs nothing on Windows too
  (`a_zip_holding_only_the_lowercase_exe_installs_nothing`).
- **The target keeps the running exe's spelling.** With
  `bin_install_path(exe)`, a portable `handpic.exe` updated from a ZIP
  holding `Handpic/Handpic.exe` is still listed as `handpic.exe` on NTFS
  (`a_signed_zip_replaces_the_portable_exe_and_keeps_its_name`, asserted from
  `read_dir`).
- `crates/app/src/main.rs` and `crates/app/src/update.rs` pass the
  `current_exe` path and never build an archive path; no change was needed.

## Deferred issues (todo candidates)

- **Pending manual checks (already added to `todo.md`, do not add them
  again).** The plan's hand checks are in `todo.md` under the heading "App:
  real-device checks of the capital executable name", as the plan's Done-when asked: a real
  upgrade over a 2.0.0 Inno Setup install (by hand and through `Restart
  Now`), a portable `handpic.exe` / `Handpic.exe` update from a test feed, a
  2.0.0 portable copy's expected failure, the macOS bundle and its update,
  and a `Package` dispatch's listings. Step 1's checkbox was ticked on the
  automated criteria. Basis: plan Step 1, "Pending hand checks to add to
  `todo.md`". Files: `tools/package/windows.sh`,
  `crates/app/packaging/windows/handpic.iss`, `tools/package/macos.sh`,
  `crates/app/packaging/macos/Info.plist`, `mise.toml`,
  `crates/engine/src/update.rs`.
