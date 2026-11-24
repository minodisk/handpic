# Learnings

## Step 1

- `LICENSE-APACHE` was downloaded from
  `https://www.apache.org/licenses/LICENSE-2.0.txt` unmodified (202 lines,
  SHA-256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`,
  the canonical file's hash).
- `license.workspace = true` in `handpic-core` and `handpic-engine` resolves
  against the repository root's `[workspace.package]` also when they are built
  as path dependencies of `crates/app`'s own workspace: `cargo metadata
  --manifest-path crates/app/Cargo.toml` reports `MIT OR Apache-2.0` for
  `handpic-app`, `handpic-core` and `handpic-engine`. No fallback to literal
  keys was needed, and neither `Cargo.lock` changed.
- `bash tools/package/notices.sh <tmp>` leaves the five existing notices plus
  `LICENSE-MIT` and `LICENSE-APACHE`.
- On the Windows host a bare `python3 -` in Git Bash blocked on stdin until
  killed (as `CLAUDE.md` warns); use `mise x -- python` or avoid it.
- `mise run ci:local` (clippy and tests of the four changed crates, from a
  cold `target/`) was stopped by Claude Code after 180 s for host memory
  pressure, not by a failure; per the user's rule that the PR's GitHub
  Actions verify, it was not re-run locally. `cargo metadata` on both
  workspaces and the `notices.sh` run passed.

## Deferred issues (todo candidates)

- Pending post-merge check (not a manual platform check): after the merge to
  `main`, `gh api repos/minodisk/handpic --jq .license` should be non-null with
  `spdx_id` `Apache-2.0` (not `NOASSERTION`), and the repository page's
  sidebar should list both licenses. GitHub's detection can lag a push by
  minutes. Basis: plan Step 1's last "Done when" item. Files: `LICENSE-MIT`,
  `LICENSE-APACHE`. Step 1's checkbox was ticked on the pre-merge criteria.
