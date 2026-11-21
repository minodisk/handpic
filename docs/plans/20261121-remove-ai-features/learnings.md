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
