# Learnings

## Step 1: `.dop` flag across virtual copies

- `locate` now collects every depth-3 `{` line inside `Items`; the first is `Items[0]` (the `Doc` fields as before) and the rest are `VirtualCopy`s holding only their close, `ShouldProcess` and `ModificationDate`. Naming the struct `Copy` would have shadowed the `Copy` trait, hence `VirtualCopy`.
- The two-item test text is built from `_DSC0003.ARW.dop`: the item is the slice between `Items = {\n` and the last `}\n,\nUuid = "`, and the copy gets its own Uuid, `ModificationDate`, `Rating`, `Orientation` and a `ColorLabel`, so a byte-for-byte comparison shows which item a write touched.
- `mise run ci:local` failed twice on `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` (`crates/core/src/scan.rs`, untouched here): `took 1.14s` / `took 1.10s` against its 1 s bound under the whole core suite's load. It passed alone in 0.57 s and on the third full run.
- `CLAUDE.md`'s `src/dop.rs` clause ("rating, the tri-state pick / reject flag as `ShouldProcess`, and `ColorLabel`") still holds and was left unchanged.

## Deferred issues (todo candidates)

- Pending manual check (from Step 1, PhotoLab on any platform it runs on): give a frame a virtual copy in PhotoLab, close PhotoLab, change the frame's flag in Handpic with `.dop` sidecars on, reopen the folder in PhotoLab, and expect the new flag on both the master and the virtual copy. The per-item `ModificationDate` rule is unverified. Step 1 was ticked on the automated criteria; the item is already filed in `todo.md` as "Core: real-device check of the `.dop` flag on a PhotoLab virtual copy" (the plan asked for it there). Files: `crates/core/src/dop.rs`, `docs/agents/photolab.md`.
- `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` (`crates/core/src/scan.rs`) is load-sensitive: its 1 s bound on a cancel was exceeded (1.10-1.14 s) in two of three `mise run ci:local` runs on Windows while the full `handpic-core` suite ran, and it passes alone in 0.57 s. Found while running Step 1's local checks; out of scope for this step.
