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

# Install rust-analyzer through mise

## Purpose

Claude Code's rust-analyzer LSP plugin failed to start with "its program
rust-analyzer was not found" because the rust-analyzer rustup component was
not installed. The Rust toolchains of this repository come from `mise install`
(the root `mise.toml` for the root workspace, `crates/native/mise.toml` for
the GPUI app's separate workspace), each listing its rustup components. Adding
`rust-analyzer` to both lists makes `mise install` install it for both
toolchains, so every environment set up through mise has it.

## Steps

- [x] Step 1: Add `rust-analyzer` to the rust components in both mise files
  - Done when:
    - The `[tools]` entry in the root `mise.toml` reads
      `rust = { version = "1.97", components = "rustfmt,clippy,rust-analyzer" }`.
    - The `[tools]` entry in `crates/native/mise.toml` reads
      `rust = { version = "1.98.1", components = "rustfmt,clippy,rust-analyzer" }`.
    - After `mise install` at the repository root,
      `rustup component list --installed --toolchain <resolved root version>`
      includes `rust-analyzer`; after `mise install` in `crates/native`, the
      same holds for `1.98.1`. Record both outputs in the PR description.
    - Nothing else changes: no PATH or environment settings, no other tool,
      no documentation.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - The files that change are `mise.toml` (line 6) and
      `crates/native/mise.toml` (line 2), one line each. Keep the existing
      form, a comma-separated string with no spaces.
    - The root file pins `1.97`, which mise resolves to a full version
      (`mise current rust` printed `1.97.1` at planning time). Pass that
      resolved version to `rustup component list --toolchain`, not `1.97`:
      rustup treats `1.97` as a separate toolchain name and auto-installs it
      when it is missing.
    - On the planning host both toolchains (`1.97.1` and `1.98.1`) already
      had the rust-analyzer component, so the check passes there before the
      change too. If mise reports the tools as already installed and a host
      without the component does not get it, measure whether
      `mise install --force rust` is needed and note the result in
      `learnings.md`.
    - No documentation lists the rust components (`README.md`,
      `CONTRIBUTING.md`, `CLAUDE.md` and `docs/agents/**` were searched), so
      none is updated. The line in `docs/plans/20261109-gpui-port/plan.md`
      that quotes the native pin is a record of that plan and stays.
    - For the root `mise.toml` `ci:local` selects the formatter checks,
      `vp check` (which formats TOML), shellcheck, actionlint, lychee and the
      bash tests. This was superseded: a change to `crates/native/mise.toml`
      also selects the native crate's checks (clippy and 867 tests, several
      minutes); see `learnings.md`.

## Trade-offs and risks

- **The per-task pins of the native toolchain are left as they are** (the
  user chose this). The root `mise.toml` pins the native toolchain five more
  times, as `tools = { rust = { version = "1.98.1", components = "rustfmt,clippy" } }`
  on the `native:*` tasks. The request names only the two `[tools]` entries,
  and those are what `mise install` reads, so this plan does not touch the
  task lines.
- **CI downloads one more component.** The `lint`, `test`, `native` and
  release jobs run `jdx/mise-action@v2`, which runs `mise install`, so each
  job also downloads rust-analyzer, which CI never uses. The cost is a small
  download per job.
- **This may not be what stops the plugin on every host.** On the planning
  host both toolchains already had the component, so a "not found" there
  would come from the PATH of the process that starts the plugin. PATH
  changes are out of scope for this plan.

## Progress

- 2026-11-15: Step 1 done. Added `rust-analyzer` to the rust `components` in
  `mise.toml` and `crates/native/mise.toml`. Both toolchains (`1.97.1`,
  `1.98.1`) list the component; outputs are in `learnings.md`. Deferred: this
  host already had the component, so whether a plain `mise install` adds it to
  an already installed toolchain or `mise install --force rust` is needed was
  not measured (recorded under `## Deferred issues (todo candidates)` in
  `learnings.md`).
