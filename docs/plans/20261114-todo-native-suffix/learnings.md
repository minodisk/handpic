# Learnings

## Step 1

- The plan named `### Native: the per-folder EXIF filter sections in the filter menu (closes port Step 7)` as the last item of `todo.md`, but no item has that title; the file ended with the item on the status line's `n/total` lagging a refilter that hides the shown file. The new item went after that one, the actual last item.
- `CLAUDE.md` still names the native index folder `com.minodisk.handpic.native`, while `datadir.rs` and `docs/agents/native-app.md` use `dev.minodisk.handpic.native`. Left as is (out of scope); the new todo item lists `CLAUDE.md` among the files to update at the switch-over anyway.

## Deferred issues (todo candidates)

- `CLAUDE.md`'s `crates/native` paragraph says the native index lives at `<cache dir>/com.minodisk.handpic.native/index.sqlite`, but `crates/native/src/datadir.rs` (`index_path`) and `docs/agents/native-app.md` ("The native app keeps its own index (Inferred)") put it under `dev.minodisk.handpic.native`. Found while checking the facts for this plan's Step 1. Files: `CLAUDE.md`, `crates/native/src/datadir.rs`.
