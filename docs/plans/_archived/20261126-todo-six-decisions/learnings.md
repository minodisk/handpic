# Learnings

## Step 1

- The "Develop: check intervening commits" item in `todo.md` sat between two
  `---` separators; removing the item with its trailing `---` keeps a single
  separator before the next item.
- `todo.md` uses LF line endings, so a line-range `sed -i '<a>,<b>d'` applied
  bottom-up removed the five sections and the Tier 3 line cleanly.
- The skill paragraph adds a short parenthetical with the incident (a plain
  `git checkout <base> -- <file>` dropping a later PR's hunks) so the rule
  carries its reason.
- `READ_SLOTS`'s doc comment in `crates/app/src/scan.rs` says nothing about
  NVMe or an open decision, so leaving it untouched leaves nothing stale.
