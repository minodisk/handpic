# Contributing

## Building and running

Prerequisites:

- **macOS**: Xcode Command Line Tools (`xcode-select --install`).
- **Windows**: MSVC Build Tools.
- **Linux**: the development packages GPUI links against (fontconfig,
  Wayland, X11 and xkbcommon) and a Vulkan loader; the `app` job of
  [the CI workflow](./.github/workflows/ci.yml) lists them for Ubuntu.
- **All**: NASM on x86 and x86_64 (below).

The Rust toolchains come from `mise install`.

`crates/app` is the desktop app, built on GPUI. It is a separate Cargo
workspace with its own `Cargo.lock` and its own toolchain, Rust 1.98.1 (the
`mise.toml` next to it), which is what Zed's GPUI tracks. A `cargo` run from
the repository root never sees it. It depends on `crates/core` and
`crates/engine`, the GUI-free libraries of the root workspace, so a backend
change is checked by both `mise run ci` and `mise run app:check` (or, as
the agents do, by `mise run ci:local` and the PR's checks; see
[Checks](#checks)).

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
- `HANDPIC_NATIVE_ALLOW_NO_SIMD=1` turns the failure into a warning, for builds
  whose timings do not matter.

```sh
mise run app:dev                # debug build; fast to compile, slow at runtime
mise run app:dev:screenshot     # debug build with the MCP `screenshot` tool (not for timings)
mise run app:watch              # debug build, rebuilt and restarted on a source change
mise run app:release            # optimized build; the only one to take timings from
mise run app:check              # clippy and the tests
```

**Any timing measurement has to come from the optimized build**
(`app:release`), because the numbers in
[docs/humans/performance.md](./docs/humans/performance.md) are all optimized
ones and a debug build is not comparable to them.

`mise run ci` does not run `app:check` (CI has an `app` job for it); only
the formatting check of the crate is part of `mise run lint`.

## Checks

- `mise run fmt` formats both Rust workspaces (`cargo fmt`).
- `mise run lint` runs the OS-independent checks (`cargo fmt --check`,
  shellcheck, actionlint, an offline `lychee` check of Markdown links and
  `#heading` anchors, and the bash tests of the scripts).
- `mise run test` runs clippy and the tests of the root workspace.
- `mise run ci` runs both, as CI does.
- `mise run ci:local` is the light check the agents run before every commit or
  push (`tools/ci/local.sh`). It selects each check from the branch's changed
  paths: the working tree against `git merge-base HEAD origin/main`, committed
  and uncommitted, plus untracked files, with no fetch. A Markdown-only change
  runs only the offline `lychee` check (also run when a file was deleted or
  renamed); a changed shell script is shellchecked, a workflow change runs
  `actionlint`, and a bash test runs when its script changed. A Rust change runs the `cargo fmt` check
  of its workspace and `cargo clippy -p` / `cargo test -p` of only the changed
  root packages (`handpic-core`, `handpic-engine`, `handpic-cli`; the root
  `Cargo.toml` / `Cargo.lock` count as all three), the native clippy for a
  `crates/app`, `crates/core` or `crates/engine` change, and the native
  tests for a `crates/app` change. A change to `mise.toml` or
  `.shellcheckrc`, or a missing merge base, runs every check. It prints what it
  selected and what it left out. It runs on this platform only and does not
  test the crates that depend on a changed one: the PR's `lint`, `test` (three
  platforms) and `app` jobs cover those.

## Adding a Lightroom label preset language

The Lightroom color label presets in the settings window come from one JSON
file per language in `crates/core/i18n/`. See
[its README](./crates/core/i18n/README.md) for the shape and how to check it.

## The index

The folder index is a SQLite cache, one row per file keyed by absolute path,
in the app cache directory:

- **macOS**: `~/Library/Caches/dev.minodisk.handpic/index.sqlite`
- **Windows**: `%LOCALAPPDATA%\dev.minodisk.handpic\index.sqlite`

Deleting it only costs one more scan (plus any judgments not yet written to
a sidecar). Nothing runs `VACUUM`, so renaming a folder leaves its old rows
behind and the file does not shrink.

## The benchmark CLI

`crates/cli` builds `handpic-cli`, used to inspect files and measure decode
and scan speed:

```sh
cargo build --release
./target/release/handpic-cli info     <file.ARW|file.DNG>            # where the embedded JPEGs are
./target/release/handpic-cli focusbox <file.ARW|file.DNG> <out.png>  # draw the focus box on the preview
./target/release/handpic-cli crop     <file.ARW|file.DNG> <out.png> [size]  # partially decode the focus point at 1:1
./target/release/handpic-cli bench    <file.ARW|file.DNG>...         # measure decode speed
./target/release/handpic-cli scan     <dir> [threads]                # extract a whole folder in parallel
```

## Promotion stats

The [Stats workflow](./.github/workflows/stats.yml) runs once a day and
commits a snapshot to the orphan `stats` branch, which holds only CSV files
and a README (no code, no CI). `downloads.csv` has the columns
`date,tag,asset,download_count`: one row per asset of every release
(prereleases and drafts included) per day, dated in UTC. The counts are the
cumulative totals GitHub reports, so the downloads of a period are the
difference between two dated rows. Every asset keeps its own row, so the
Tauri releases' installers (`.dmg`, `-setup.exe`, `.msi`, `.AppImage`, `.deb`, `.rpm`) can be
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
read the repository's traffic; `GH_REPO` defaults to `minodisk/handpic`):

```sh
tools/stats/snapshot.sh <dir>
```
