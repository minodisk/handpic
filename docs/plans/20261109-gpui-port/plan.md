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

# Port the desktop UI from Tauri 2 / WebView2 to GPUI, starting with a preview pager

## Purpose

Fast preview display is Riffle's first property. The latency plan
(`../20261108-preview-latency-breakdown/plan.md`) gave every stage of the
Tauri path its own number and wrote a rule, R5, for when native rendering is
even on the table: only if `ipc + post + worker + draw` stays over 50 ms at
p95 with `read` under 10 ms, and "if that sum is under 20 ms at p95 at any
point, write that a rewrite cannot win more than that and close the
question". The same plan put the port's size at about 15k lines of
TypeScript and 48 test files against a pre-1.0 toolkit.

A throwaway spike (a bare GPUI window, `reader::read_preview` + mozjpeg to
BGRA on a background thread, `paint_image`, timed the same way) was then
measured against the Tauri app on the same kind of files. Windows, SATA SSD
`D:`, idle, warm page cache, `opt-level = 3`, SIMD mozjpeg, GPUI n=20,
timer-driven presses, p50 in ms:

| Preview | Tauri `keypressToPixels` | GPUI `keypressToPixels` |
|---------|--------------------------|-------------------------|
| ARW 1616x1080 | **19.4** (p95 22.6; read 6.0, ipc 4.0, decode 6.0, draw 1.0) | **15.56** (p95 24.44; read 0.74, hop 0.03, decode 6.71, paint+present 0.83, vsync wait 6.27; **8.34 excluding the vsync wait**) |
| RAF 4416x2944 | **97.8** (p95 113.0; read 3.0, ipc 28.0, decode 55.2, draw 6.0; n=7) | **57.89** (p95 68.11; read 1.80, hop 0.39, decode 46.05, paint+present 3.55, vsync wait 6.81; **51.72 excluding it**) |

The two columns do not end at the same place: Tauri's number stops at
`drawImage`, GPUI's includes the vsync wait and `Present`. The 5 ms `read`
gap on 1616x1080 is not attributable to GPUI (its cause is unconfirmed). The
240 fps on-screen comparison was not done. By R5's own arithmetic this gap
(about 4 ms on ARW, 40 ms on RAF, most of the RAF gap being the IPC copy of
a 1.3-5.5 MiB payload) would have closed the native question. **The user
chose to port anyway**, on these numbers and on the removal of the WebView
hop, the JPEG copy and the `createImageBitmap` worker as a class of cost.

This plan therefore does two things. It specifies the first vertical slice in
full: a new GPUI app crate, `crates/native` (`riffle-native`), that opens a
folder and pages its RAW previews with the arrow keys through `riffle_core`,
applies the EXIF orientation, logs per-page stage timings comparable to the
Tauri `page` line, and is measured against the spike. And it lays out the
parity milestones coarsely, so the order is agreed before the next one is
planned in detail. The Tauri app in `crates/app` stays the shipped app, is
not edited by this plan's slice (another session is implementing preview
prefetch there), and keeps building under the unchanged root workspace,
toolchain and `Cargo.lock` until the switch-over milestone.

Other in-flight plans: `../20261107-preview-read-gate/plan.md` (done but for a
hand check), `../20261108-preview-latency-breakdown/plan.md` (Step 2 onward:
the prefetch work in `crates/app`), `../20261108-full-size-eye-check/plan.md`,
`../20261010-burst-best-mark/plan.md`. None touches the files this plan adds.

### Toolchain facts the steps rest on

- GPUI is taken from Zed's repository at git rev
  `b5cd310b3b54dfc659fcc06453e6358843dfe216`, crates `gpui` and
  `gpui_platform` (crates.io `gpui 0.2.2` is from 2025-10 and stale). Zed's
  `rust-toolchain.toml` pins **1.98.1**; the repo's `mise.toml` pins 1.97
  (1.98.1 is already installed through mise on the user's machine: `mise ls
  rust` lists both).
- Zed's `[patch.crates-io]` entries that GPUI needs:
  `async-task = { git = "https://github.com/zed-industries/async-task.git", rev = "015845a19fc31752c02367567d3b8900a406f9da" }`
  and `async-process = { git = "https://github.com/zed-industries/async-process.git", rev = "0b6d6713570af61806e1e5cb40e0f757cb93fd9d" }`.
  Zed's other patches (`cpal`, `notify`, `calloop`, `khronos-egl`, ...) are
  not needed by `gpui` / `gpui_platform` and must not be copied: the repo
  uses crates.io `notify 8.2.0` in `crates/app`.
- Lock comparison (repo `Cargo.lock`, 648 packages, vs the spike's, 644):
  `async-task 4.7.1` and `async-process 2.5.0` are **already in the repo
  lock** (pulled by Tauri's Linux tree), so a workspace-wide patch would
  swap the Tauri app's copies for Zed's forks. About 170 shared packages
  differ by a compatible version only (the repo's are newer; cargo would keep
  them); duplicates GPUI would add: `windows 0.57` next to 0.61/0.62, the
  `objc2 0.5` family next to 0.6, `rand 0.8` / `0.9` next to 0.10,
  `quick-xml 0.30` next to 0.42. `image 0.25.10`, `windows 0.62`,
  `objc2-foundation 0.3.2` and `raw-window-handle 0.6.2` already agree.
- GPUI on Linux needs, per Zed's `script/linux`: `libfontconfig-dev`,
  `libwayland-dev`, `libx11-xcb-dev`, `libxkbcommon-x11-dev`, `libvulkan1`
  (plus a Vulkan-capable driver at run time). macOS needs only the Xcode CLT
  (Metal). Windows needs nothing beyond MSVC.
- NASM: `mozjpeg-sys`'s build script runs `nasm -v`, prints a
  `cargo:warning` and **continues without SIMD** when it fails; cargo hides
  build-script warnings of non-path dependencies, so the fallback is
  silent. On the user's machine `C:\Program Files\NASM` is on `PATH` but the
  directory does not exist (`which nasm` finds nothing); RAF decode measured
  about 99 ms without SIMD and 46 ms with it. `nasm-rs` honors a `NASM`
  environment variable as the binary path.
- GPUI facts: `RenderImage` holds BGRA in an `image::RgbaImage`; every image
  handed to the renderer must be released with `cx.drop_image(image, None)`
  or it leaks on the GPU; textures are created with one mip level, so a
  large downscale (RAF fit to a 1600 px window) may alias (visually
  unconfirmed); the image sprite (`PolychromeSprite`) has no
  `transformation`, only the monochrome SVG sprite does, so rotation cannot
  be done by the painter; IME handling exists in `gpui_windows` but is
  unverified; there is no public after-`Present` hook, `cx.defer` from the
  paint closure runs after the frame's `draw()` + `present()` and is the
  workaround the spike used; `cx.prompt_for_paths(PathPromptOptions)` is the
  native folder picker.

## Steps

- [x] Step 1: Scaffold `crates/native` as its own workspace with the GPUI toolchain, the NASM check and a Windows CI job
  - Done when:
    - `crates/native/Cargo.toml` declares package `riffle-native` (bin
      `riffle-native`, edition 2024, `publish = false`) and its own
      `[workspace]` (empty table) so it is a separate Cargo workspace with
      its own `crates/native/Cargo.lock`, committed. Dependencies:
      `gpui = { git = "https://github.com/zed-industries/zed", rev = "b5cd310b3b54dfc659fcc06453e6358843dfe216" }`,
      `gpui_platform = { git = ..., rev = ... }` (same), `riffle-core = { path = "../core" }`,
      `mozjpeg = "0.10"`, `image = { version = "0.25", default-features = false }`;
      the two `[patch.crates-io]` entries above and no other. `[profile.release]
      opt-level = 3` as the root. The root `Cargo.toml` gains
      `exclude = ["crates/native"]` and nothing else; the root `Cargo.lock` is
      byte-identical before and after (checked in the PR).
    - `crates/native/mise.toml` pins `rust = { version = "1.98.1", components = "rustfmt,clippy" }`.
      The root `mise.toml` gains three tasks that `cd crates/native` and run
      under that toolchain (`mise install` there, then `mise exec -- cargo ...`,
      or the task-level `tools` field if the installed mise, 2026.9.5, honors
      it; the implementation checks which and records it in `learnings.md`):
      `native:dev` (`cargo run -- "$@"`), `native:release`
      (`cargo run --release -- "$@"`; the only build timing numbers come from)
      and `native:check` (`cargo clippy --all-targets -- -D warnings` and
      `cargo test`). `lint` gains
      `cargo fmt --manifest-path crates/native/Cargo.toml --all -- --check`;
      `fmt` gains the matching format.
    - `crates/native/build.rs` fails the build on `x86` / `x86_64` targets
      when `nasm -v` run from `PATH` (whatever `NASM` holds, as `mozjpeg-sys`
      probes only that) does not run or reports `NASM version 0.`, with a message
      naming the install command (`winget install NASM.NASM` on Windows,
      `apt-get install nasm`, `brew install nasm`) and the escape hatch
      `RIFFLE_NATIVE_ALLOW_NO_SIMD=1`, which turns the failure into a
      `cargo:warning` (a path crate's warnings are shown). The check runs
      on every build of the crate (`cargo:rerun-if-env-changed=PATH`,
      `...=RIFFLE_NATIVE_ALLOW_NO_SIMD`).
    - `src/main.rs` opens one GPUI window titled `Riffle` with a black
      background and a one-line status text ("no folder" for now), and quits
      on `q` / window close. Nothing else.
    - `.github/workflows/ci.yml` gains a `native` job on `windows-latest`
      (timeout 30 min) that installs the Linux-independent prerequisites
      (checks NASM is on the runner image, else `ilammy/setup-nasm@v1`),
      restores a cargo cache keyed on `crates/native/Cargo.lock` (the Zed git
      clone lands in `~/.cargo/git/db`; `crates/native/target` is its own
      cache path) and runs `mise run native:check`; the `ci` aggregate job
      `needs` it and asserts its result. The existing `lint` and `test` jobs
      are unchanged beyond `lint`'s new fmt line.
    - `CONTRIBUTING.md` gains a "Native app (GPUI)" subsection: the
      toolchain (1.98.1 through the nested `mise.toml`), NASM as a hard
      prerequisite and why (the 99 ms vs 46 ms RAF decode), the three tasks.
      `CLAUDE.md`'s layout paragraph names `crates/native` in one sentence
      (what it is, that it is a separate workspace, that it must not depend
      on `crates/app`). A new `docs/agents/native-app.md` starts with the
      GPUI facts above as its first entries (same `(Hit)` / `(Measured)` /
      `(Inferred)` style as `tauri-app.md`).
    - `mise run ci` passes locally on Windows; the Tauri app builds and runs
      unchanged (`crates/app` has no diff).
  - Implementation approach:
    - Before anything else the user installs NASM (`winget install
      NASM.NASM`) and puts it on `PATH`; the step cannot be verified without it.
      Record the exact fix in `learnings.md`.
    - The separate workspace is the decision the user took (see "Trade-offs"
      below).
    - Verify once whether `gpui` at this rev actually fails on 1.97
      (`cargo +1.97 check` equivalent through mise) and write the result in
      `learnings.md`; the nested toolchain pin stays either way so the
      native crate tracks Zed's.
    - The first CI run clones the full Zed repository; expect minutes and
      record the duration. If it exceeds the job timeout, raise the timeout
      rather than vendoring.
    - `cargo:warning` text from `build.rs` is the only place the SIMD
      state is visible; do not try to read `mozjpeg-sys` features from the
      native crate.
- [x] Step 2: Open a folder and page its RAW previews with the arrow keys, oriented, with per-page stage timings
  - Done when:
    - `riffle-native [<folder>] [--auto N] [--size WxH] [--max N]`: with a
      folder argument it lists that folder; without one it shows the native
      folder picker (`cx.prompt_for_paths`, directories only) and lists the
      chosen folder; the listing is the files `riffle_core::scan::is_raw_file`
      accepts, directly in the folder, in `natord` file-name order (the
      order the Tauri strip's default sort uses). `--size` keeps only files
      whose embedded preview is WxH and `--max` caps the count, as the spike
      did, so the Step 3 measurement can select the same files.
    - `→` / `←` (and `Right` / `Left` on the keypad, `Home` / `End`) move the
      shown file with wraparound; `q` quits and prints the summary table.
      One preview request is in flight at a time; a result for a file that is
      no longer current is dropped and the current one is re-requested
      (the spike's `request` / `loaded`), so holding a key never queues
      stale decodes. The decode runs on `cx.background_spawn`:
      `reader::read_preview` (the 1 MiB prefix read plus a ranged read when
      the JPEG lies past it; a HDR PQ CR3 arrives as a JPEG through
      `hevc::to_jpeg` inside `reader` already), `mozjpeg::Decompress`
      straight to `JCS_EXT_BGRA`, then the orientation.
    - The EXIF orientation `Arw.orientation` (1, 3, 6, 8; the JPEG's own
      tag is neutralized by `reader`) is applied to the BGRA buffer on the
      background thread in a new `src/orient.rs` (`rotate_bgra(buf, w, h,
      orientation) -> (Vec<u8>, w, h)`, the 4-byte analogue of
      `riffle_core::decode::apply_orientation`), covered by a unit test on
      a 3x2 pattern for each of 1 / 3 / 6 / 8 against hand-written expected
      pixels. Its cost is timed as its own stage. The window then draws the
      upright image fit and centered, as the spike's canvas does, and
      releases the previous `RenderImage` with `cx.drop_image` after the
      new frame has been presented (the spike's `Pending.old`).
    - A pure `src/pager.rs` holds the state machine (current index,
      in-flight marker, the stale / re-request rule, wraparound) with unit
      tests: a result for a stale index re-requests the current one; a
      result for the current index clears the in-flight marker; two presses
      while in flight produce exactly one further request.
    - A pure `src/timing.rs` turns the `Instant`s of one page into the
      stages and prints, on stdout, one line per page
      `page wait=… read=… decode=… orient=… hop=… frameWait=… paint=… present=… keypressToPixels=… jpegKB=… px=WxH orientation=N file=…`
      (the Tauri line's `wait` / `read` / `decode` names mean the same thing;
      `ipc + post + worker` collapse into `hop`; `draw` splits into
      `frameWait + paint + present`; `keypressToPixels` includes `present`,
      which Tauri's does not — the docs say so), and `summary n=… ` with p50 /
      p95 per stage at quit, as the spike. A unit test pins the arithmetic
      (`keypressToPixels` is the end-to-end difference, never the sum of the
      stages, and `wait` is 0 for a lone press). `--auto N` presses `→` N
      times from a 1003 ms timer (out of phase with 60 Hz vsync) after a
      2 s settle and quits, so the measurement needs no hand on the keys.
    - The status text shows `index/total  file name  WxH  orientation`
      (one line, the window's only text) so a run can be followed by eye;
      an unreadable file shows `error: …` there and is logged, and paging
      continues.
    - `mise run ci` passes (the `native` job runs the new unit tests);
      `crates/app` has no diff. `docs/agents/native-app.md` gains what was
      learned (at least: the rotation cost measured, whether the aliasing
      on a RAF fit view is visible, and the `cx.defer`-after-paint timing
      pattern as the spike used it).
  - Implementation approach:
    - Assumes Step 1 merged. Start from the spike's `main.rs` reproduced in
      "Spike reference" below; split it into the modules above rather than
      growing one file.
    - Budget for `orient`: ≤ 2 ms on a 1616x1080 portrait preview and
      ≤ 15 ms on a 4000x3000 RAF in release; measure before and after a
      cache-friendly (row-blocked) loop and keep the simpler one that meets
      the budget. Landscape (orientation 1) must not copy at all.
    - `--size` filtering decodes headers only (`Decompress::new_mem` and
      `width()` / `height()`), as the spike's `select_files`, and only when
      the flag is given.
    - Keystroke names come from `KeyDownEvent.keystroke.key` (`"right"`,
      `"left"`, `"home"`, `"end"`, `"q"`); check them on Windows in the
      running app, they are lowercase in GPUI.
    - Do not add a meta pane, marks, a strip or thumbnails here; this step
      is the comparable pager and nothing more.
- [x] Step 3: Measure the pager against the spike on the compare folders and record the verdict
  - Done when:
    - `results.md` in this folder holds, for
      `D:\Photos\tests\20261108-gpui-compare\ARW` (`--size 1616x1080`) and
      `...\RAF` (`--size 4416x2944`), three runs each of
      `mise run native:release -- <folder> --size … --auto 20`, the per-run
      p50 / p95 of every stage and of `keypressToPixels`, the spike's row
      from the Purpose for comparison, and the measurement conditions as
      `docs/agents/tauri-app.md` "Write a performance number with its
      measurement conditions" demands (build, drive, idle, warm and how
      warmth was established, display refresh rate, GPU, SIMD on — the
      `build.rs` check having passed).
    - **Threshold** (changed on 2026-11-09 by the user's decision): the
      median of the three runs' p50 `keypressToPixels` is at most the p50 of
      the spike measured alternately in the same session, + 2.0 ms on ARW
      and + 5.0 ms on RAF (met: + 0.30 ms and + 1.26 ms). The original
      absolute gate (≤ 17.6 ms and ≤ 62.9 ms, from the spike's one run of
      15.56 / 57.89 ms) was missed, and the same spike binary could not meet
      it either (19.86 / 64.59 ms), so it could not tell a regression from
      a change of conditions. p95 is recorded and discussed, not gated: the
      spike's one run had p95 − p50 of 9-10 ms, which n=20 cannot gate. A portrait
      file (orientation 6 or 8) from `D:\photos\samples\<FORMAT>` is paged
      10 times and its `orient` p50 written next to the Step 2 budget.
    - A one-line visual check is recorded: the RAF fit view in the native
      window next to the Tauri app's on the same file, whether the one-mip
      downscale shows aliasing (moiré on fine texture); if it does, a
      follow-up item "downscale on the CPU to the drawn size or use a mip
      chain" is added to the Step 5 milestone below. (Not met when the step
      closed, by the user's decision: only the native window was inspected,
      three frames with no visible moiré. The side-by-side with the Tauri
      app is a pending manual check in `learnings.md`.)
    - `docs/humans/performance.md` gains a short "Native preview (GPUI)"
      subsection under "Per-page preview read" with the ARW and RAF rows and
      a pointer to `results.md`, and `performance.ja.md` the same in
      Japanese. `mise run ci` passes.
    - If the threshold is missed, `results.md` names the stage that grew
      and this plan's Step 2 is reopened (a fix PR) before Step 4 starts.
  - Implementation approach:
    - Assumes Steps 1-2 merged. The runs are timer-driven, so the user's
      only action is to keep the machine idle; the agent builds, runs,
      and computes the medians with an ad-hoc script in the scratch
      directory (not committed).
    - Record in `learnings.md` whether the spike's `read` of 0.74 ms is
      reproduced; the Tauri `read` of 6.0 ms on the same files remains
      unexplained and is out of scope here.

The remaining steps are **milestones**. Each is re-planned as its own
`docs/plans/<date>-gpui-<milestone>/plan.md` when its turn comes, and is
ticked here when that plan is archived. Their order is the point of listing
them; their contents are deliberately coarse.

- [x] Step 4: Extract the Tauri-free backend into a shared `crates/backend` library
  - Done when: `riffle-backend` (name to confirm) holds the modules of
    `crates/app/src` that have no `tauri` reference today — `index.rs`
    (SQLite folder index, rusqlite), `sidecar.rs` (the writer thread and
    `SidecarFormat`), `trash.rs` (runs, undo / redo), `shortcuts.rs`,
    `exif.rs` (display formatting), `photolab.rs` — and the near-free ones
    once their `tauri` touches are abstracted behind a trait or a callback
    (`diagnostics.rs`, `output.rs`, `watch.rs`; `perf.rs` and `ai.rs` wait for
    Step 9, by the user's decision, see
    `../_archived/20261109-gpui-backend-crate/plan.md`); `crates/app` depends on it and re-exports, with no
    behavior change (its tests move with the code); the native crate depends
    on it (through `path = "../backend"`, compiled twice like
    `riffle-core`). This is the first milestone that edits `crates/app`, so
    it starts only after the prefetch session's PRs have merged.
- [x] Step 5: The strip, the index and pass 1 of the scan
  - Done when: opening a folder runs `index.rs`'s pass 1 (thumbnails,
    metadata, `EXTRACTOR_VERSION` re-extraction) through the same
    `scan.rs` worker pool and `ReadSlots` (the preview read pausing the
    scan's reads as `commands::preview` does), the strip shows the
    thumbnails with the current file, the preview takes on-screen files
    first (`ScanFocus`), `last_viewed` / `views` are kept, and the folder
    watcher (`watch.rs`) resyncs the list. Includes the Step 3 aliasing
    follow-up if it was raised.
- [x] Step 6: The meta pane and the judgments with their sidecars
  - Done when: the EXIF / Maker note rows (`exif.rs` formatting), rating,
    the tri-state flag and the color label with their keys from
    `shortcuts.rs`, the coalescing sidecar writer (XMP, `.dop`, both; the
    PhotoLab uuid lookup), undo, and view-only JPEG folders.
- [x] Step 7: Filter, sort, focus marks, 1:1 zoom and peaking, pass 2
  - Done when: the filter menu's sections and the sort orders, pass 2
    (`run_faces_scan`: `eye_focus`, sharpness, EAR, pose) with the `f`
    mark, face boxes, the face mesh overlay, the 1:1 crop through
    `partial.rs`, focus peaking (`e`), and the `eyes_of` on-demand path.
  - Pass 2, the `f` overlays (mark, face boxes, face mesh) and `eyes_of`
    were done by `../20261110-gpui-focus-marks/plan.md`.
- [ ] Step 8: Dialogs, folder tree and folder operations
  - Done when: the folder tree with its watchers and volume roots, the
    strip's right-click menu (reveal, copy path / name, Move to Trash…,
    Delete Permanently…), Rename, Rename by Capture Date…, Import…,
    Sequence JPEG Timestamps…, Rewrite / Delete Sidecars…, the scan-idle
    gating (`idle.ts`'s rules), each with its progress events.
- [ ] Step 9: Settings, pass 3 and the companions
  - Done when: the settings modal (sidecar format, label names, shortcuts,
    Import tab, `aiEnabled` with the model download), pass 3 embeddings,
    genres, the personal ranking with its refits, the recommended mark and
    `recommended` sort, moving `perf.rs` and `ai.rs` into `crates/backend`
    behind callback seams, the MCP companion (loopback server and bridge), the
    performance overlay (`perf.rs` sampler), `Riffle.log` with the same
    greppable prefixes (`docs/agents/app-log.md`).
  - The MCP companion, `Handpic.log` and the move of `perf.rs` and `ai.rs`
    into `crates/backend` were done by `../20261113-gpui-companions/plan.md`.
  - The settings modal slice (the native settings store, the persisted
    filter and sort, the Sidecar tab's format and label names, the Culling
    tab's auto-advance, the Keyboard Shortcuts tab and the Import tab) was
    done by `../20261113-gpui-settings/plan.md`. Still open of Step 9's
    settings: the Cache tab, the first-launch format dialog and a macOS app
    menu with `Settings…`. The AI features are not ported (below).
  - Not ported, by the user's decision (2026-11-14): the AI features, that
    is `aiEnabled` with the model download, pass 3 embeddings, genres and
    the filter's `Genre` section, the personal ranking with its refits, the
    recommended mark and the `recommended` sort. The archived evaluations
    put the ranking's within-burst Top-1 at 54.9% against 39.4% for the
    first frame (`../_archived/20261012-image-embeddings/ranking.md`), too
    weak to be worth the port. They drop out of Step 9's done-when; the
    native app keeps reading a stored `recommended` sort as capture order,
    and a settings `aiEnabled` toggle is not needed. What to do with the
    Tauri-side code and `crates/backend/src/ai.rs` at the switch-over is
    left to Step 10.
  - The performance overlay (`d`) was done by
    [the gpui-perf-overlay plan](../20261114-gpui-perf-overlay/plan.md); Step 9
    stays open.
- [ ] Step 10: Packaging, the updater, the three-platform CI and the switch-over
  - Done when: the six sub-milestones below are done, in this order, each
    re-planned as its own plan when its turn comes. The packager
    (`cargo-packager`) and the updater (self-written, keeping the signed
    `latest.json` contract) are decided in
    [`packaging-updater.md`](./packaging-updater.md), with what is still
    unverified.
    1. The `native` CI job on `windows-latest`, `ubuntu-latest` (the apt list
       above) and `macos-latest`, and the packaging / updater decision
       (`../20261114-gpui-ci-packaging/plan.md`).
    2. Package the native app with `cargo-packager`, without the Tauri
       bundler, on a manual `workflow_dispatch` only, uploading workflow
       artifacts: Windows (NSIS, MSI), macOS arm64 and x86_64 (`.app`, DMG),
       Linux (deb, AppImage, rpm per the decision). No release is touched.
    3. The self-update in `crates/backend` replacing `tauri-plugin-updater`,
       with the native app's startup check, menu item and quit-time install,
       reading `latest.json`.
    4. `release.yml` builds, signs and attaches the native installers for
       Windows, macOS (both arches) and Linux and a `latest.json` with
       today's keys, each signature verified against the public key; a
       real-device update from the last Tauri release on each platform before
       the first native release is published.
    5. Merge the native crate into the root workspace: one `Cargo.lock`, the
       root toolchain bumped, the patches moved to the root, one `notify`
       version (deciding the `todo.md` item on whether `configure` still
       queues behind `Unwatch` on 8.2.2), and `crates/app` and
       `crates/app/ui` removed with their Vite+, pnpm and Tauri tooling.
    6. `README.md` / `README.ja.md`, `CONTRIBUTING.md`, `CLAUDE.md`,
       `docs/humans/**` and `docs/agents/**` rewritten for the native app,
       `tauri-app.md` archived.

## Trade-offs and risks

- **Separate workspace (taken, by the user's decision) vs non-default
  workspace member.** Premises: `async-task 4.7.1` and `async-process 2.5.0`
  are already in the root lock through Tauri's Linux tree, so one workspace
  would replace them with Zed's forks in the shipped Tauri app; its toolchain
  bump to 1.98.1 would apply to `crates/app`, whose clippy may gain lints that
  cannot be fixed while that crate is frozen; its lock churn is about 170
  compatible version drifts plus duplicate `windows 0.57`, `objc2 0.5`,
  `rand 0.8/0.9`, `quick-xml 0.30`. Costs of the separate workspace:
  `riffle-core` (and later `riffle-backend`) compile twice, two `Cargo.lock`s
  to bump, `mise run ci` only covers the native crate through the explicit
  `native:*` tasks and a dedicated CI job, and a `cargo` run from the repo root
  never sees the native crate. The merge is deferred to Step 10, when
  `crates/app` is removed and nothing conflicts.
- **Sharing `crates/app` logic without editing it.** `riffle-app` has only
  `main.rs` (no `lib.rs`), so no crate can depend on it. The slice uses
  `riffle_core` alone until Step 4 extracts a library.
  `#[path = "../../app/src/index.rs"] mod index;` includes were rejected: they
  compile the same file in two crates where `crate::` paths and `tauri::`
  imports break as soon as a module has any, and a change in `crates/app` can
  break the native build without a diff there. Step 4 names the modules by
  their current `tauri` reference count so the extraction order is known.
- **Orientation on the CPU.** GPUI cannot rotate an image sprite (no
  `transformation` on `PolychromeSprite`); rotating 7 MB of BGRA
  (1616x1080) or 48 MB (RAF) on the background thread costs time only on
  portrait shots and is budgeted (2 ms / 15 ms) and measured in Steps 2-3.
  The alternative, a shader-level rotation, means a GPUI fork and is out of
  scope; if the RAF budget is missed the fallback is to rotate after a
  DCT-scaled decode to the drawn size (`decode_upright_near` already does
  this for RGB), which Step 5 may want anyway for the aliasing.
- **Regression threshold.** p50 + 2.0 ms (ARW) / + 5.0 ms (RAF), median of
  three runs of n=20. Reasoning: the spike's p50 sits near the 60 Hz vsync
  half-period (6-7 ms of the 15.56 is vsync wait), so a 2 ms budget is one
  eighth of a frame and about the size of the added `orient` stage plus
  logging; RAF's decode alone varies by more than 2 ms run to run, hence
  5 ms. p95 at n=20 is one sample and is not gated.
- **The Zed git dependency.** Cargo clones the whole Zed repository
  (hundreds of MB, ~900 crates in its workspace) for 9 path crates. First
  local and cold-CI builds take minutes; the `~/.cargo/git/db` cache keyed
  on `crates/native/Cargo.lock` removes that from warm runs. Vendoring the
  gpui crates into the repo was rejected (unmaintainable, licensing
  bookkeeping); a Zed fork to pin is unnecessary while a rev is pinned.
- **GPUI is pre-1.0 and pinned to one rev.** Bumping the rev is a
  deliberate chore PR whenever it is needed (a fix, a feature), never a
  side effect; `docs/agents/native-app.md` records API changes met at each
  bump.
- **Mip levels and aliasing.** No moiré in three native-only frames (Step 3);
  the side-by-side with the Tauri app is pending, and Step 5 fixes it if
  needed. **IME on Windows** matters only from Step 6 (rename, label
  names); unverified until then. **No after-Present hook**: the
  `cx.defer`-from-paint timing is a workaround that may change with a GPUI
  bump; the timing test in `timing.rs` pins the arithmetic, not the hook.
- **NASM as a hard failure.** A developer without NASM on x86 cannot build
  the native crate until they install it or set
  `RIFFLE_NATIVE_ALLOW_NO_SIMD=1`; that is the point (a 2x slower decode
  must not pass unnoticed), but CI runners need it too (Step 1 checks the
  Windows image; the Linux and macOS jobs of Step 10 add `nasm` to their
  installs, macOS arm64 needs none).
- **Two apps for a long time.** Every judgment feature lands twice until
  Step 10; the plan accepts that by keeping the native app out of releases
  and docs for users until parity, and by letting Step 4's library carry
  behavior changes to both from then on.
- **Conflicts with in-flight work.** The prefetch session edits
  `crates/app` (`main.ts`, `commands.rs`); this plan's Steps 1-3 add files
  only and touch the root `Cargo.toml` (one `exclude` line), `mise.toml`,
  `ci.yml`, `CONTRIBUTING.md`, `CLAUDE.md` and the performance docs. Step 4
  waits for that session to merge.

## Spike reference

The throwaway spike this plan starts from lived in a scratch directory and
is gone; what Step 2 needs is reproduced here. Its `Cargo.toml` used path
dependencies into a Zed checkout; Step 1 replaces them with the git rev.

```toml
[package]
name = "gpui-spike"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
gpui = { path = "../zed/crates/gpui" }
gpui_platform = { path = "../zed/crates/gpui_platform" }
riffle-core = { path = "../../crates/core" }
mozjpeg = "0.10"
image = { version = "0.25", default-features = false }

[patch.crates-io]
async-process = { git = "https://github.com/zed-industries/async-process.git", rev = "0b6d6713570af61806e1e5cb40e0f757cb93fd9d" }
async-task = { git = "https://github.com/zed-industries/async-task.git", rev = "015845a19fc31752c02367567d3b8900a406f9da" }
```

The load, on `cx.background_spawn`:

```rust
use gpui::RenderImage;
use image::{Frame, RgbaImage};
use riffle_core::reader;

fn load(path: &Path, index: usize) -> Result<Loaded, String> {
    let started = Instant::now();
    let (_, jpeg) = reader::read_preview(path).map_err(|e| e.to_string())?;
    let read = started.elapsed();
    let started = Instant::now();
    let mut d = mozjpeg::Decompress::new_mem(&jpeg)
        .map_err(|e| e.to_string())?
        .to_colorspace(mozjpeg::ColorSpace::JCS_EXT_BGRA)
        .map_err(|e| e.to_string())?;
    let (w, h) = (d.width() as u32, d.height() as u32);
    let bgra = d.read_scanlines::<u8>().map_err(|e| e.to_string())?;
    d.finish().map_err(|e| e.to_string())?;
    let buffer = RgbaImage::from_raw(w, h, bgra).ok_or("short decode")?;
    let image = Arc::new(RenderImage::new(vec![Frame::new(buffer)]));
    Ok(Loaded { index, image, read, decode: started.elapsed(), finished: Instant::now(), jpeg_kb: jpeg.len() / 1024, px: (w, h) })
}
```

One request in flight, a stale result re-requests (`pager.rs` in Step 2):

```rust
fn request(&mut self, cx: &mut Context<Self>) {
    if self.in_flight { return; }
    self.in_flight = true;
    self.sent_at = Instant::now();
    let (index, path) = (self.index, self.files[self.index].clone());
    cx.spawn(async move |this, cx| {
        let loaded = cx.background_spawn(async move { load(&path, index) }).await;
        let received = Instant::now();
        this.update(cx, |this, cx| this.loaded(loaded, received, cx)).ok();
    })
    .detach();
}

fn loaded(&mut self, result: Result<Loaded, String>, received: Instant, cx: &mut Context<Self>) {
    self.in_flight = false;
    let loaded = match result { Ok(l) => l, Err(e) => { println!("error {e}"); return; } };
    if loaded.index != self.index { self.request(cx); return; }   // stale: re-request the current file
    let old = self.image.replace(loaded.image);
    // keep `old` in `pending` so it is dropped after the new frame presents;
    // when no key timed this switch, `cx.drop_image(old, None)` right away
    cx.notify();
}
```

The paint and the after-present stamp (`canvas` child of a `div().size_full().track_focus(&self.focus).on_key_down(...)`):

```rust
canvas(|_, _, _| {}, move |bounds: Bounds<Pixels>, _, window, cx| {
    let Some(image) = image else { return };
    let timing = pending.borrow_mut().take();
    let paint_started = Instant::now();
    let px_size = image.size(0);
    let (iw, ih) = (px_size.width.0 as f32, px_size.height.0 as f32);
    let scale = (f32::from(bounds.size.width) / iw).min(f32::from(bounds.size.height) / ih);
    let fit = size(px(iw * scale), px(ih * scale));
    let origin = bounds.origin + point((bounds.size.width - fit.width) / 2., (bounds.size.height - fit.height) / 2.);
    let fit = Bounds::new(origin, fit);
    window.paint_image(fit, fit, Corners::default(), image, 0, false).expect("paint_image");
    let paint_finished = Instant::now();
    if let Some(t) = timing {
        // `defer` runs when the update holding draw() and present() ends: the after-Present stamp.
        cx.defer(move |cx| {
            let end = Instant::now();
            // wait = sent - key; hop = received - bg_finished; frameWait = paint_started - notified;
            // paint = paint_finished - paint_started; present = end - paint_finished;
            // keypressToPixels = end - key
            if let Some(old) = t.old { cx.drop_image(old, None); }
        });
    }
})
.size_full()
```

Keys came from `KeyDownEvent.keystroke.key` (`"right"`, `"left"`, `"q"`);
the window was opened with `gpui_platform::application().run(|cx: &mut App| { cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1600.), px(1000.)), cx))), ..Default::default() }, |window, cx| cx.new(|cx| { let focus = cx.focus_handle(); window.focus(&focus, cx); ... })) })`
followed by `cx.activate(true)`. The `--auto N` driver was a
`cx.spawn` loop: a 2 s settle, then `view.update(cx, |this, cx| this.advance(1, cx))`
every 1003 ms through `cx.background_executor().timer(..)`, printing the
summary and `cx.quit()` when N presses are done.

## Progress

- (2026-11-09) Step 1 complete
- (2026-11-09) Step 2 complete
- (2026-11-09) Step 3 complete
- (2026-11-09) Step 4 complete, through `../_archived/20261109-gpui-backend-crate/plan.md`
- (2026-11-10) Step 5 complete, through `../_archived/20261110-gpui-strip-index/plan.md`
- (2026-11-13) Step 6 complete, through `../_archived/20261110-gpui-judgments/plan.md`
- (2026-11-12) Step 7's 1:1 zoom and focus peaking slice done, through `../_archived/20261110-gpui-zoom-peaking/plan.md` (Step 7 stays open)
- (2026-11-13) Step 7: the filter and the sort, through `../_archived/20261113-gpui-filter-sort/plan.md`; the per-folder EXIF filter sections remain
- (2026-11-13) Step 7 complete: the per-folder EXIF filter sections and two refilter fixes, through `../_archived/20261113-gpui-exif-filter/plan.md`
- (2026-11-14) Step 9: the MCP companion, `Handpic.log` and the backend's perf and AI modules, through `../20261113-gpui-companions/plan.md` (Step 9 stays open)
- (2026-11-14) Step 9: the AI features (pass 3, genres, the ranking, the recommended mark and sort, `aiEnabled`) dropped from the port by the user's decision
- (2026-11-14) Step 8: the folder tree, the strip's selection and its right-click menu, through `../_archived/20261113-gpui-tree-trash/plan.md`; the folder operation dialogs remain
- (2026-11-14) Step 9: the settings modal (the native store, the persisted filter and sort, the Sidecar, Culling, Keyboard Shortcuts and Import tabs), through `../20261113-gpui-settings/plan.md`; the Cache tab, the first-launch format dialog and a macOS `Settings…` menu item remain, and the AI features are not ported by the user's decision (Step 9 stays open)
- (2026-11-14) Step 10, sub-milestone 1: the `native` CI job on three platforms and the packaging / updater decision, through `../20261114-gpui-ci-packaging/plan.md` (Step 10 stays open)
- (2026-11-14) Step 9: the performance overlay (`d`), through [the gpui-perf-overlay plan](../20261114-gpui-perf-overlay/plan.md) (Step 9 stays open)
