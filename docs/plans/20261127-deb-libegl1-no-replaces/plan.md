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

# Linux deb: depend on `libegl1`, no `Conflicts` / `Replaces: riffle`

## Purpose

Close the `todo.md` item "App: decide the Linux deb's `Conflicts` /
`Replaces: riffle` and whether it depends on `libegl1`". Both decisions are
taken without a Linux machine at hand:

- **No `Conflicts` / `Replaces: riffle`.** Handpic and Riffle are separate
  products with separate package names, binaries and install paths
  (`/usr/lib/handpic`), so the two debs install side by side. This matches
  Windows, where Riffle 1.10.0 (`%LOCALAPPDATA%\Riffle`) and Handpic 2.1.0
  (`%LOCALAPPDATA%\Programs\Handpic`) are both installed on one Windows 11
  host with two uninstall entries (`Riffle` and `dev.minodisk.handpic_is1`,
  seen on 2026-11-27). A `Replaces` / `Conflicts` would also make apt remove a
  Riffle the user may still want.
- **`libegl1` as a plain dependency**, not `libvulkan1 | libegl1`. The
  Vulkan loader package alone gives no adapter on a machine without a Vulkan
  driver, and wgpu's GL fallback then needs `libEGL.so.1`. With the
  alternative form, `libvulkan1` alone would satisfy the dependency and leave
  such a machine without `libEGL`. `libegl1` is small and present on every
  desktop, so always depending on it costs nothing.

Once done, a deb install on a Vulkan-less machine pulls in what the GL
fallback opens, and the guide says why the deb leaves a Riffle install alone.

## Steps

- [x] Step 1: Add `libegl1` to the deb's `Depends`, record both decisions and
      remove the todo item
  - Done when:
    - `tools/package/linux.sh` appends `libegl1` to `dpkg-shlibdeps`'s result
      when it is absent, in the same loop as `libvulkan1` and
      `libwayland-client0`, and its comment no longer says the GL fallback is
      left out.
    - `.github/workflows/package.yml`'s "Check the .deb" step asserts
      `libegl1` in the built deb's `Depends` next to `libc6`, `libvulkan1`
      and `libwayland-client0`.
    - `crates/app/packaging/linux/control` is unchanged.
    - `docs/agents/app.md` records both decisions with their reasons: the
      `Depends` rule under "Packaging" and the "Linux needs system libraries"
      section say `libegl1` is a dependency and why it is not
      `libvulkan1 | libegl1`, marked as not measured on a Vulkan-less
      machine; the `Package: handpic` rule says there is no `Conflicts` /
      `Replaces` against `riffle` and why, decided without a real-device deb
      check.
    - `CLAUDE.md`'s `linux.sh` description names `libegl1` in the `Depends`
      list.
    - The `todo.md` section "App: decide the Linux deb's `Conflicts` /
      `Replaces: riffle` and whether it depends on `libegl1`" is gone, heading
      through its TODO list, and no other section of `todo.md` is changed.
    - The PR's GitHub checks pass (local `ci:local` may be skipped; CI
      verifies).
    - A `Package` run dispatched on the PR's branch (no `update_test_repo`,
      no signing) passes its Linux job, "Check the .deb" included. The PR's
      own checks do not build the deb, so this run is the only one that
      exercises the change; put its run id in the PR description.
  - Implementation approach:
    - `tools/package/linux.sh`: add `libegl1` to
      `for package in libvulkan1 libwayland-client0; do`; rewrite the comment
      above it to list the GL fallback's `libEGL.so.1` among what GPUI opens
      at run time.
    - `.github/workflows/package.yml` (around line 589): add `libegl1` to
      `for package in libc6 libvulkan1 libwayland-client0; do`.
    - `docs/agents/app.md`: edit in place the three passages (around lines
      373, 851-852 and 855-858). Keep the measured `dpkg-shlibdeps` result
      and its run ids.
    - `CLAUDE.md` (around line 260): add `libegl1` to the `Depends` list.
    - `todo.md`: delete only that section, up to the next `###` heading.
      Other sessions edit `todo.md` in parallel: rebase before the edit and
      touch no other line.
    - No Rust changes and no local cargo build.

## Trade-offs and risks

- **`libegl1` is not measured.** No Vulkan-less machine was at hand; the
  Linux real-device bullet in the preceding `todo.md` section keeps tracking
  whether the GL fallback starts.
- **`libvulkan1 | libegl1` not taken**: it lets the loader alone satisfy the
  dependency with no usable adapter.
- **No `Conflicts` / `Replaces` is decided without the real-device deb
  check.** Coexistence was read from the packaging (package names and install
  paths differ). If a path collision with the Tauri deb turns up during
  implementation, stop and bring it back rather than adding `Replaces`.
- **`docs/plans/20261109-gpui-port/packaging-updater.md`** (in flight) still
  names the question as open; it is left alone as a record of that plan's
  state (decided by the caller: surgical change).

## Progress
