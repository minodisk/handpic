# Learnings

## Step 1

- `mise install` at the root and in `crates/native` both reported every tool
  as already installed (`rust@1.97 ... already installed`), so the changed
  `components` string did not trigger a reinstall. On this host the
  rust-analyzer component was already present for both toolchains before the
  change, so this host cannot show whether a host without it gets it from a
  plain `mise install` or needs `mise install --force rust`; that was not
  measured here.
- `mise current rust` printed `1.97.1` at the root and `1.98.1` in
  `crates/native`.
- `rustup component list --installed --toolchain 1.97.1` (for the PR body):

  ```text
  cargo-x86_64-pc-windows-msvc
  clippy-x86_64-pc-windows-msvc
  rust-analyzer-x86_64-pc-windows-msvc
  rust-docs-x86_64-pc-windows-msvc
  rust-std-aarch64-apple-darwin
  rust-std-x86_64-pc-windows-msvc
  rustc-x86_64-pc-windows-msvc
  rustfmt-x86_64-pc-windows-msvc
  ```

- `rustup component list --installed --toolchain 1.98.1` (for the PR body):

  ```text
  cargo-x86_64-pc-windows-msvc
  clippy-x86_64-pc-windows-msvc
  rust-analyzer-x86_64-pc-windows-msvc
  rust-docs-x86_64-pc-windows-msvc
  rust-std-x86_64-pc-windows-msvc
  rustc-x86_64-pc-windows-msvc
  rustfmt-x86_64-pc-windows-msvc
  ```

- The first `mise run fmt` failed with a Node.js error (output cut to its
  last lines, so the cause was not captured); an immediate rerun passed with
  no change. It looked transient.
- Contrary to the plan's note that `ci:local` would run no clippy or cargo
  test, a change to `crates/native/mise.toml` makes it select the native
  crate's checks: it ran its tests (867 passed) and took several minutes.

## Deferred issues (todo candidates)

- Pending check: whether a host whose toolchain lacks the rust-analyzer
  component gets it from a plain `mise install` once mise reports
  `rust@<version>` as already installed, or needs `mise install --force rust`.
  Basis: the plan's Step 1 implementation approach; this host already had the
  component, so it could not be measured. Files: `mise.toml`,
  `crates/native/mise.toml`. Step 1 was ticked on the automated criteria.
  How to check: `rustup component remove rust-analyzer --toolchain <v>`, then
  a plain `mise install` at the root and in `crates/native`, then
  `rustup component list --installed --toolchain <v>`. Done when the result is
  recorded; if a plain `mise install` is not enough, add a
  `mise install --force rust` line to the setup docs (`docs/agents/native-app.md`
  or the README setup section).
