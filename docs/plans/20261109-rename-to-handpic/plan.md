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

# Rename Riffle to Handpic and publish a landing page

## Purpose

"Riffle" collides with an earlier photo culling tool
(`github.com/sheshbabu/riffle`), and the app is about to be promoted to
general users. The app, the repository and the docs become **Handpic**
(hand + pic, a play on handpick), and a one-page site at
`https://handpic.minodisk.dev` (GitHub Pages, Cloudflare DNS) gives those
users a demo placeholder, the three selling points, the supported cameras,
per-OS download buttons and the first-launch steps for the unsigned app.

A user on the last Riffle release must receive the update through the
existing updater and end up with **one** app named Handpic, with its
identifier `com.minodisk.handpic`, that still has their judgments, settings,
index, ranking, model and viewed history, and with no `com.minodisk.riffle`
folder left on disk. That constraint decides most of what below is renamed
and what is kept.

### What is renamed, what is kept, and why

| Item | Decision | Why |
| --- | --- | --- |
| `productName`, window title, every visible string, dialogs, menu, MCP tool descriptions | **Rename** | The user-facing name. |
| Tauri `identifier` `com.minodisk.riffle` | **Rename to `com.minodisk.handpic`, migrate the data** | Every data dir (`index.sqlite` in `app_cache_dir`, `settings.json` and `.window-state.json` in `app_config_dir`, `ranking.json`, the SigLIP model and Windows' `EBWebView` in `app_local_data_dir`, logs in `app_log_dir`) is `<base>/<identifier>`, so the first Handpic launch moves the old folders' contents to the new ones and deletes the old folders (Step 1's `migrate.rs`). Nothing in the update path keys on the identifier: `tauri-plugin-updater` 2.12.0 swaps the macOS bundle by path, runs the Windows installer and rewrites the AppImage / runs `dpkg` / `rpm`, so the update installs and relaunches on every OS. The one unavoidable cost is on macOS, whose folder-access grants (TCC) are keyed on the bundle id: the user is asked once more for Desktop / Documents / Downloads / removable volumes. |
| Log file name `Riffle.log` | **Becomes `Handpic.log`, in the new log dir** | `tauri_plugin_log` names the file after `package_info().name` (= `productName`) and puts it in `app_log_dir`, now `…/com.minodisk.handpic/…`. The migration carries the old `Riffle.log` into the new folder as history. `docs/agents/app-log.md`'s path table changes to `com.minodisk.handpic`; the greppable prefixes do not change, and the migration adds one `invariant:` row. |
| Log targets `riffle_app::...` | Follow the crate rename (Step 3) | Documented in `app-log.md`; updated there. |
| Settings keys, Tauri events | **Nothing to do** | None contains the name (verified by grep). |
| Sidecar signature (`x:xmptk="riffle"` in `xmp.rs`, `Software = "riffle"` in `dop.rs`) | **Rename to `handpic`** | Written only into freshly created sidecars; nothing in the app or PhotoLab parses it back, so old sidecars stay compatible and new ones carry the new name. |
| `copy::TEMP_SUFFIX` `.riffle-tmp` | **Rename to `.handpic-tmp`** | Internal and short-lived (only during an import run); `watch.rs` filters it, its tests follow. |
| MCP server name `riffle` and the `claude mcp add … riffle` snippet | **Rename to `handpic`** | A user's existing `riffle` alias in their MCP client config keeps working (the alias is local). |
| Cargo crate names `riffle-core` / `riffle-app` / `riffle-cli`, binary `riffle-cli`, `RIFFLE_*` test env vars | **Rename (Step 3)** | Internal only, but the repository stops saying "riffle" anywhere new contributors look; the log targets and `CONTRIBUTING.md` follow. Mechanical. |
| Updater endpoint `github.com/minodisk/riffle/...` | **Rename to `minodisk/handpic`** | GitHub redirects the old URL and `tauri-plugin-updater` uses reqwest's default redirect policy, so old installs still find `latest.json`. |
| Updater `pubkey` | **Keep** | The signing key is unchanged; a new key would make every old install reject the update. |
| App icons | **Keep** | Not name-bearing. A new icon is a separate piece of work. |
| `CHANGELOG.md`, `docs/plans/_archived/**`, `todo.md` history | **Keep as history** | Rewriting history helps nobody. |

### Decisions made by the user (2026-11-09)

1. macOS bundle name after the update: **document only**. The updater swaps
   Handpic into `/Applications/Riffle.app`; the Dock and the menu bar say
   Handpic, only the folder name is stale, and `usage.md` tells the user they
   may rename it in Finder. No first-launch rename code.
2. Version of the rename release: **2.0.0** (`feat(app)!:`).
3. Cargo crates: **rename** (Step 3).
4. Download buttons: **client-side GitHub API**, with the Releases page as the
   no-JS and failure fallback.
5. Landing page languages: **two HTML files** (`/` and `/ja/`).
6. `LICENSE`: **not changed by this plan**; the landing page must not claim a
   license until the user decides separately.
7. macOS first-launch text: **lead with Privacy & Security → Open Anyway** for
   macOS 15 and later; the user confirms on a Mac.
8. macOS leftovers outside Tauri's four dirs: **delete them too** on first
   launch, after the data migration succeeded (`~/Library/WebKit/<old id>`,
   `~/Library/HTTPStorages/<old id>`, `~/Library/Preferences/<old id>.plist`,
   `~/Library/Saved Application State/<old id>.savedState`). macOS-only, checked
   by hand on a Mac.
9. A destination file that already exists: **keep the destination and discard
   the old file**, per file, so the old folder is always removed. In the rare
   case of Handpic having run before the Riffle update, the old `settings.json`
   or `ranking.json` is lost.

## Steps

- [ ] Step 1: Make the installers migrate a Riffle install in place, and add the data-dir migration (no visible change yet)
  - Done when: (a) a Handpic build (local `mise run tauri:release` with `productName` and `identifier` temporarily changed for the check only) installed over Riffle 1.10.0 leaves one app on each OS; (b) `crates/app/src/migrate.rs` exists with unit tests over temp dirs covering a plain move, a cross-volume fallback (copy, verify, delete), an interrupted run resumed (destination files already present), a destination with data (decision 9), a read-only source (nothing deleted, failure reported), identical old and new paths (no-op) and a missing old dir (no-op); (c) it is wired at the top of `main()` but is a no-op on `main` because `tauri.conf.json` still says `com.minodisk.riffle`, so nothing user-visible ships; (d) `mise run ci` passes.
  - Implementation approach (as far as it is known):
    - **Windows NSIS** (`crates/app/tauri.conf.json` `bundle.windows.nsis.installerHooks`, new `crates/app/nsis/hooks.nsh`): a `NSIS_HOOK_PREINSTALL` macro that reads `SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\Riffle"` (`UninstallString`, `InstallLocation`) and, when present, runs that uninstaller with `/P /UPDATE _?=<dir>` then removes the leftover `uninstall.exe` and folder. Tauri's template keys this entry on `PRODUCTNAME`, not the identifier, and in `/UPDATE` mode it never uninstalls a previous version, so without the hook the updater installs Handpic next to Riffle. Verify in Tauri's `installer.nsi` and on the real 1.10.0 uninstaller that this run skips the remove-app-data branch, so `%APPDATA%\com.minodisk.riffle` and `%LOCALAPPDATA%\com.minodisk.riffle` are still there for Handpic's first launch to migrate. The install mode is the default `currentUser`, so `SHCTX` is `HKCU`. The hook is a no-op when the key is absent.
    - **Windows MSI**: set `bundle.windows.wix.upgradeCode` to the UpgradeCode the Riffle 1.10.0 MSI carries (read it from the released `Riffle_1.10.0_x64_en-US.msi` with the WindowsInstaller COM object or `msiinfo`), because Tauri derives the default from `productName` and the config schema says it "must stay the same across all of your updates".
    - **Linux**: `bundle.linux.deb.replaces` / `conflicts` = `["riffle"]` and `bundle.linux.rpm.obsoletes` = the Riffle package name; confirm the exact old names with `dpkg-deb -f Riffle_1.10.0_amd64.deb Package` and `rpm -qp --qf '%{NAME}' Riffle-1.10.0-1.x86_64.rpm`. The AppImage is rewritten in place by the updater, so its file name stays `Riffle_1.10.0_amd64.AppImage`; document, do not fix.
    - **macOS bundle folder**: no code (decision 1).
    - **Data-dir migration, `crates/app/src/migrate.rs`**:
      - Runs first thing in `main()`, before `tauri::Builder` is built, because `tauri_plugin_log` opens `Handpic.log` and `tauri_plugin_window_state` reads `.window-state.json` during plugin registration, and the index and the settings store open in `setup`. It therefore cannot use `app.path()`; it computes the dirs with the `dirs` crate (the version Tauri already resolves in `Cargo.lock`, added as a direct dependency of the app crate) by Tauri's own formulas in `tauri-2.11.6/src/path/desktop.rs`: config `dirs::config_dir()/<id>`, local data `dirs::data_local_dir()/<id>`, cache `dirs::cache_dir()/<id>`, log `~/Library/Logs/<id>` on macOS else `data_local_dir()/<id>/logs` (inside local data, so not a separate pair there). The new identifier comes from `tauri::generate_context!()`'s `config().identifier`, the old one from a `LEGACY_IDENTIFIERS: &[&str] = &["com.minodisk.riffle"]` constant. Pairs whose old path equals the new path, or whose old path does not exist, are skipped; duplicate pairs (Windows: cache == local data) are deduplicated by old path.
      - Per pair `(old_dir, new_dir)`: create `new_dir`; walk `old_dir` recursively; for each file, if the destination exists, discard the source (decision 9), else move it: `fs::rename` first; on failure (cross-volume `EXDEV`, or any error) copy to `<dest>.migrating`, compare lengths, `fs::rename` onto `<dest>`, then delete the source. Only when every entry of the pair succeeded is `remove_dir_all(old_dir)` called. Any failure leaves `old_dir` (what was already moved stays moved; the tmp-then-rename makes a present destination a complete file, so the next launch resumes idempotently) and is recorded, and the app does not start (it shows an error dialog and exits before the logger exists, so nothing is created in the new folders next to the only real copy). A stale `<dest>.migrating` from an interrupted copy is overwritten.
      - Everything under the old dirs is moved, not a known-file list, so `index.sqlite-wal` / `-shm`, the legacy `last_folder` file, the old `Riffle.log` and Windows' `EBWebView` all travel.
      - The function returns a report (`Vec<String>`: one line per pair, success or failure with the path and the error). A failed pair is shown in an error dialog and the process exits, so no `invariant: data migration` line is ever logged (see `docs/agents/app-log.md`); the lines also go to stderr, which a Windows release build discards. The function has no `unwrap` on IO and returns every error.
      - macOS leftovers (decision 8): after every pair succeeded, delete `~/Library/WebKit/<old id>`, `~/Library/HTTPStorages/<old id>`, `~/Library/Preferences/<old id>.plist` and `~/Library/Saved Application State/<old id>.savedState`, `#[cfg(target_os = "macos")]`, each failure reported the same way.
      - Unit tests build both trees under `std::env::temp_dir()` as the other `crates/app` tests do and call the migration with explicit paths (the dir computation is a thin separate function), so they run on CI on all three OSes; the cross-volume path is tested by injecting a rename that fails.
    - Record the per-OS check results in `learnings.md`; the CI runners cannot do this, so the checks are manual, on the user's machines where available, with the Linux ones in a VM or container.

- [ ] **Gate (user, manual): trademark search.** Before Step 2 is merged, the user confirms the search for HANDPIC / HANDPICK / ハンドピック on J-PlatPat and USPTO, classes 9 and 42, found no blocking registration, and records the date and result in `learnings.md`. The iPhone app "Handpick: Photo Organizer" (2026-10-02) is known and judged not a blocker. Nothing user-visible under the new name is merged before this line.

- [ ] Step 2: Rename the product to Handpic (app, docs, repository)
  - Done when: the app, its window title, dialogs, update messages, MCP server, settings dialog, README pair, `docs/humans/*` pairs, `CLAUDE.md`, `CONTRIBUTING.md`, issue templates and `tools/stats/snapshot.sh` say Handpic; the updater endpoint and every `github.com/minodisk/riffle` link point at `minodisk/handpic`; `crates/app/tauri.conf.json` says `identifier: com.minodisk.handpic`, and a first launch of this build on a machine with Riffle 1.10.0's data moves it under `com.minodisk.handpic` and leaves no `com.minodisk.riffle` folder (Step 1's migration, now live), with the open folder, ratings, settings, ranking, model and viewed history intact; `mise run ci` passes (lychee's offline link check included).
  - Manual (user), before merging this PR: Step 1's criterion (a), the install-over-Riffle checks listed in `learnings.md`, which the user chose on 2026-11-09 to run after merging Step 1 (Step 1 stays unticked until they pass; the Windows NSIS check is the one that must not be skipped).
  - Manual (user), right before merging this PR: rename the repository `minodisk/riffle` to `minodisk/handpic` on GitHub, then run `curl -sI https://github.com/minodisk/riffle/releases/latest/download/latest.json` and confirm a `301` to the new name. Never create a new repository named `minodisk/riffle` afterwards (that would end the redirect and strand every Riffle install). Update the local remotes (`git remote set-url`).
  - Implementation approach:
    - `crates/app/tauri.conf.json`: `productName`, window `title`, updater `endpoints`, and `identifier` → `com.minodisk.handpic`. Keep `pubkey`. The identifier change and the migration (Step 1) must ship in the same release, which this ordering guarantees: Step 1 is merged first and inert, this step makes it live.
    - `docs/agents/app-log.md` path table and `CONTRIBUTING.md` index paths: `com.minodisk.handpic`; add the `data migration` row to the `invariant:` table; note that the old `Riffle.log` sits beside `Handpic.log` in the new log dir.
    - `crates/app/src/update.rs`, `main.rs` (menu / log folder comments), `mcp.rs` (`INSTRUCTIONS`, error strings, `Implementation::new("handpic", …)` and its test), `diagnostics.rs`, `import.rs`, `perf.rs`, `exif.rs`, `sidecar.rs` comments; `crates/core/src/xmp.rs` `x:xmptk`, `dop.rs` `Software`, `copy.rs` `TEMP_SUFFIX` and `crates/app/src/watch.rs` tests.
    - Frontend: `crates/app/ui/index.html` (`<title>`, the sidecar notice), `ui/src/main.ts` (`shownTitle`, `renderTitle`, overlay labels `Riffle CPU` / `Riffle disk`), `companion.ts` and its tests, `mcp.ts` and its test, `refresh.ts`, `uncaught.ts`, `perf.ts`, `filter.ts`, `meta.ts` comments.
    - Docs: `README.md` / `README.ja.md`, `docs/humans/usage.md` / `.ja.md`, `performance`, `cameras`, `raw-formats` pairs, `CLAUDE.md` (title and the two `Riffle.log` / `Riffle's` mentions), `CONTRIBUTING.md`, `docs/agents/app-log.md` (title, `Handpic.log`, table paths unchanged since the identifier is kept), `photolab.md`, `tauri-app.md`, `tract-onnx-inference.md`, `ui-styling.md`, `raw-metadata-parsing.md`, `.github/ISSUE_TEMPLATE/*.yml`, `.github/workflows/*.yml` comments, `tools/stats/snapshot.sh` default `GH_REPO`, `crates/core/i18n/README.md`.
    - Add an "Upgrading from Riffle" paragraph to `usage.md` (and `.ja.md`): the first Handpic launch moves your data to the new location and removes the old `com.minodisk.riffle` folders; on macOS the system asks once more for folder access, because the grant is tied to the app's identifier; on Windows the old Riffle entry is removed by the installer; on macOS the app folder stays `Riffle.app` and may be renamed in Finder; the Linux AppImage keeps its old file name; `.deb` / `.rpm`: the new package replaces the old; the old `Riffle.log` is carried beside `Handpic.log`.
    - README pair install table: `Handpic_<version>_…` file names. Fix the macOS first-launch text at the same time: on macOS 15 and later the right-click → Open override no longer works for unnotarized apps, so lead with System Settings → Privacy & Security → Open Anyway (after the blocked first attempt), keep right-click → Open for macOS 14 and earlier, and keep the `xattr -d com.apple.quarantine` line that `usage.md` already has. The agent cannot verify this on a Mac; ask the user to confirm on their machine and note it in `learnings.md`. The landing page (Step 4) copies this text.
    - Commit as `feat(app)!: rename the app to Handpic` (decision 2).
    - Leave `CHANGELOG.md`, `docs/plans/_archived/**`, `todo.md` as they are; the `20261010-burst-best-mark` in-flight plan too.

- [ ] Step 3: Rename the Cargo crates and the CLI
  - Done when: the workspace is `handpic-core` / `handpic-app` / `handpic-cli`, the CLI binary is `handpic-cli`, the ignored tests read `HANDPIC_*` env vars, `package.json` `name` is `handpic`, log targets are `handpic_app::…` in `docs/agents/app-log.md`, `CONTRIBUTING.md` and `tract-onnx-inference.md` follow, `release-please-config.json` `extra-files` paths still resolve (they are path-based, so unchanged), `Cargo.lock` is regenerated, `mise run ci` passes.
  - Implementation approach:
    - Mechanical: `Cargo.toml` of each crate and the `riffle-core` dependency lines; `use riffle_core::` → `use handpic_core::` across `crates/app`, `crates/cli`, tests; temp dir prefixes in tests (`riffle-app-…`); `crates/core/tests/*`; the `cargo test -p riffle-app` snippets in doc comments.
    - Assumes Step 2 is merged (so one diff is "the product", the other "the crates").
    - `mise.toml` has no crate name in it; check `tools/**` and `.claude/skills/**` scripts for `riffle-cli` / `target/release/riffle-cli`.
    - Does not change `identifier`, `productName` or anything from Step 2.

- [ ] Step 4: Landing page and its Pages deployment
  - Done when: `site/` holds the static page in English (`/`) and Japanese (`/ja/`), a new `.github/workflows/pages.yml` deploys it to GitHub Pages on a push to `main` that touches `site/**` (and on `workflow_dispatch`), the page shows a demo-media placeholder, the three selling points, the supported cameras (linking `docs/humans/cameras.md`), per-OS download buttons that resolve to the current release's files, and the first-launch steps per OS from Step 2's text; `https://handpic.minodisk.dev` serves it over HTTPS; `mise run ci` passes.
  - Manual (user), in this order: (1) Cloudflare DNS on the `minodisk.dev` zone: `CNAME handpic → minodisk.github.io`, DNS only (grey cloud). (2) GitHub → Settings → Pages: Source = GitHub Actions. (3) After the first successful deploy, set the custom domain `handpic.minodisk.dev` and, once the certificate is issued, tick Enforce HTTPS. The `site/CNAME` file in the artifact keeps the setting across deploys.
  - Implementation approach:
    - Source lives in `site/` at the repository root (not under `docs/`, which lychee and the plans conventions treat as Markdown; not inside the Tauri frontend, whose Vite root is `crates/app/ui`). Plain `index.html`, `ja/index.html`, `style.css`, `main.js`, `CNAME`, `assets/` (the app icon from `crates/app/icons/128x128@2x.png`, a placeholder box for the demo). No framework, no build step; the artifact is the folder.
    - Two HTML files (decision 5): works without JS, each has `<html lang>` and `<link rel="alternate" hreflang>`; the Japanese body is a translation of the English one, kept in the same PR (the `README.md` / `README.ja.md` rule extended to the site).
    - Download buttons (decision 4): `main.js` fetches `https://api.github.com/repos/minodisk/handpic/releases/latest` (CORS-enabled, 60 requests per hour per visitor IP, no token), picks the asset per button by pattern (`_aarch64.dmg`, `_x64.dmg`, `_x64-setup.exe`, `_amd64.AppImage`, plus `.deb` / `.rpm` links under Linux) and sets `href`; each button's static `href` is `https://github.com/minodisk/handpic/releases/latest`, so without JS or when the API fails the click still lands on the right page. Optionally preselect the visitor's OS from `navigator.platform` / UA hints and show the others below. `latest.json` was rejected: it lists only updater artifacts (no `.dmg`, `.deb`, `.rpm`).
    - Selling points: fast because it reads only the embedded previews; never writes to RAW; works with Lightroom / DxO PhotoLab through sidecars. Source for the camera list: `docs/humans/cameras.md`. The page states no license (decision 6).
    - `pages.yml`: `permissions: pages: write, id-token: write, contents: read`; `actions/configure-pages`, `actions/upload-pages-artifact` with `path: site`, `actions/deploy-pages`; `concurrency: pages`, no cancel. Document the workflow in `docs/agents/github-actions-workflows.md` in the same style as the Stats entry. `actionlint` runs on it in `mise run lint`.
    - Check during implementation whether `vp fmt` / `vp check` pick up `site/main.js` and `site/*.html` (the fmt and lint `ignorePatterns` in `vite.config.ts` are repository-wide); either let them format it or add `site/**` to the ignore lists, and say which in `learnings.md`. lychee only checks Markdown, so the site's links are checked by hand once.
    - Link the site from `README.md` / `README.ja.md` (top) and add `homepage` to `crates/app/Cargo.toml` only if it is useful for the bundler's metadata; otherwise skip.

- [ ] Step 5: Ship the first Handpic release and verify the upgrade from Riffle 1.10.0
  - Done when: the release-please PR is merged and the release published (user action, as every release); on each OS available to the user, a machine that had Riffle 1.10.0 installed and running has been updated by the in-app updater (not by a fresh install) and shows: one app named Handpic; the folder index, `settings.json`, `.window-state.json`, `ranking.json`, the downloaded model and the `views` table working under the new identifier (last folder reopens, ratings and picks shown, ranking on, AI not re-downloading); no `com.minodisk.riffle` folder under `%APPDATA%` / `%LOCALAPPDATA%` (Windows), `~/Library/Application Support`, `~/Library/Caches`, `~/Library/Logs` and the decision-8 paths (macOS), `~/.config`, `~/.cache`, `~/.local/share` (Linux); `Help > Open Log Folder` opening the `com.minodisk.handpic` log dir with `Handpic.log` and the carried-over `Riffle.log`, and no data-migration error dialog having appeared; the landing page's buttons resolve to the new file names; the results and any surprises are in `learnings.md`, and any fix is a follow-up PR referencing this step.
  - Implementation approach:
    - Before merging the release PR, confirm Steps 1–4 are on `main` and the repository is already renamed, so `latest.json` and the assets are published under `minodisk/handpic` with `Handpic_…` names, and the previous `Riffle` installs reach them through the redirect.
    - Check the Windows path twice: an NSIS-installed Riffle (the common case) and an MSI-installed one (verifies `upgradeCode`).
    - Also test the resume path once: make the old cache dir read-only before launching, confirm the error dialog and that the app exits without creating files in the new folders, restore, relaunch, confirm the app starts and the old folder is gone.
    - `docs/humans/usage.md` pair: adjust the "Upgrading from Riffle" paragraph to what was observed, if it differs.

## Risks

- **Repository rename and the redirect.** Old installs depend on `github.com/minodisk/riffle` redirecting forever. Any new repository under that name ends the redirect and strands every unupdated Riffle. GitHub Pages, Discussions links in the issue templates and `tools/stats/snapshot.sh` follow the rename through redirects, but the plan updates them anyway.
- **Windows hook correctness.** The pre-install hook runs inside the updater's passive `/P /UPDATE` run after Riffle quit. If the old uninstaller prompts (it should not under `/P`) the update would hang silently; test on a real Riffle 1.10.0 install before merging Step 2.
- **MSI UpgradeCode.** If the pinned value does not match the Riffle MSI, MSI-installed users get a side-by-side install. The value must be read from the actual released MSI, not recomputed.
- **Trademark gate timing.** Step 1 is safe to merge before the gate (no visible change); Steps 2–5 are not. If the search finds a conflict, Step 1's config is name-agnostic enough to reuse under another name.
- **The migration runs before the logger.** A failure is shown only in the error dialog (and stderr, discarded on a Windows release build), and a panic inside the migration would leave no trace in `Handpic.log`.
- **macOS folder grants reset.** TCC keys on the bundle identifier; after the update macOS asks again for the folders the user opened. No code can carry this over; the "Upgrading from Riffle" paragraph says so.
- **Identifier must change together with the migration.** Step 1 is merged first and inert, Step 2 flips the identifier; neither may ship without the other in the release that changes the identifier.
- **Riffle's uninstaller and the old data on Windows.** An interactive uninstall with the checkbox ticked deletes `com.minodisk.riffle`; the hook's passive run must not. Confirm on the real 1.10.0 uninstaller before merging Step 2.
- **Landing page lint.** `vp check` is type-aware and may reject a plain `site/main.js`; either ignore `site/**` in `vite.config.ts` or write the file so it passes. Decide in Step 4.

## Progress

- (2026-11-09) Step 1 implemented and locally reviewed; merged with criterion (a), the manual per-OS install checks, still pending and moved in front of Step 2's merge by the user's decision.
