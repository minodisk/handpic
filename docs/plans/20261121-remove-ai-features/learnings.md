# Learnings: remove-ai-features

## Step 1

- `FilterArgs` and `SettingsArgs` carry `#[serde(deny_unknown_fields)]`, so
  once `genres` and `ai_enabled` left the structs a client sending them gets a
  deserialization error ("unknown field" naming `genres`) from the engine before the
  request reaches the app; `by: "recommended"` likewise fails as an unknown
  `SortBy` variant. The engine test `the_removed_ai_values_are_unknown_fields`
  and `set_sort_takes_the_menus_keys_by_name` state it. The app's own
  `set_sort` now answers `recommended` with its generic "by must be one of
  name, capture, rating".
- The busy-guard case of `every_feature_not_in_the_app_is_refused_with_the_shared_message`
  used `refit_ranking`; it now uses `rename_file`, the one tool left in
  `UNPORTED_TOOLS`.
- Left for later steps: `todo.md`'s "decide the fate" item still names the
  removed refusals (Step 5), and `docs/agents/app.md`'s `aiEnabled` /
  `recommended` rule near the settings store and the latency-gate rule
  (Step 5). The stored `sortOrder` `recommended` notes in `docs/agents/app.md`
  stay (Decisions).

## Step 2

- `Companion::seven` was also the test helper `silent()`'s constructor, so it
  became `Companion::new`, building `tool_router() + app_router()`; the two
  `#[tool_router]` blocks stay separate. `with_screenshot` adds its router to
  whatever `tools` holds, so with no `with_all_tools` left to replace the
  router afterward the call order note in `docs/agents/app.md` went.

## Step 3

- The v27 drop is `DROP TABLE IF EXISTS embeddings` under
  `(20..27).contains(&version)`. `IF EXISTS` because a failed migration
  rebuilds the index and loses dirty ratings; dropping the table also drops
  its `embeddings_dir` index.
- Of the migration fixtures, only the v19 one ran `DROP TABLE embeddings;`
  (the older ones relied on the table coming back through
  `CREATE TABLE IF NOT EXISTS`). It became the v26 test. The v20-v25 fixtures
  now re-create the table from the test constant `OLD_EMBEDDINGS` and assert
  through `has_embeddings` (`sqlite_master`, table and index) that `open`
  drops it; `a_fresh_index_has_no_embeddings_table` covers a new index.
- `with_suffix` and `lock` stay: `index.rs` itself uses them (`open`'s
  `-wal` / `-shm` cleanup, the scan passes), not only `ai.rs`. No engine
  dev-dependency was `ai.rs`'s alone.
- `reqwest` and `rustls` stay in both lock files as transitive dependencies
  (`self_update`, `rmcp`); the regenerated lock files only lost the two names
  in `handpic-engine`'s dependency list.
- `RANK_DIRS` became `KEPT_DIRS`. The eviction test that asserted
  `rank_dirs()` was empty after `clear` now counts `KEPT_DIRS` rows directly.
- The deleted `shared-crates.md` section "The third pass" also held two
  bullets unrelated to the embeddings: `output::outputs_in`'s never-guess rule
  and the `views` dwell / table / test-fixture facts. Both moved into
  sections of their own, the `views` one without the ranking.

## Step 4

- PIL's resampling went to its own `pub mod resample` rather than private in
  `fullsharp.rs`: the PIL-coefficient test exercises `Filter::Bilinear`,
  which nothing else uses, so a private enum would have needed the variant
  dropped (and the test changed) to pass clippy's dead-code check. `Filter`
  and the two functions were moved byte for byte; `resample` stays
  `pub(crate)`.
- No core or CLI dependency went (`working_set`, `dot` and `top_k` were the
  embedding commands' alone and went with them; the CLI's other imports all
  have other users), so neither lock file changed.
- Python run from a Bash heredoc here saw `\\` collapsed to `\`, so a
  replacement of the CLI's usage string (which holds literal `\n`) silently
  matched nothing; a script file written with the Write tool, or the Edit
  tool, did it.
