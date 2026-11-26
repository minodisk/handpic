# Learnings: copy-noreplace-rename

## Step 1

- The `libc` 0.2.189 signatures the Linux and macOS bodies were written against: `SYS_renameat2: c_long` (x86_64 316, aarch64 276), `RENAME_NOREPLACE: c_uint` (Linux), `renamex_np(*const c_char, *const c_char, c_uint) -> c_int` and `RENAME_EXCL: c_uint` (Apple). Neither body could be built on the Windows host; the PR's ubuntu and macos jobs compile and test them.
- `CString::new`'s `NulError` converts into an `io::Error` of kind `InvalidInput` through std's `From`, so the path conversion is a plain `?`.
- Windows' verbatim form is taken as std does, `std::path::absolute` (`GetFullPathNameW`, no handle opened, so a volume that refuses `canonicalize` still renames) plus the `\\?\` / `\\?\UNC\` prefix (`verbatim` in `copy.rs`); the long-path test (two 150-character folders) passes through it.
- The race test needs no hook: the `progress` callback runs on the writer's thread after the job's first chunk, which is after the reader's `exists()` check and before the rename, so it creates the destination there.
