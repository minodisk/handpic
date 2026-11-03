# Learnings

## Step 1

- The whole change is one entry in `ADVANCING` plus `judged = judge(...)` in
  `runAction`'s `unflag` case; `judge` already returns the target count
  whether or not anything changed, so a repeat `u` on an unflagged file
  advances with no extra code, and every other gate (view-only, Compare,
  batch, `stillCurrent`) is the shared post-switch `shouldAdvance` block.
- The JA usage bullet words the key as 「不採用または採用を外す操作」 to match
  the key table's `u` row (「現在のファイルの不採用または採用を外す」).

## Deferred issues (todo candidates)

- Pending manual check (Step 1): on a real build (any platform), with
  `Auto-advance after a star, reject or pick` on, press `u` on a picked file,
  on a rejected file and on an unflagged file; each should move to the next
  file (the unflagged one writing nothing and adding no undo entry). With
  Auto-advance off, in Compare, or with two files selected, `u` should stay.
  The step's checkbox was ticked on the automated criteria (`advance.test.ts`
  and `mise run ci`). Files: `crates/app/ui/src/advance.ts`,
  `crates/app/ui/src/main.ts`.
