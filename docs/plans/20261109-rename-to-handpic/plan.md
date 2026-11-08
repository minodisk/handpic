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
existing updater and end up with **one** app named Handpic that still has
their judgments, settings, index and viewed history. That constraint
decides most of what below is renamed and what is kept.

### What is renamed, what is kept, and why

| Item | Decision | Why |
| --- | --- | --- |
| `productName`, window title, every visible string, dialogs, menu, MCP tool descriptions | **Rename** | The user-facing name. |
| Tauri `identifier` `com.minodisk.riffle` | **Keep** | Every data dir (`index.sqlite` in `app_cache_dir`, `settings.json` in `app_config_dir`, `ranking.json` and the SigLIP model in `app_local_data_dir`, logs in `app_log_dir`) and the NSIS "remove app data" path are keyed on it. Changing it would orphan all of them and (on macOS) break the updater's bundle match. It is never shown to a user except in the log path. |
| Log file name `Riffle.log` | **Becomes `Handpic.log`** by itself | `tauri_plugin_log` names the file after `package_info().name`, which is `productName`. The old `Riffle.log` stays beside it, harmless. `docs/agents/app-log.md` and `Help > Open Log Folder` docs follow. The greppable prefixes (`panic:`, `uncaught-js:`, `invariant:`) do not change. |
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

## Steps

- [ ] Step 1: Make the installers migrate a Riffle install in place (no visible change yet)
  - Done when: a Handpic build (local `mise run tauri:release` with `productName` temporarily set to `Handpic` for the check only) installed over Riffle 1.10.0 leaves one app on each OS, with settings, index, ranking and model intact; the config lands on `main` while `productName` is still `Riffle`, so nothing user-visible ships; `mise run ci` passes.
  - Implementation approach (as far as it is known):
    - **Windows NSIS** (`crates/app/tauri.conf.json` `bundle.windows.nsis.installerHooks`, new `crates/app/nsis/hooks.nsh`): a `NSIS_HOOK_PREINSTALL` macro that reads `SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\Riffle"` (`UninstallString`, `InstallLocation`) and, when present, runs that uninstaller with `/P` and `_?=<dir>` then removes the leftover `uninstall.exe` and folder. Tauri's template keys this entry on `PRODUCTNAME`, not the identifier, and in `/UPDATE` mode it never uninstalls a previous version, so without the hook the updater installs Handpic next to Riffle. Verify Riffle's passive uninstall leaves `%APPDATA%\com.minodisk.riffle` and `%LOCALAPPDATA%\com.minodisk.riffle` (it only removes them when the user ticks the checkbox). The install mode is the default `currentUser`, so `SHCTX` is `HKCU`. The hook is a no-op when the key is absent.
    - **Windows MSI**: set `bundle.windows.wix.upgradeCode` to the UpgradeCode the Riffle 1.10.0 MSI carries (read it from the released `Riffle_1.10.0_x64_en-US.msi` with the WindowsInstaller COM object or `msiinfo`), because Tauri derives the default from `productName` and the config schema says it "must stay the same across all of your updates".
    - **Linux**: `bundle.linux.deb.replaces` / `conflicts` = `["riffle"]` and `bundle.linux.rpm.obsoletes` = the Riffle package name; confirm the exact old names with `dpkg-deb -f Riffle_1.10.0_amd64.deb Package` and `rpm -qp --qf '%{NAME}' Riffle-1.10.0-1.x86_64.rpm`. The AppImage is rewritten in place by the updater, so its file name stays `Riffle_1.10.0_amd64.AppImage`; document, do not fix.
    - **macOS**: no code (decision 1).
    - Record the per-OS check results in `learnings.md`; the CI runners cannot do this, so the checks are manual, on the user's machines where available, with the Linux ones in a VM or container.

- [ ] **Gate (user, manual): trademark search.** Before Step 2 is merged, the user confirms the search for HANDPIC / HANDPICK / ハンドピック on J-PlatPat and USPTO, classes 9 and 42, found no blocking registration, and records the date and result in `learnings.md`. The iPhone app "Handpick: Photo Organizer" (2026-10-02) is known and judged not a blocker. Nothing user-visible under the new name is merged before this line.

- [ ] Step 2: Rename the product to Handpic (app, docs, repository)
  - Done when: the app, its window title, dialogs, update messages, MCP server, settings dialog, README pair, `docs/humans/*` pairs, `CLAUDE.md`, `CONTRIBUTING.md`, issue templates and `tools/stats/snapshot.sh` say Handpic; the updater endpoint and every `github.com/minodisk/riffle` link point at `minodisk/handpic`; `mise run ci` passes (lychee's offline link check included).
  - Manual (user), right before merging this PR: rename the repository `minodisk/riffle` to `minodisk/handpic` on GitHub, then run `curl -sI https://github.com/minodisk/riffle/releases/latest/download/latest.json` and confirm a `301` to the new name. Never create a new repository named `minodisk/riffle` afterwards (that would end the redirect and strand every Riffle install). Update the local remotes (`git remote set-url`).
  - Implementation approach:
    - `crates/app/tauri.conf.json`: `productName`, window `title`, updater `endpoints`. Keep `identifier` and `pubkey`.
    - `crates/app/src/update.rs`, `main.rs` (menu / log folder comments), `mcp.rs` (`INSTRUCTIONS`, error strings, `Implementation::new("handpic", …)` and its test), `diagnostics.rs`, `import.rs`, `perf.rs`, `exif.rs`, `sidecar.rs` comments; `crates/core/src/xmp.rs` `x:xmptk`, `dop.rs` `Software`, `copy.rs` `TEMP_SUFFIX` and `crates/app/src/watch.rs` tests.
    - Frontend: `crates/app/ui/index.html` (`<title>`, the sidecar notice), `ui/src/main.ts` (`shownTitle`, `renderTitle`, overlay labels `Riffle CPU` / `Riffle disk`), `companion.ts` and its tests, `mcp.ts` and its test, `refresh.ts`, `uncaught.ts`, `perf.ts`, `filter.ts`, `meta.ts` comments.
    - Docs: `README.md` / `README.ja.md`, `docs/humans/usage.md` / `.ja.md`, `performance`, `cameras`, `raw-formats` pairs, `CLAUDE.md` (title and the two `Riffle.log` / `Riffle's` mentions), `CONTRIBUTING.md`, `docs/agents/app-log.md` (title, `Handpic.log`, table paths unchanged since the identifier is kept), `photolab.md`, `tauri-app.md`, `tract-onnx-inference.md`, `ui-styling.md`, `raw-metadata-parsing.md`, `.github/ISSUE_TEMPLATE/*.yml`, `.github/workflows/*.yml` comments, `tools/stats/snapshot.sh` default `GH_REPO`, `crates/core/i18n/README.md`.
    - Add an "Upgrading from Riffle" paragraph to `usage.md` (and `.ja.md`): what the update does per OS (Windows: the old Riffle entry is removed by the installer; macOS: the folder stays `Riffle.app` and may be renamed in Finder; Linux AppImage: the file keeps its old name; `.deb` / `.rpm`: the new package replaces the old), that data and settings stay, and that `Riffle.log` is left beside `Handpic.log`.
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
  - Done when: the release-please PR is merged and the release published (user action, as every release); on each OS available to the user, a machine that had Riffle 1.10.0 installed and running has been updated by the in-app updater (not by a fresh install) and shows: one app named Handpic, the folder index, `settings.json`, `ranking.json`, the downloaded model and the `views` table intact, `Help > Open Log Folder` opening the `com.minodisk.riffle` log dir with `Handpic.log`; the landing page's buttons resolve to the new file names; the results and any surprises are in `learnings.md`, and any fix is a follow-up PR referencing this step.
  - Implementation approach:
    - Before merging the release PR, confirm Steps 1–4 are on `main` and the repository is already renamed, so `latest.json` and the assets are published under `minodisk/handpic` with `Handpic_…` names, and the previous `Riffle` installs reach them through the redirect.
    - Check the Windows path twice: an NSIS-installed Riffle (the common case) and an MSI-installed one (verifies `upgradeCode`).
    - `docs/humans/usage.md` pair: adjust the "Upgrading from Riffle" paragraph to what was observed, if it differs.

## Risks

- **Repository rename and the redirect.** Old installs depend on `github.com/minodisk/riffle` redirecting forever. Any new repository under that name ends the redirect and strands every unupdated Riffle. GitHub Pages, Discussions links in the issue templates and `tools/stats/snapshot.sh` follow the rename through redirects, but the plan updates them anyway.
- **Windows hook correctness.** The pre-install hook runs inside the updater's passive `/P /UPDATE` run after Riffle quit. If the old uninstaller prompts (it should not under `/P`) the update would hang silently; test on a real Riffle 1.10.0 install before merging Step 1.
- **MSI UpgradeCode.** If the pinned value does not match the Riffle MSI, MSI-installed users get a side-by-side install. The value must be read from the actual released MSI, not recomputed.
- **Trademark gate timing.** Step 1 is safe to merge before the gate (no visible change); Steps 2–5 are not. If the search finds a conflict, Step 1's config is name-agnostic enough to reuse under another name.
- **Landing page lint.** `vp check` is type-aware and may reject a plain `site/main.js`; either ignore `site/**` in `vite.config.ts` or write the file so it passes. Decide in Step 4.

## Progress

- (none yet)
