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

# Keep the production updater key in a `main`-only environment

## Purpose

Today `UPDATE_SIGNING_KEY`, the production zipsign key the updater trusts, is
a repository secret. A dispatch of `release.yml` on any branch runs that
branch's workflow code with the key, so unreviewed branch code can sign
archives, and "never publishes from a non-`main` ref" is a check inside the
branch's own workflow. This was an accepted risk of
[native-release](../20261121-native-release/plan.md) Step 2 and is
the open `todo.md` item "Release: decide whether to protect
`UPDATE_SIGNING_KEY` from branch dispatches".

The user has decided: the secret moves into a GitHub environment, `release`,
whose deployment branch policy allows only `main`. Once that is done and the
repository-level secret is deleted, no workflow run on another ref can read
the production key, whatever its workflow file says. A branch dry run is
made with the test key instead (`gh workflow run package.yml --ref <branch>
-f update_test_repo=<owner/name>`, which already exists).

## Steps

- [x] Step 1: Read the production key through the `release` environment, and record the decision
  - Done when:
    - `release.yml`'s `guard` job and `package.yml`'s three signing legs
      (`macos`, `windows`, `linux`) reference the `release` environment
      whenever they sign with the production key; a `package.yml` dispatch
      (no `sign_production`) references no environment.
    - A `release.yml` dispatch on a ref other than `main` no longer builds
      and signs: the workflow comments, the `version` input's description
      and the docs say what happens instead and name the test-key path.
    - `docs/agents/app.md`: the Rule "a branch dispatch ... runs that
      branch's `release.yml` with the production key" is replaced by the
      decision (the environment, its `main`-only policy, what a branch
      dispatch does now, how both directions are verified), and the other
      passages that describe the repository secret or the branch dry run
      agree with it.
    - `docs/agents/github-actions-workflows.md`, `CLAUDE.md`,
      `.claude/skills/release/SKILL.md` and the header comment of
      `tools/package/sign-production.sh` no longer describe the old
      behavior.
    - The `todo.md` item "Release: decide whether to protect
      `UPDATE_SIGNING_KEY` from branch dispatches (native-release)" is
      deleted (the whole item with its separator).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - The PR-side measurements (verification (a) and the empty environment
      name check below) are run by the main agent after "Before the merge" of
      "Repository settings" is done, and recorded in `learnings.md`; the
      implementer does not dispatch workflows.
  - Implementation approach (as far as it is known; omit if unknown):
    - How environment secrets reach a reusable workflow (GitHub docs,
      "Reuse workflows", read 2026-11-26): the job that calls a reusable
      workflow cannot use `environment`; the job inside the reusable
      workflow sets `environment` and then receives the environment
      secret's value; the caller must still pass the secret by name ("You
      can pass a secret by name even if it only exists in the
      environment"), else it resolves to an empty string. So:
      - `release.yml`'s `build` job keeps `secrets: UPDATE_SIGNING_KEY:
        ${{ secrets.UPDATE_SIGNING_KEY }}` and gets no `environment`.
      - `package.yml` keeps the `workflow_call.secrets` declaration and the
        `env: UPDATE_SIGNING_KEY: ${{ secrets.UPDATE_SIGNING_KEY }}` on the
        signing steps.
      - Each of `macos`, `windows` and `linux` gains an `environment` that
        is `release` only under `sign_production`.
      - `guard` in `release.yml` gains `environment: release`
        unconditionally (it always checks the production key).
    - Intended shape for the legs (the documented object form;
      `deployment: false` uses the environment's secrets without creating a
      deployment object, so the repository gets no deployment records):

      ```yaml
      environment:
        name: ${{ inputs.sign_production && 'release' || '' }}
        deployment: false
      ```

      and for `guard`: `environment: { name: release, deployment: false }`.
      The `checksums`, `publish` and `release-please` jobs need no
      environment.
    - Verify on the PR (none of these is documented):
      - An empty environment name is accepted and means "no environment":
        dispatch `gh workflow run package.yml --ref <pr-branch>` and see
        the three legs start with no environment; cancel the run once they
        have started.
      - The pinned actionlint (1.7.12, `mise run lint`) accepts the
        `deployment` key (the implementer checks this locally).
      - The branch policy is enforced with `deployment: false`: this is
        verification (a) below.
      - If any of the three fails, stop and take the matching option in
        "Trade-offs and risks" to the user; do not improvise.
    - `guard` cleanup in `release.yml`: with the environment on `guard`,
      GitHub rejects a non-`main` dispatch before any step runs, so the
      `publish=false` / "Dispatched on $GITHUB_REF, not main: nothing is
      published." path becomes unreachable. Replace it with an explicit
      failure that names the test-key path (reached only if the
      environment's policy is ever loosened or the environment recreated
      without one), for example: `release.yml runs only on main; package a
      branch with: gh workflow run package.yml --ref <branch> -f
      update_test_repo=<owner/name>`. Whether the then-constant `publish`
      output and the `publish` job's use of it are removed in the same PR
      is the implementer's call under "remove what your change orphaned";
      say which in the PR body.
    - Comments to bring in line:
      - `release.yml`'s header ("a dispatch on another ref builds and
        signs, uploads the workflow artifacts ...");
      - the `version` input's description;
      - the comment above `guard`;
      - `package.yml`'s header ("since secrets.UPDATE_SIGNING_KEY would
        resolve on a dispatch too").
      Keep the `sign_production` gate itself: it is still what keeps a
      `package.yml` dispatch on `main` from signing with the production
      key.
    - `tools/package/sign-production.sh`: only the header comment "(the
      repository secret of that name)" changes, to name the `release`
      environment's secret. Its messages stay true and are not touched.
    - `docs/agents/app.md` passages to change:
      - the `package.yml` paragraph ("resolves to the repository secret on a
        direct dispatch too");
      - job 1 and job 2 of "The `Release` workflow publishes a release";
      - the paragraph "A dispatch on any ref but `main` ... runs the guard
        and the build" (now: rejected at `guard`; a branch is packaged
        through `package.yml` with `update_test_repo`);
      - the Rule on how the secret is set (`gh secret set
        UPDATE_SIGNING_KEY --env release`, still by the user only);
      - the branch-dispatch Rule, rewritten as the decision. Record there:
        the environment's name and policy; that the caller passes the secret
        by name and the legs declare the environment; that the calling job
        cannot; the expected rejection (the measured text is filled in after
        verification (a)); the two verifications; that the settings are the
        user's to change.
    - `docs/agents/github-actions-workflows.md`:
      - the section "A dispatchable reusable workflow sees the repository
        secrets too" stays true in general; fix the sentences that use
        `UPDATE_SIGNING_KEY` as its example;
      - add the environment-secret rule above (one short section, with its
        confidence label per the file's convention).
      "Prove a signing secret before the long build" stays, with `guard`
      now reading the environment's secret.
    - `CLAUDE.md` (the `package.yml` / `release.yml` passage): "the
      production key from the `UPDATE_SIGNING_KEY` secret" and "a dispatch
      on another ref publishes nothing".
    - `.claude/skills/release/SKILL.md`: the secret `guard` checks, and "A
      dispatch on another ref only builds and signs, uploading workflow
      artifacts".
    - Do not edit archived plans or `docs/plans/20261109-gpui-port/plan.md`.
    - Pushing a workflow change needs the `workflow` scope
      (`github-actions-workflows.md`).

## Repository settings (manual; each is the user's call, outside the step's diff)

An agent runs the `gh api` commands only when the user approves each one. No
agent reads, copies or asks for the key: the user registers the secret's
value.

Before the PR's first dispatch:

1. Create the environment with a custom branch policy:

   ```sh
   gh api -X PUT repos/minodisk/handpic/environments/release --input - <<'JSON'
   {"can_admins_bypass": false, "deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
   JSON
   ```

2. Allow only `main`:

   ```sh
   gh api -X POST repos/minodisk/handpic/environments/release/deployment-branch-policies -f name=main -f type=branch
   ```

3. Read it back:

   ```sh
   gh api repos/minodisk/handpic/environments/release --jq '{can_admins_bypass, deployment_branch_policy}'
   gh api repos/minodisk/handpic/environments/release/deployment-branch-policies --jq '.branch_policies[] | {name, type}'
   ```

4. The user registers the key in the environment, from their private key
   file:

   ```sh
   base64 -w0 <private-key> | gh secret set UPDATE_SIGNING_KEY --env release --repo minodisk/handpic
   gh secret list --env release --repo minodisk/handpic
   ```

After the merge:

5. Delete the repository-level secret, before verification (b) (decided:
   the release candidate then proves the environment-only path).

   ```sh
   gh secret delete UPDATE_SIGNING_KEY --repo minodisk/handpic
   gh secret list --repo minodisk/handpic
   ```

## Verification

- (a) A branch cannot use the production key. On the PR's branch, after
  settings 1-4: `gh workflow run release.yml --ref <pr-branch> -f
  version=0.0.0-rc.0`. Expected: `guard` fails before any step runs, with
  GitHub's environment protection annotation (record the exact text), and
  `build` and `publish` do not run. If `guard` starts running steps instead,
  cancel the run at once.
- (b) A release from `main` still works. After the merge and setting 5, the
  user dispatches a release candidate: `gh workflow run release.yml --ref
  main -f version=<the next unused x.y.z-rc.N>`, then `mise exec
  cargo:zipsign@0.2.2 -- bash .claude/skills/release/scripts/verify-release.sh
  v<version>` reports `STATUS=ok`.

## Trade-offs and risks

- Decided: the repository secret is deleted before (b); a branch dispatch's
  failure message is GitHub's annotation (no extra job).
- If an empty environment name is not accepted on the legs: either move
  production signing into one `release.yml` job with `environment: release`
  (native-release's rejected option B), or add a second unrestricted
  environment for the unsigned case. Decide with the user.
- If the branch policy is not enforced with `deployment: false`, or
  actionlint 1.7.12 refuses the key: drop `deployment: false` (each release
  run then creates deployment records), or bump actionlint.
- Not covered: the key still reaches four runners that run third-party
  actions; no required reviewer; anyone with push access to `main` can sign.
- Branch dry runs of `release.yml` itself are gone; only its `package.yml`
  part can be rehearsed on a branch, with the test key.

## Progress

- (2026-11-26) Step 1 complete.
- (2026-11-27) Verification (a) and the empty-environment-name check measured on the PR branch (runs 45846600132, 45846638280; see `learnings.md`). Still pending, the user's: the repository secret's deletion after the merge and verification (b), filed in `todo.md`.
