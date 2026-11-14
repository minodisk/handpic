# Learnings: Light Local CI

## Step 1

### Measurements (Windows host, 24 logical cores, 31.8 GB, other sessions building)

Method: a PowerShell sampler started the command, polled every 500 ms and
summed the working sets of the command's whole process tree (rustc, clippy,
the linker, the test binaries); "system" is total minus free physical memory
at the same polls, so it includes the other sessions' builds. Wall time is the
whole command. The per-command rows ran in a throwaway worktree at
`origin/main` with a fresh `CARGO_TARGET_DIR` per package (cold), then again
after a one-line change to the same crate (warm). The `ci:local` rows ran
`tools/ci/local.sh` there after removing `target/` and
`crates/native/target`, so every one of them also ran the shellcheck and
`local.test.sh` of the untracked `tools/ci/` (a few seconds).

| Command | Cold wall | Cold tree peak | Warm wall | Warm tree peak |
| --- | --- | --- | --- | --- |
| `cargo clippy -p riffle-core --all-targets` | 29.6 s | 1.67 GB | 2.1 s | 0.33 GB |
| `cargo test -p riffle-core` | 117.9 s | 3.35 GB | 76.3 s | 4.26 GB |
| `cargo clippy -p riffle-backend --all-targets` | 44.6 s | 2.49 GB | 3.2 s | 0.51 GB |
| `cargo test -p riffle-backend` | 98.9 s | 3.26 GB | 27.0 s | 2.89 GB |
| `cargo clippy -p riffle-cli --all-targets` | 34.4 s | 1.81 GB | 1.2 s | 0.11 GB |
| `cargo test -p riffle-cli` | 65.5 s | 3.39 GB | 3.0 s | 1.78 GB |
| native `cargo clippy --all-targets` | 53.3 s | 3.41 GB | 3.6 s | 0.46 GB |
| native `cargo test` | 78.7 s | 5.50 GB | 16.9 s | 4.70 GB |

`riffle-core`'s tests themselves run about 68 s (409 tests), which is why its
warm test is still over a minute.

| `ci:local` for a change in | Cold wall | Tree peak | System peak | Warm wall | Tree peak |
| --- | --- | --- | --- | --- | --- |
| Markdown only (`docs/agents/app-log.md`) | 9.4 s | 0.06 GB | 17.6 GB | | |
| frontend only (`crates/app/ui/src/sort.ts`) | 7.9 s | 0.41 GB | 18.0 GB | | |
| `crates/core` (fmt, core clippy + test, native clippy) | 264.7 s | 3.32 GB | 28.0 GB | 156.5 s | 4.99 GB |
| `crates/backend` (fmt, backend clippy + test, native clippy) | 199.3 s | 3.35 GB | 25.9 GB | 36.3 s | 2.94 GB |
| `crates/cli` (fmt, cli clippy + test) | 66.2 s | 3.09 GB | 22.6 GB | 7.5 s | 1.84 GB |
| `crates/native` (fmt, native clippy + test) | 153.6 s | 5.55 GB | 25.1 GB | 16.9 s | 4.18 GB |

Every check at once (this branch, whose `mise.toml` change selects them all,
cold `target/` and `crates/native/target`, the three root packages then still
one `cargo` call each): 593.6 s, a 5.59 GB tree peak, the system at 26.3 GB.
That is at the Bash tool's 600 s limit, so the script now hands the selected
root packages to one `cargo clippy` and one `cargo test` call (`--package` per
package), which builds them in one parallel graph; the selection and its test
are unchanged. The same cold run after that change: 382.4 s, a 5.34 GB tree
peak, the system at 23.8 GB.

The Markdown-only run is mostly `lychee` (about 4 s over the whole repository
here, not the 0.2 s the plan quotes) and `local.test.sh` (38 cases, each a
`bash` start).

- `cargo tree -p riffle-core / riffle-backend / riffle-cli -e normal,build,dev`
  lists no `tauri`, `wry` or `riffle-app` (161, 317 and 169 crates against
  537 for `riffle-app`), so `cargo clippy -p` / `cargo test -p` of a root
  package never compiles the Tauri crate.
- The native test stays in `ci:local`. Criterion: the largest realistic set
  for a `crates/native` change (fmt, the native clippy and test, cold
  `crates/native/target`) finishes inside the Bash tool's 600 s limit without
  the host running out of memory. Measured 153.6 s and a 5.55 GB tree peak,
  the system at 25.1 GB of 31.8 GB.
- No `--jobs` cap. The largest tree peak of any selected check was 5.5 GB (the
  native test) and the heaviest ordinary case (a `crates/core` change, cold)
  peaked at 3.3 GB; the system peaks of 28–29 GB were the other sessions'
  builds plus ours. The earlier out-of-memory kills were the full root
  workspace with `riffle-app`'s 537-crate graph and its test binaries, which
  `ci:local` never builds.

### What tripped me up

- `node`, `pnpm` and `cargo` are not on the plain Git Bash `PATH` here; only
  `mise exec --` or a mise task finds them. `tools/ci/local.sh` runs inside
  the `ci:local` task, so it calls them directly.
- `vp fmt` refuses a directory outside a pnpm workspace ("Package not found in
  workspace"), so the extension list was probed with oxfmt's own binary
  (`node_modules/.pnpm/oxfmt@0.68.0_*/node_modules/oxfmt/bin/oxfmt
  --list-different`) over one unformatted file per extension: it formats
  `.js .jsx .mjs .cjs .ts .tsx .mts .cts .json .jsonc .json5 .yaml .yml .toml
  .html .htm .vue .css .scss .less .graphql .gql .hbs .handlebars .md .mdx
  .markdown`, plus dotfiles such as `.prettierrc` and `.babelrc`, and not
  `.svelte`, `.astro`, `.xml`, `.svg` or `Cargo.lock`. Its bundled linguist
  table lists many rarer extensions too; the script keeps the main ones per
  language, which over-triggers nothing and could only miss an exotic
  extension the repository does not use.
- NASM was on the Git Bash `PATH` here, and the native build found it through
  `mise exec rust@1.98.1 -- cargo` inside a `bash` started by the sampler. The
  script still adds `%LOCALAPPDATA%\bin\NASM` when `nasm` is missing and fails
  with a message naming NASM when `mise exec rust@1.98.1 -- nasm -v` fails.
- The Bash tool moved a foreground command to the background after 180 s
  even with `timeout: 600000` (the cold `ci:local` runs). The agents'
  "do not set `run_in_background`" instructions still hold, but a cold Rust
  `ci:local` can end up observed through a task notification anyway.
- `origin/main` moved (to f942b4dd) while this branch was at cb674451; the
  merge base stayed cb674451, so the selection saw only this branch's paths.

## Deferred issues (todo candidates)

### Tooling: format JSON and YAML without node once the Tauri app is gone

#### Background

`mise run lint` and `mise run ci:local` (`tools/ci/local.sh`) run `vp check`,
which formats JSON, YAML and TOML through oxfmt from `vite-plus`, so both need
node and `pnpm install` even for a change that touches no frontend code (a
plan's `.json`, a workflow `.yml`, a `Cargo.toml`). Once the Tauri app and
`crates/app/ui` are removed, the only reason left for node in those checks is
that formatting. A standalone Rust formatter installed through mise, such as
dprint (its JSON, YAML and TOML plugins), would do the same without node.
Plan: `docs/plans/20261114-light-local-ci/plan.md`
Files: `mise.toml` (`[tasks.lint]`, `[tasks.fmt]`), `vite.config.ts`
(`fmt.ignorePatterns`), `tools/ci/local.sh` (`vp_extensions`, the
`vp-check` check), `.github/workflows/ci.yml` (the `lint` job's node setup).

#### TODO

- [ ] When the Tauri app and `crates/app/ui` are removed, move the JSON / YAML
      (and TOML) formatting from vp (oxfmt through pnpm) to a standalone
      formatter installed through mise, such as dprint, carrying over
      `vite.config.ts`'s ignore patterns, and drop node and pnpm from
      `[tasks.lint]`, `[tasks.fmt]` and `tools/ci/local.sh`. Done when
      `mise run lint` and `mise run ci:local` run with no node installed.

### Agents: a cold Rust `ci:local` is moved to the background by the Bash tool after 180 s

#### Background

During Step 1 the Bash tool moved a foreground command to the background after
about 180 s even with `timeout: 600000`, and the cold Rust `ci:local` runs hit
this. `.claude/agents/implementer.md` (step 7) and
`.claude/agents/local-review-addresser.md` tell the agent to run the check in
the foreground so the exit code is observable, and say nothing about the run
being backgrounded anyway.
Plan: `docs/plans/_archived/20261114-light-local-ci/plan.md`

#### TODO

- [ ] Reproduce the behavior once with a cold `ci:local`. If it holds, add to
      both agent definitions that an exit code arriving as a task notification
      is acceptable and is waited for, not retried, and that the rule is only
      about not choosing `run_in_background` yourself. Done when both
      definitions say what to do when the Bash tool backgrounds the run.
