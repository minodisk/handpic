# Learnings

## Step 1: `.dop` flag across virtual copies

- `locate` now collects every depth-3 `{` line inside `Items`; the first is `Items[0]` (the `Doc` fields as before) and the rest are `VirtualCopy`s holding only their close, `ShouldProcess` and `ModificationDate`. Naming the struct `Copy` would have shadowed the `Copy` trait, hence `VirtualCopy`.
- The two-item test text is built from `_DSC0003.ARW.dop`: the item is the slice between `Items = {\n` and the last `}\n,\nUuid = "`, and the copy gets its own Uuid, `ModificationDate`, `Rating`, `Orientation` and a `ColorLabel`, so a byte-for-byte comparison shows which item a write touched.
- `mise run ci:local` failed twice on `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` (`crates/core/src/scan.rs`, untouched here): `took 1.14s` / `took 1.10s` against its 1 s bound under the whole core suite's load. It passed alone in 0.57 s and on the third full run.
- `CLAUDE.md`'s `src/dop.rs` clause ("rating, the tri-state pick / reject flag as `ShouldProcess`, and `ColorLabel`") still holds and was left unchanged.

## Step 2: skip oversize sidecars in `Rewrite Sidecars from Index…`

- `targets` now takes the format and lists with `read_listing(dir, Some(format))`, then drops a row whose RAW has a sidecar of the format's kinds (`trash::sidecars_of`) with `size > MAX_SIDECAR_BYTES`, the same comparison `sidecar::reconcile_of` makes. The filter runs after the "no row" refusal and before `mark_dirty`, so a folder whose every row is oversize still opens the dialog on a rewrite of 0 files.
- `Summary` gained `skipped`, the same figure as `Preview::skipped` (taken from the same `targets` call). Neither struct reaches MCP clients as is: `companion.rs` shows only the dialog's text, so the new field changes no tool reply.
- The dialog's row now reads `N files skipped (no judgment on record, or a sidecar larger than 4 MiB)`, the MiB figure taken from `MAX_SIDECAR_BYTES`; the status line adds `, N skipped` before `, N failed`.
- `CLAUDE.md`'s `src/foldersidecars.rs` clause still holds and was left unchanged.
- The first `mise run ci:local` failed on the exactly-`MAX_SIDECAR_BYTES` test: a 4 MiB file of filler is not skipped, so the writer patches it and fails with `no rdf:Description element`. The test now pads a real XMP packet with whitespace before `<?xpacket end` to exactly 4 MiB, which the writer patches.

## Deferred issues (todo candidates)

- Pending manual check (from Step 1, PhotoLab on any platform it runs on): give a frame a virtual copy in PhotoLab, close PhotoLab, change the frame's flag in Handpic with `.dop` sidecars on, reopen the folder in PhotoLab, and expect the new flag on both the master and the virtual copy. The per-item `ModificationDate` rule is unverified. Step 1 was ticked on the automated criteria; the item is already filed in `todo.md` as "Core: real-device check of the `.dop` flag on a PhotoLab virtual copy" (the plan asked for it there). Files: `crates/core/src/dop.rs`, `docs/agents/photolab.md`.
- `scan::tests::a_cancel_ends_a_scan_waiting_on_a_pause_or_a_permit` (`crates/core/src/scan.rs`) is load-sensitive: its 1 s bound on a cancel was exceeded (1.10-1.14 s) in two of three `mise run ci:local` runs on Windows while the full `handpic-core` suite ran, and it passes alone in 0.57 s. Found while running Step 1's local checks; out of scope for this step.
- `todo.md`'s "App: real-device checks" item for `todo-four-display-fixes` (around its line 48) expects the rewrite dialog's third row to read `N files with no judgment on record (skipped)`. Step 2 reworded that row to `N files skipped (no judgment on record, or a sidecar larger than 4 MiB)`, so that manual check's expected text is stale; this step was told to touch only its own `todo.md` section. Files: `crates/app/src/sidecars.rs` (`rewrite_rows`), `todo.md`.
