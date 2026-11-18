# Learnings

## Step 1: Remove the legacy `.native` index migration

- The Done-when leftover check
  `git grep -nIE 'move_native_index|IndexMove|index-aside|\.native\b'` still
  prints one live hit after this step: `.github/workflows/ci.yml`'s
  `needs.native.result` (the CI job named `native`). It is not about the index
  and Step 2 renames it (`needs.native` → `needs.app`), so it is left alone
  here; the check holds over the live files once Step 2 lands.
- `todo.md`'s removed item had only one `---` separator next to it (after it;
  the item before it had none), so that one was kept to separate the icon item
  from the next one rather than removed.
- `datadir.rs`'s test helpers `put` and `read` went with the move tests; the
  packaging tests below them never used them.
- `mise run ci:local` for this step took about five and a half hours on this
  Windows host (started 01:38, `test native` began at 07:05 and then
  finished in minutes). It was not hung: it was a cold build of the native
  crate's clippy and test targets, and from 07:05 a second `ci:local` from
  another session was building on the same host at the same time. The Bash
  tool moved the run to the background after 180 s even with the maximum
  timeout, so its exit code came in a task notification.
