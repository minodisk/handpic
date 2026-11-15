# Learnings: native-tree-left-right

## Step 1

- `keys::key_name` names the plain space bar `space` and Shift+Space
  `shift+space`, so matching `"space"` in `tree_key_down` takes only the plain
  key; Shift+Space still falls through to `keys::typed_char` as type-ahead,
  as the plan left it.
- A folder not listed yet counts as expandable (`children.is_none_or(..)`), so
  Space on such a row (e.g. the `keyed()` fixture's `2026`) expands it; only a
  folder listed empty is a leaf that Space ignores.
- `move_tree_cursor` is still used by Up / Down / Home / End and type-ahead,
  so removing `TreeCommand::Focus` left no orphan.
- The `swallows_the_culling_actions` test now also names `burstNext` and
  `burstFramePrevious`, the actions bound to Down and Alt+Left, to pin that
  only plain `previous` / `next` pass the gate.

## Deferred issues (todo candidates)

- Pending manual check (native app, Windows or any desktop platform the user
  runs `crates/native` on): basis is Step 1's "Not verifiable headless"
  note in `plan.md`; files `crates/native/src/main.rs` (`tree_key_down`),
  `crates/native/src/treekeys.rs`, `crates/native/src/tree.rs`. Steps: open a
  folder with several photos, click a folder name in the tree so the tree
  has the keyboard (the cursor ring is drawn), then
  1. press Right / Left: the strip's current file moves to the next /
     previous photo and the preview follows, while the ring stays on the tree
     cursor;
  2. press Up / Down / Home / End: the tree cursor still moves;
  3. press Space on a collapsed folder with subfolders: it expands; Space
     again: it collapses; Space on a folder listed empty: nothing happens,
     and no type-ahead jump occurs;
  4. press Enter: the cursor's folder opens;
  5. press Shift+Left or Alt+Left: nothing happens (swallowed).
  The step's checkbox was ticked on the automated criteria (unit tests,
  `mise run ci:local`).
