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

# No-replace rename for the import's copy

## Purpose

`handpic_core::copy::copy_files` promises that a destination that already
exists is never overwritten. It checks `dst.exists()` before reading a file
and again in `finish` just before `fs::rename`, but `fs::rename` replaces
the destination on every platform. A file created at the destination between
that last check and the rename, such as a sidecar the app writes for a RAW
that just landed, is replaced. This is the `todo.md` item "Core:
`copy::copy_files` can replace a destination created between its check and
the rename", left by the import-from-card work
([plan](../_archived/20261106-import-from-card/plan.md)).

Once done, the final rename itself refuses an existing destination on
Windows, Linux and macOS, the job fails with the same `exists: ...` message
the pre-check gives, and the todo item is gone.

## Steps

- [x] Step 1: Rename the temp file into place without replacing, per platform, and close the todo item
  - Done when:
    - `crates/core/src/copy.rs` no longer ends a copy in a plain `fs::rename` on Windows, Linux or macOS: Windows calls `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, Linux `renameat2` with `RENAME_NOREPLACE`, macOS `renamex_np` with `RENAME_EXCL`.
    - A destination found existing at rename time fails the job with exactly `exists: <dst display>`, the message the reader's pre-check produces; any other rename error keeps `rename: <dst display>: <error>`. The temp file is removed in both cases and the existing destination's bytes are untouched.
    - A unit test creates the destination after the reader's existence check and before the rename, and asserts: the job is `Outcome::Failed` with a message starting `exists:`, the destination still holds the bytes the test wrote, no `TEMP_SUFFIX` file is left, and a following job is still copied.
    - A unit test calls the no-replace rename directly on a temp dir with an existing destination and asserts an `AlreadyExists` error (not the unsupported-filesystem fallback), both files intact; and one with a free destination asserts the move.
    - On Windows, a unit test renames into a destination path longer than 260 characters and succeeds (parity with `fs::rename`).
    - The existing tests of `copy.rs` pass unchanged.
    - The `### Core: copy::copy_files can replace a destination created between its check and the rename` section is deleted from `todo.md` (heading, Background, Files, TODO); no other todo item is touched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass (the `mise run test` matrix on ubuntu-latest, macos-latest and windows-latest is what compiles and tests the other platforms' branches).
  - Implementation approach:
    - Files: `crates/core/src/copy.rs`, `todo.md`. No `Cargo.toml` change, so neither `Cargo.lock` (root, `crates/app`) moves.
    - Add one private function in `copy.rs`, e.g. `rename_noreplace(from: &Path, to: &Path) -> io::Result<()>`, with one `cfg`-gated body per platform, and have `finish` call it in place of `fs::rename`. `finish` maps `io::ErrorKind::AlreadyExists` to `format!("exists: {}", dst.display())` and everything else to the present `rename: ...` message. `finish` already removes the temp file on any error; keep that.
    - Linux (`cfg(target_os = "linux")`): `libc` is already a `cfg(unix)` dependency of the crate (used in `scan.rs`). Paths go through `CString::new(path.as_os_str().as_bytes())` (`std::os::unix::ffi::OsStrExt`), an interior NUL becoming an `InvalidInput` error. Call with `libc::AT_FDCWD` for both directories and `libc::RENAME_NOREPLACE`; prefer `libc::syscall(libc::SYS_renameat2, ...)` over the `libc::renameat2` wrapper so the binary takes no `renameat2@GLIBC_2.28` symbol and the same code builds on every libc environment. A non-zero return is `io::Error::last_os_error()`; `EEXIST` maps to `AlreadyExists` by std.
    - macOS (`cfg(target_os = "macos")`): `libc::renamex_np(from, to, libc::RENAME_EXCL)`, same path conversion and error handling.
    - Windows (`cfg(windows)`): the crate has no Windows API dependency. Declare the one function by hand, `#[link(name = "kernel32")] extern "system" { fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32; }`, call it with flags `0` (same-volume move, no replace; the temp file is always in the destination's folder), paths as NUL-terminated UTF-16 from `std::os::windows::ffi::OsStrExt::encode_wide`. A zero return is `io::Error::last_os_error()`; std maps both `ERROR_ALREADY_EXISTS` and `ERROR_FILE_EXISTS` to `AlreadyExists`.
    - Windows long paths: `std::fs::rename` converts a path past `MAX_PATH` to verbatim form internally, and a raw `MoveFileExW` call does not, so convert before the call. One way that needs no dependency: `fs::canonicalize` the destination's parent folder (it exists, and on Windows the result is a `\\?\` path, UNC included), then join the file names of the temp file and the destination onto it. Handle a destination with an empty parent (a bare file name) by canonicalizing `.`. The long-path test in "Done when" decides whether the conversion is right.
    - Other targets (`cfg(not(any(windows, target_os = "linux", target_os = "macos")))`): return an `Unsupported` error so the fallback below runs and the crate still compiles there.
    - Unsupported filesystem: when the platform call fails with `EINVAL` or `ENOSYS` on Linux, or `ENOTSUP` on macOS (and for the other-targets body), fall back to what `finish` does today: `dst.exists()` giving `exists: ...`, else `fs::rename`. Keep this fallback the only place the pre-rename `exists()` check remains.
    - Remove the `dst.exists()` check from `finish`'s main path (the rename now reports it). This is what lets the race test prove the rename: the writer, and so the `progress` callback, runs on the thread that called `copy_files_with`, and the reader's `dst.exists()` for a job precedes its `Msg::Begin`, so a test whose `progress` callback writes the destination on that job's first chunk creates it between the check and the rename with no new hook. Give the source more than one chunk at `small()` options so the callback fires before `Msg::End`.
    - Keep the reader's pre-check in `read_all` as it is: it spares reading a file whose destination is already there.
    - Update `finish`'s doc comment (it says "unless `dst` appeared meanwhile") to say the rename itself refuses an existing destination, and name the fallback. The module doc's "A destination that already exists is never overwritten" stays.
    - `CLAUDE.md` describes `copy.rs` as "renamed into place and never overwriting", which stays accurate; change it only if the implementation ends up contradicting it.
    - Do not change `crates/engine/src/import.rs`: its `run_plan` counts a failed job whose destination exists with the source's size as already imported and does not read the message.
    - The Linux and macOS bodies cannot be built on the Windows host. Write them against the `libc` 0.2.189 signatures in the lock and let the PR's ubuntu and macos jobs compile and test them; record any correction in `learnings.md`.

## Trade-offs and risks

- **A filesystem that does not support the no-replace rename** (Linux `EINVAL` / `ENOSYS`, e.g. NFS or an old kernel; macOS `ENOTSUP` on a volume without `VOL_CAP_INT_RENAME_EXCL`). The plan falls back to the present check-then-`fs::rename`: an import to such a volume keeps working and the race stays open there only. Options not taken: failing the job (safe, but an import to a NAS over NFS would stop working), or `fs::hard_link` plus removing the temp file (atomic and works on NFS, but adds a third code path and does not work on FAT / exFAT).
- **Windows binding: hand-declared `extern` vs. `windows-sys`.** The plan declares `MoveFileExW` by hand: one function, no new dependency, no change to either `Cargo.lock`.
- **Windows long paths.** Replacing `fs::rename` with a raw API call loses std's verbatim conversion. Without the conversion, an import into a destination past 260 characters would start failing at the rename, a regression. The long-path unit test guards it.
- **Where the race test hooks in.** The plan uses the `progress` callback, which requires dropping the pre-rename `exists()` check from `finish`'s main path.
- **`EINVAL` is not only "unsupported".** `renameat2` also returns it for invalid flag combinations and for moving a directory into itself; neither can happen for a temp file renamed within its own folder, so treating it as unsupported is safe here.

## Progress
