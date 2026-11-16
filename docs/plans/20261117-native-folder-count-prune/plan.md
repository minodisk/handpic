<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Prune the stale rows of the folder count cache (native app)

## Purpose

The GPUI app's folder tree keeps each folder's file count in the index's
`folder_counts` table, keyed by the tree node's own path (see
`../_archived/20261116-native-folder-count-cache/plan.md`). A row is removed
only by a zero write, a date split, a rename's carry, or `Index::clear`,
which the native app never calls. So a folder deleted or renamed outside the
app keeps its row forever, and a folder created again at the same path shows
the old count until it is listed.

After this work, whenever the tree lists a folder (an expand, `Refresh`, a
tree watcher change, `Expand All`, a reveal's chain, all through
`tree_listed`'s `Ok` branch), the rows of that folder's child folders that
the listing no longer holds are deleted, together with the rows under them
(decided by the user, 2026-11-17). The listing is the one `read_dir` the tree
already did: the prune reads nothing from disk. The Tauri app (`crates/app`)
is frozen and is not changed.

What the investigation found (the facts the step relies on):

- A hidden or dot-named folder gets no row in practice. `folders::list`
  leaves it out of its parent's children, `reveal` stops at a chain entry the
  tree does not hold (`this.tree.has(&dir)`), and `set_listed_count` only
  matches nodes, so such a folder is never listed by the tree and never
  written. The exception is a tree root (home, a volume, a root added by
  `tree::root_of`): a root is a node whatever its parent's listing says.
- `Index::rename_folder_counts` builds its prefix as
  `format!("{old}{MAIN_SEPARATOR}")`. That is right for a renamed folder but
  wrong for a parent that already ends in a separator (`C:\`, `/`), which a
  prune meets when a root is listed.
- `tree::CountWrites::push` replaces a folder's pending count in place, so
  the folder keeps its first position in the batch.
- `folders::list` skips an entry it cannot read (`entries.flatten()`, `kind`
  returning `None`), so a child that is unreadable for a moment is absent
  from the listing.
- `Tree::set_children` does not drop the nodes of children that left the
  listing; such a node keeps its listed `raw_count` for the session.
- The index is in WAL mode with `synchronous = NORMAL`.

## Steps

- [x] Step 1: Prune the `folder_counts` rows of the child folders a tree listing no longer holds
  - Done when:
    - `Index` has a method that, given a parent folder and the child paths
      to keep, deletes every `folder_counts` row under the parent whose
      first path segment below the parent is not one of the kept children:
      the absent child's own row and the rows under it. It leaves the
      parent's own row, the kept children's rows and everything under them,
      and every row outside the parent
    - The method takes the paths as spelled (no canonicalization, no case
      fold) and builds the parent prefix with exactly one separator, also
      for a parent that already ends in one (`C:\`, `/`)
    - Every successful tree listing (`tree_listed`'s `Ok` branch) queues one
      prune of the listed folder with its listed children; a failed listing
      and an open's count (`set_listed_count`) queue none; nothing is queued
      without an index
    - The prune adds no disk access: no `read_dir`, `metadata` or
      `canonicalize` beyond the listing the tree already made
    - A prune and a count write reach the table in the order the tree
      learned them: a count pushed after a prune of its parent is applied
      after that prune, also when an earlier count of the same folder was
      still pending
    - The rows of a tree root that lies under the listed folder, and the
      rows under it, are not pruned even when the listing does not hold the
      child leading to it
    - Unit tests in `crates/backend/src/index.rs` cover: an absent child's
      row and its descendants' rows deleted; a kept child's row and its
      descendants' rows kept; the parent's own row kept; a prefix sibling of
      the parent (`photos2` beside `photos`) and its subfolders kept; an
      absent child whose name is a prefix of a kept child's (`a` absent,
      `ab` kept) deleted without touching `ab`; names holding `_` and `%`;
      an empty keep set deleting everything under the parent; a parent
      ending in the separator
    - Unit tests in `crates/native/src/tree.rs` cover: `CountWrites` handing
      out counts and prunes in push order; a later count of a folder moving
      behind a prune pushed after its earlier count; one write in flight at
      a time as before; the keep set holding the listed children plus the
      child leading to a root under the listed folder
    - `../../agents/native-app.md` (the count rules under "The folder tree is
      a `uniform_list` over rows kept in the view"), the `CLAUDE.md` layout
      text (`src/index.rs`'s `folder_counts` sentence and `src/tree.rs`'s
      `CountWrites` clause) and the `todo.md` item "Native app: real-device
      check of the folder tree's file counts on collapsed rows" describe the
      prune
    - `mise run ci:local` passes locally and the PR's GitHub checks pass
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/backend/src/index.rs`, `crates/native/src/tree.rs`,
      `crates/native/src/main.rs`, `docs/agents/native-app.md`, `CLAUDE.md`,
      `todo.md`. Nothing under `crates/app`
    - Backend: one new `Index` method beside `rename_folder_counts`, in its
      own transaction, errors as `String`. Match the parent with the same
      `substr(dir, 1, length(?)) = ?` rule (no `LIKE`, so `_` and `%` need
      no escaping) and `std::path::MAIN_SEPARATOR`; append the separator
      only when the parent does not already end in one. Do not bind one SQL
      parameter per kept child (a folder can hold thousands): either select
      the rows under the prefix and decide in Rust against a set, deleting
      by exact `dir`, or use a temporary table. The step decides which
    - Compare the child by the full path the listing gives (`Child::path`,
      which is `read_dir`'s `parent.join(name)` and so the key a listing of
      that child would write), byte for byte
    - Native: `tree_listed` already has `children` and `cx`; queue the prune
      there next to `cache_count`. The keep set is the listed children's
      paths plus, for each tree root that `relation` puts under the listed
      folder, the path of the listed folder's direct child leading to it; a
      pure function in `tree.rs` (it has `roots`, `relation`, `segments`,
      `join`)
    - `CountWrites`: keep one write in flight and the whole pending set
      going out as one batch, but make the batch an ordered list of two
      kinds of entry, a count and a prune. A count pushed for a folder that
      already has a pending count must replace it and move to the end of
      the batch, not stay in place; whether a second prune of the same
      parent replaces the first or is appended is the step's choice (both
      are correct). `push`'s and `settled`'s return type changes, so the
      existing test
      `count_writes_go_one_at_a_time_the_latest_count_of_a_folder_winning`
      is adjusted, its expected order included
    - `write_folder_counts` applies the batch in order under one
      `index::lock` guard on `background_spawn`; consecutive counts can
      still go as one `set_folder_counts` call. A failure is logged as
      today and the next batch still goes out
    - The tree's in-memory cached map is not changed by a prune (see
      Trade-offs); it is replaced whole at the next read
    - `native-app.md`: extend the rule "every count the tree learns from
      disk is written to `folder_counts`" with the prune (which listings,
      the keep set, the descendants, exact spelling, the order within
      `CountWrites`) and mark it Inferred until seen in the running app, as
      the neighbouring rules are
    - `todo.md`: add the prune to the item's background and one check to
      each platform's line (delete or rename a counted folder outside the
      app, expand or `Refresh` its parent, restart, create a folder of the
      same name: expect no count on it until listed). Keep it one item

## Trade-offs and risks

- **Descendants are pruned too** (decided by the user, 2026-11-17). A
  deleted folder's whole subtree leaves the table in one listing of its
  parent. The cost: a child that is absent for a reason other than deletion
  loses its subtree's counts, each back only after its own next listing.
  That covers an unplugged volume or an unreachable share whose mount parent
  is listed, a folder turned hidden, and an entry `folders::list` skipped as
  unreadable.
- **Exact spelling, not the tree's case fold.** Exact keeps the backend's
  rule that a `folder_counts` key is an opaque string and needs no fold in
  `riffle-backend`. Its effects on Windows and macOS: rows stored under a
  parent spelled in another case (a root spelled differently in an earlier
  session) are not found and so not pruned, as today; a child whose case
  changed on disk loses its old-spelling row, so its collapsed row shows no
  count until listed. That also removes one of the case-only duplicate rows
  the previous plan accepted. The folded alternative would need `tree.rs`'s
  `fold` duplicated in the backend, or a whole-table read on every batch.
- **Tree roots are kept.** A root is drawn and listed whatever its parent's
  listing holds, so without this a hidden root, or a root under a hidden
  folder, would have its rows deleted by every listing of an ancestor and
  written again by its own.
- **The in-memory cached map is not pruned.** The map is read at startup and
  after a rename or a date split, so a pruned row's count can still be drawn
  in the same session if a folder of that path appears again before the next
  read. Pruning the map as well would be one more `Tree` method, but its keys
  are folded while the table's are exact, and a node left over from an
  earlier listing still shows its own listed count for the session
  (`set_children` drops no node), so the session would not become fully
  consistent either way.
- **A prune queued before a rename or a date split can land after it.** A
  batch waits for the write in flight (which can sit behind the launch's
  eviction `VACUUM` or a scan batch on the same mutex); a rename or split
  that runs meanwhile moves or writes rows under names the queued prune's
  keep set does not hold, and the prune then deletes them. The result is a
  missing count for the renamed folder or the new date folders after a
  restart, repaired by their next listing. Accepted as rare; the guard would
  be dropping the pending prunes of the affected parent when such an
  operation runs.
- **A prune still queued at quit is lost**, like a count write: the stale
  rows stay until the parent's next listing.
- **Only tree listings prune.** An open's listing (`opened`,
  `set_listed_count`) knows the folder's files, not its child folders, so it
  prunes nothing; rows under a folder that is never listed again by the
  tree stay.

## Progress
