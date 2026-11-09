# Contributing

## Building and running

Prerequisites:

- **macOS**: Xcode Command Line Tools (`xcode-select --install`). Tauri needs no
  other system dependency there.
- **Windows**: MSVC Build Tools, the WebView2 runtime, and `nasm` (`mozjpeg-sys`
  builds libjpeg-turbo from source and needs it for SIMD on x86).

The Rust toolchain, Node and pnpm all come from `mise install`.

```sh
mise run tauri:dev
```

That is a debug build of the Rust side: quick to compile, slow at runtime. **Any
timing measurement has to come from the optimized build instead**, because the
numbers in [docs/humans/performance.md](./docs/humans/performance.md) are all optimized ones and a debug build is
not comparable to them:

```sh
mise run tauri:release:devtools
```

Both tasks run `pnpm install` first. `tauri:dev` starts the Vite dev server on
`localhost:1420` through `beforeDevCommand` (the frontend reloads on save). `tauri:release:devtools` also passes
`--features devtools`: Tauri only wires the webview's devtools up automatically
in a debug build, so without it there is no console to read the timings from. A
distributable build leaves the feature off.

`mise run tauri:dev` and `mise run tauri:release:devtools` need no signing key. A
`pnpm tauri build` does: it creates the updater artifacts, which are signed
with `TAURI_SIGNING_PRIVATE_KEY` (and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`), so
the build fails without them.

Both tasks also get a **Debug** menu, which a distributable build does not have.
Its `Timing logs` item turns on the 1:1 view's keypress → invoke → bitmap
timings, logged to the console; it starts unchecked on every launch.

To try the distributable build itself, bundle it the way the release workflow
does:

```sh
mise run tauri:release
```

The bundles land under `target/release/bundle/`. It skips the updater
artifacts, which need the release signing key.

### Native app (GPUI)

`crates/native` is the GPUI port of the desktop app. It is a separate Cargo
workspace with its own `Cargo.lock` and its own toolchain, Rust 1.98.1 (the
`mise.toml` next to it), which is what Zed's GPUI tracks. A `cargo` run from
the repository root never sees it. It depends on `crates/backend`, the
Tauri-free library shared with `crates/app` and a member of the root workspace,
so a backend change is checked by both `mise run ci` and `mise run native:check`.

NASM is a hard prerequisite on x86 and x86_64: the crate's `build.rs` fails
without it. `mozjpeg-sys` otherwise falls back to a build without SIMD with
only a warning cargo hides, and a RAF preview then decodes in about 99 ms
instead of 46 ms.

- Install it with `winget install NASM.NASM` on Windows, `apt-get install nasm`
  on Debian / Ubuntu or `brew install nasm` on macOS.
- winget's package installs per user (`%LOCALAPPDATA%\bin\NASM`), not under
  `C:\Program Files\NASM`, and does not add itself to `PATH`. Add that
  directory yourself. `mozjpeg-sys` only looks for `nasm` on `PATH`, so the build
  fails unless `nasm` is on `PATH`, whatever the `NASM` variable holds.
- `RIFFLE_NATIVE_ALLOW_NO_SIMD=1` turns the failure into a warning, for builds
  whose timings do not matter.

```sh
mise run native:dev                # debug build; fast to compile, slow at runtime
mise run native:release            # optimized build; the only one to take timings from
mise run native:check              # clippy and the tests
```

`mise run ci` does not run `native:check` (CI has a `native` job for it); only
the formatting check of the crate is part of `mise run lint`.

## Checks

- `mise run fmt` formats both Rust (`cargo fmt`) and the frontend (`vp fmt`).
- `mise run lint` runs `pnpm exec vp check` (format, lint, types) plus the other
  OS-independent checks (`cargo fmt --check`, shellcheck, actionlint, and an offline
  `lychee` check of Markdown links and `#heading` anchors).
- Frontend tests run with `pnpm exec vp test` (`pnpm exec vp test watch` while
  developing); `mise run test` runs them once.
- `mise run ci` runs both, as CI does. Node 24 comes from `mise install`.

## Adding a Lightroom label preset language

The Lightroom color label presets in the settings window come from one JSON
file per language in `crates/core/i18n/`. See
[its README](./crates/core/i18n/README.md) for the shape and how to check it.

## The index

The folder index is a SQLite cache, one row per file keyed by absolute path,
in the app cache directory:

- **macOS**: `~/Library/Caches/com.minodisk.riffle/index.sqlite`
- **Windows**: `%LOCALAPPDATA%\com.minodisk.riffle\index.sqlite`

Deleting it only costs one more scan (plus any judgments not yet written to
a sidecar). Nothing runs `VACUUM`, so renaming a folder leaves its old rows
behind and the file does not shrink.

## The benchmark CLI

`crates/cli` builds `riffle-cli`, used to inspect files and measure decode
and scan speed:

```sh
cargo build --release
./target/release/riffle-cli info     <file.ARW|file.DNG>            # where the embedded JPEGs are
./target/release/riffle-cli focusbox <file.ARW|file.DNG> <out.png>  # draw the focus box on the preview
./target/release/riffle-cli crop     <file.ARW|file.DNG> <out.png> [size]  # partially decode the focus point at 1:1
./target/release/riffle-cli bench    <file.ARW|file.DNG>...         # measure decode speed
./target/release/riffle-cli scan     <dir> [threads]                # extract a whole folder in parallel
```

## Promotion stats

The [Stats workflow](./.github/workflows/stats.yml) runs once a day and
commits a snapshot to the orphan `stats` branch, which holds only CSV files
and a README (no code, no CI). `downloads.csv` has the columns
`date,tag,asset,download_count`: one row per asset of every release
(prereleases and drafts included) per day, dated in UTC. The counts are the
cumulative totals GitHub reports, so the downloads of a period are the
difference between two dated rows. Every asset keeps its own row, so the
installers (`.dmg`, `-setup.exe`, `.msi`, `.AppImage`, `.deb`, `.rpm`) can be
told apart from the updater-only artifacts (`latest.json`, `.app.tar.gz`) and
the `.sig` files; on Windows the updater fetches the same `-setup.exe` users
install from, so that count covers both.

The other files come from the repository's traffic and metadata, which GitHub
keeps for 14 days at most:

- `views.csv` and `clones.csv`: `date,count,uniques`, the page views (or
  clones) and unique visitors (or cloners) of each UTC day. Every run rewrites
  the days of the 14-day window GitHub returns, so a missed run is filled in
  by the next one, and the latest day is partial until the next day's run.
- `referrers.csv` and `paths.csv`: `date,referrer,count,uniques` and
  `date,path,count,uniques`, the top referring sites and most viewed paths
  over the 14 days before `date`, with their views and unique visitors.
  GitHub gives no per-day breakdown of these, so each date is a rolling
  aggregate and a missed day is lost.
- `stars.csv`: `date,stargazers_count,forks_count`, the totals on that day.

The workflow reads all of them with the `STATS_TOKEN` repository secret, a
fine-grained personal access token for this repository with
`Contents: Read and write` and `Administration: Read-only` (the traffic API
needs the latter, which `GITHUB_TOKEN` cannot be granted). A missing, expired
or under-scoped secret fails the run, downloads included, rather than
silently skipping traffic; renew it before the 14-day window runs out.

To take a snapshot now, run the workflow by hand from the Actions tab or with
`gh workflow run stats.yml`. A second run on the same UTC day replaces that
day's rows. To write the files locally instead (with a `gh` login that can
read the repository's traffic; `GH_REPO` defaults to `minodisk/riffle`):

```sh
tools/stats/snapshot.sh <dir>
```
