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

# Dual license: MIT OR Apache-2.0

## Purpose

The repository is public but carries no license of its own, so nobody may
legally use, modify or redistribute Handpic, and GitHub shows no license on
the repository page (`gh api repos/minodisk/handpic --jq .license` is `null`).
This work licenses Handpic under `MIT OR Apache-2.0`, the Rust ecosystem's
convention: the license texts at the repository root, the SPDX expression in
every crate's manifest, a License section in both READMEs that also says the
bundled third-party material keeps its own license, and Handpic's own license
texts in the `licenses/` folder every package ships.

The dependency licenses were audited beforehand through `cargo metadata`: no
dependency requires GPL (`r-efi` and `self_cell` are dual MIT / Apache; the
MPL-2.0 `dwrote`, `option-ext` and `cbindgen` are used unmodified), so there
is no compatibility issue.

## Steps

- [x] Step 1: License the repository under `MIT OR Apache-2.0` (license
      files, crate manifests, READMEs, shipped notices)
  - Done when:
    - `LICENSE-MIT` and `LICENSE-APACHE` exist at the repository root.
      `LICENSE-MIT` is the standard MIT text with
      `Copyright (c) 2026 minodisk`; `LICENSE-APACHE` is the Apache License
      2.0 text.
    - `cargo metadata --format-version 1 --no-deps` at the root reports
      `"license": "MIT OR Apache-2.0"` for `handpic-core`, `handpic-engine`
      and `handpic-cli`, and
      `cargo metadata --format-version 1 --manifest-path crates/app/Cargo.toml`
      reports it for `handpic-app` and for the `handpic-core` and
      `handpic-engine` it takes by path (this shows the inherited key
      resolves when the crates are built from the app's workspace).
    - `README.md` and `README.ja.md` each end with a License section with
      the same content (the Japanese one in Japanese): Handpic is licensed
      under either of MIT or Apache-2.0 at your option, with links to
      `./LICENSE-MIT` and `./LICENSE-APACHE`, and the bundled third-party
      material follows its own license, naming each file listed in the
      implementation approach.
    - `bash tools/package/notices.sh <an empty temporary folder>` leaves
      `LICENSE-MIT` and `LICENSE-APACHE` in it next to the five existing
      notices.
    - `docs/agents/app.md`'s "The license notices are due at packaging"
      section no longer says the repository has no project license file,
      and lists the two files in the shipped set; `CLAUDE.md` mentions the
      root license files where it describes `notices.sh`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - After the merge to `main`:
      `gh api repos/minodisk/handpic --jq .license` is no longer `null` and
      its `spdx_id` is not `NOASSERTION` (expected: `Apache-2.0`, see the
      implementation approach), and the repository page's sidebar lists
      both licenses. GitHub's detection can lag a push by some minutes;
      re-check before concluding it failed.
  - Implementation approach:
    - License files:
      - Use the names `LICENSE-MIT` and `LICENSE-APACHE` exactly, at the
        root, and add no plain `LICENSE` file. Measured at planning time
        (2026-11-24): public repositories with exactly this pair at the
        root (`serde-rs/serde`, `dtolnay/anyhow`, `rust-lang/cargo`) answer
        `gh api repos/<repo> --jq .license.spdx_id` with `Apache-2.0`, not
        `NOASSERTION`. The API's `license` field holds one license, so
        `Apache-2.0` is the expected answer; the repository page shows
        both.
      - Take the Apache text from the canonical
        `https://www.apache.org/licenses/LICENSE-2.0.txt`, unmodified. Do
        not copy it out of `crates/app/packaging/NOTICE-gpui`, which puts a
        preamble before it. Leave the appendix's
        `Copyright [yyyy] [name of copyright owner]` placeholder as is
        (decided with the user, 2026-11-24).
      - `.gitattributes` already normalizes text to LF; nothing to add.
    - Manifests:
      - Add `[workspace.package]` with `license = "MIT OR Apache-2.0"` to
        the root `Cargo.toml` and `license.workspace = true` to the
        `[package]` of `crates/core`, `crates/engine` and `crates/cli`.
      - `crates/app/Cargo.toml` is its own workspace root (an empty
        `[workspace]`) and is excluded from the root one, so it cannot
        inherit from the root: give it the literal
        `license = "MIT OR Apache-2.0"` in `[package]`.
      - Cargo resolves a path dependency's `*.workspace = true` against
        that dependency's own workspace root (the repository root
        manifest), so `handpic-core` and `handpic-engine` keep working when
        built from `crates/app`. Verify this with the `cargo metadata`
        call in "Done when" rather than assuming it. If it does not hold,
        fall back to the literal key in all four manifests.
      - Do not touch `version` or anything else in the manifests.
        `release-please-config.json` updates only `$.package.version` in
        these files and `tools/package/version.sh` reads only
        `[package].version`; neither is affected. `Cargo.lock` files do not
        record a license and should not change.
    - READMEs:
      - Add the section as the last one, after `## For developers` in
        `README.md` and after `## 開発者向け` in `README.ja.md`. Change
        nothing else in either file. No contribution clause (not
        requested).
      - Third-party files to name (every bundled license / notice file in
        the repository, by `git ls-files`):
        `crates/core/models/LICENSE` (the YuNet model, MIT),
        `crates/core/models/LICENSE-mediapipe` (the MediaPipe Face
        Landmarker model and the ported face geometry data, Apache-2.0),
        `crates/app/themes/LICENSE` (the vendored gpui-kit themes,
        Apache-2.0), `crates/app/packaging/NOTICE-gpui` (the GPUI crates,
        Apache-2.0) and `crates/app/LICENSE-lucide` (the Lucide icons and
        the two derived ones, ISC).
      - Write the links relative (`./LICENSE-MIT`, ...); lychee checks them
        offline in `mise run lint`.
    - Shipped notices:
      - Add `"LICENSE-MIT:LICENSE-MIT"` and
        `"LICENSE-APACHE:LICENSE-APACHE"` to the `notices` array of
        `tools/package/notices.sh`. No name collides with the existing
        entries (`NOTICE-gpui`, `LICENSE-lucide`, `LICENSE-themes`,
        `LICENSE-yunet`, `LICENSE-mediapipe`).
      - Nothing else in packaging needs a change; every consumer takes the
        whole folder from `notices.sh`:
        - `mise.toml`'s `app:bundle:macos` copies it into
          `Contents/Resources/licenses`, which `tools/package/macos.sh`
          packages.
        - `tools/package/windows.sh` fills `Handpic/licenses` for the
          portable ZIP and passes it as `LicensesDir` to
          `crates/app/packaging/windows/handpic.iss`, whose `[Files]` line
          takes `{#LicensesDir}\*`.
        - `tools/package/linux.sh` fills the tarball's `licenses/` and the
          deb's `/usr/share/doc/handpic`.
      - `.github/workflows/package.yml` checks the folder only through
        `NOTICE-gpui`'s presence; leave those checks alone. `Package` does
        not run on a PR, so the local `notices.sh` run in "Done when" is
        the check of this part.
    - Docs:
      - In `docs/agents/app.md`, replace the sentence "The repository has
        no project `LICENSE` file, so none is shipped." with the two root
        files being part of the shipped set.
      - Keep the `CLAUDE.md` edit to a clause.
    - Commit / PR title: `chore: license Handpic under MIT OR Apache-2.0`.
      A `feat` or `fix` type would make release-please propose a release
      for this alone.

## Trade-offs and risks

- **Apache appendix copyright line.** The copyright holder and year go in
  `LICENSE-MIT` only; `LICENSE-APACHE` stays the unmodified canonical text,
  as in `serde`, `anyhow` and `cargo`, which keeps licensee's match exact.
- **The API reports one license.** `gh api ... --jq .license` will say
  `Apache-2.0`, not a dual expression; the dual license is visible in the
  page sidebar, the READMEs and the manifests.
- **Existing installs keep the old `licenses/` folder.** The portable ZIP
  and tarball updaters replace only the executable, so those installs gain
  the two files only on a fresh download. Inno Setup, deb and `.app`
  updates replace the folder. Not addressed here.
- **Ported code without a license file.** `crates/core/src/resample.rs` is
  a reimplementation of PIL's resampling filters and
  `crates/core/src/sequence.rs` derives from `minodisk/lapse` (MIT, the same
  author). Whether the PIL port needs a Pillow (MIT-CMU) notice was not
  assessed; out of scope here.

## Progress

- 2026-11-24: Step 1 done. Handpic is licensed under MIT OR Apache-2.0
  (license files and manifests updated in commit `30324346`). Not yet
  verified: `mise run ci:local` was stopped for host memory pressure and is
  left to the PR's GitHub Actions, and the post-merge `gh api ... .license`
  check is still pending (both recorded in `learnings.md`).
