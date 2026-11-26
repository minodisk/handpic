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

# Pillow notice for the PIL resampling port

## Purpose

`crates/core/src/resample.rs` is a port of PIL's bilinear and bicubic
resampling. The dual-license work
([plan](../_archived/20261124-dual-license/plan.md)) left open whether that
port needs a Pillow (MIT-CMU) notice, and `todo.md` still carries the
question. The user has decided it: a port gets a notice, the conservative
reading, as with the themes removed for having no determined license. This
work ships Pillow's license text with every package, names it in the
documentation that lists the third-party notices, and closes the todo item.

## Steps

- [x] Step 1: Ship Pillow's license notice for the resampling port and close
      the todo item
  - Done when:
    - `crates/core/LICENSE-pillow` exists and holds the text of Pillow's
      upstream `LICENSE`
      (`https://github.com/python-pillow/Pillow/blob/main/LICENSE`)
      unchanged.
    - `tools/package/notices.sh` copies it as `LICENSE-pillow`:
      `bash tools/package/notices.sh <temp dir>` leaves a `LICENSE-pillow`
      there identical to the source file, and the script stays
      shellcheck-clean.
    - The License sections of `README.md` and `README.ja.md` each list the
      port with its license (MIT-CMU) and a link to the notice, and the two
      stay in sync.
    - `docs/agents/app.md`'s section "The license notices are due at
      packaging" names `LICENSE-pillow` in its `notices.sh` list and records
      the decision (the port is treated as derived from Pillow, so it ships
      the notice).
    - The `todo.md` item "Core: check whether the PIL resampling port needs
      a Pillow notice" is gone, heading and body.
    - `crates/core/src/resample.rs` is unchanged.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Fetch the upstream `LICENSE` at implementation time (the raw file is
      `https://raw.githubusercontent.com/python-pillow/Pillow/main/LICENSE`)
      and keep its text as it is, the non-ASCII copyright signs included,
      UTF-8 with LF line endings. Do not retype it from memory.
    - Place the file at the crate root, `crates/core/LICENSE-pillow`, as
      `crates/app/LICENSE-lucide` is placed for the app crate.
      `crates/core/models/` holds the notices of the model files stored
      there (`LICENSE`, `LICENSE-mediapipe`); this notice covers source
      code, not a model.
    - `crates/core/models/LICENSE-mediapipe` and `crates/app/themes/LICENSE`
      open with a few lines saying what in the repository the notice covers
      before the license text. Follow that: a short header naming
      `crates/core/src/resample.rs` as a port of Pillow's resampling,
      then the upstream text. State only what can be verified: the header
      comment of `resample.rs` names no Pillow source file, version or
      commit, so do not write one unless it is checked against upstream
      during implementation.
    - In `tools/package/notices.sh`, add one
      `"crates/core/LICENSE-pillow:LICENSE-pillow"` entry to the `notices`
      array, with the other `crates/core` entries. Every package
      (`macos.sh`, `windows.sh`, `linux.sh`, the `app:bundle:macos` task)
      takes its notices from this script, so nothing else in the packaging
      changes. `.github/workflows/package.yml` asserts only `NOTICE-gpui`
      per package and needs no change; the script runs under `errexit`, so
      a missing source file already fails the build.
    - In both READMEs, add one bullet to the "third-party material" list in
      the License section, in the form of the existing bullets (what it is,
      the license's short name, a link to the file): `README.md` around
      line 464, `README.ja.md` around line 200, the Japanese body in
      Japanese. Use `MIT-CMU`, the name upstream gives the license.
    - In `docs/agents/app.md`, extend the paragraph starting
      "`tools/package/notices.sh <dir>` copies the set" (around line 346)
      with `crates/core/LICENSE-pillow`, and record the decision in the same
      section. The section's existing rule ("a new bundled asset under
      another license adds its notice there") speaks of assets; say that a
      port of third-party code counts too.
    - Other files that mention `LICENSE-mediapipe` (`docs/humans/usage.md`,
      `docs/humans/usage.ja.md`, `docs/agents/tract-onnx-inference.md`) cite
      it as model provenance and do not enumerate the shipped notices; they
      need no change. `CLAUDE.md` has no list of notices either and is left
      alone.
    - Delete the `todo.md` item whole: from its `###` heading through its
      `#### TODO` checkbox, leaving the neighbouring items and the blank
      line between them intact.
    - Commit as Conventional Commits, in English, for example
      `docs(core): ship Pillow's license notice for the resampling port`.

## Trade-offs and risks

- Where the notice lives. `crates/core/LICENSE-pillow` follows
  `crates/app/LICENSE-lucide`, a notice at the root of the crate whose
  source it covers, rather than `crates/core/models/`, which otherwise holds
  only the ONNX models and their licenses. The user approved the crate root.
- A header before the license text. The header says what the notice covers,
  as `LICENSE-mediapipe` and the themes' `LICENSE` do; the license text after
  it is unchanged. A header must not name a Pillow version or commit that was
  not verified.
- `CLAUDE.md` and `resample.rs` are left alone: `CLAUDE.md` lists no notices,
  and the source file is out of scope.

## Progress
