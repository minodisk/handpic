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

# Remove the AI features

## Purpose

The image embeddings (SigLIP 2), the genre labels and the personal best-shot
ranking are removed from the repository. The user decided this after reviewing
the evaluations: the ranking reads single frames at AUC 0.508, the per-genre
offsets show no significant gain, and the genre labels remove barely more
non-picks than random. Nothing in `crates/app` calls this code; it was left in
`crates/core` and `crates/engine` when the Tauri app was removed. This closes
the `todo.md` item "Engine: decide the fate of the engine code only the Tauri
app reached" by its "remove it" branch.

Once done, no code, data file, model notice or setting for embeddings, genres
or the ranking remains outside `docs/plans/` and the index's migration history,
and the docs and `todo.md` describe the app without them. Archived plans and
their offline Python scripts stay untouched, as do the live plan folders,
`docs/plans/review-history/` and `CHANGELOG.md`.

## Steps

- [x] Step 1: Remove the AI tools and values from the MCP surface and the app
  - Done when:
    - The MCP server no longer lists `refit_ranking`, and `set_sort`,
      `set_filter` and `set_settings` no longer declare `recommended`, `genres`
      and `ai_enabled`.
    - `crates/app/src/companion.rs` has no `RECOMMENDED`, `GENRE`, `RANKING` or
      `AI_SWITCH` and no Coverage entry for them. `not_in_app`, `COMPARE` and
      `FILE_RENAME` (`rename_file`) stay and still refuse.
    - The app's comments no longer say the `Recommended` sort or the `Genre`
      section is still to come.
    - `docs/humans/usage.md` and `docs/humans/usage.ja.md` name only compare
      and file rename as refused, in sync with each other.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - `crates/engine/src/mcp.rs`:
      - Remove `SortBy::Recommended` and the `SortArgs` doc line about it.
      - Remove `FilterArgs.genres` and `genres` from the "set reset or at least
        one of ..." message.
      - Remove the `ai_enabled` settings argument and its entry in the
        settings message.
      - Remove the `refit_ranking` tool.
      - Reword the `set_sort`, `set_filter` and `set_settings` tool
        descriptions.
      - Reword the last sentence of `NATIVE_INSTRUCTIONS` to name only compare
        and file rename.
      - Update the tests: the full tool list in
        `all_tools_lists_the_apps_full_set`, and the `filter_args` /
        `settings_args` pass-through cases for `genres` / `ai_enabled`.
    - `crates/app/src/companion.rs`:
      - Remove the four constants and the Coverage rows "the recommended
        sort", "the Genre filter", "the ranking's refit" and "the AI switch".
      - Remove the `set_sort` `recommended` arm, the `genres` check in
        `set_filter` and the `ai_enabled` check in `set_settings`.
      - `UNPORTED_TOOLS` keeps only `rename_file`.
      - Update the module doc and the tests (around lines 3368, 3759-3806).
    - Verify what a client now gets for `by: "recommended"`, `genres` or
      `ai_enabled` (a schema error or the generic message) and make the tests
      state it. A silently ignored `genres` or `ai_enabled` is acceptable only
      if the existing argument handling already ignores unknown fields.
    - Comments only: `crates/app/src/filter.rs` (module doc line 13 and the
      tests' doc line 478), `crates/app/src/menu.rs` (module doc), and
      `crates/app/src/sort.rs` (`from_setting`'s comment and the tests' doc
      line 95).
    - The stored `sortOrder` value `recommended` keeps mapping to capture
      order (see Decisions); only its comment changes, and the two
      `settings.rs` tests stay.
    - The commit is not marked breaking (see Decisions).
    - Docs:
      - `docs/agents/app.md`: the tool list near line 1067, the tool count "35
        tools" (becomes 34), line 1114, the `not_in_app` list near lines
        1361-1366.
      - `CLAUDE.md`: the `companion.rs` clause "compare, the `recommended`
        sort, the Genre filter, `rename_file`, `refit_ranking`, the AI
        switch", and "`filter.ts` less the genres".
      - `docs/humans/usage.md` near line 1030 and the matching sentence in
        `docs/humans/usage.ja.md` (line 181).

- [x] Step 2: Remove the seven-tool default mode of the MCP server
  - Done when:
    - `Mcp::new` lists the app's whole tool set with the app's instructions.
    - `Mcp::with_all_tools`, `Companion::seven` and the Tauri `INSTRUCTIONS`
      text are gone.
    - `with_screenshot` still adds `screenshot` only when called.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged (both edit the same tool list and tests).
    - The mode's only users are tests in `crates/engine/src/mcp.rs`; the app
      (`crates/app/src/mcp.rs` line 121) always calls `with_all_tools`, and
      the CLI does not use MCP.
    - `crates/engine/src/mcp.rs`:
      - The constructor builds `Companion::tool_router() +
        Companion::app_router()` with the app's instructions. Keeping the two
        `#[tool_router]` blocks is fine; merging them is not required.
      - Delete `the_tools_are_exactly_the_seven_companion_tools`.
      - Adapt `all_tools_lists_the_apps_full_set` and
        `screenshot_is_listed_only_after_with_screenshot` (its `seven` and
        `seven_with` cases go).
      - Drop `.with_all_tools()` from the other tests.
      - Update the module doc (lines 7-10).
    - Rename `NATIVE_INSTRUCTIONS` to `INSTRUCTIONS` now that it is the only
      one.
    - `crates/app/src/mcp.rs`: drop the `.with_all_tools()` call and the
      module doc's mention.
    - Docs:
      - `docs/agents/app.md` near lines 1043 and 1067-1085. The note that
        `with_screenshot` must come after `with_all_tools` no longer applies.
      - `CLAUDE.md`: the engine `src/mcp.rs` clause ("the seven companion
        tools, or every tool after `with_all_tools`") and the app `src/mcp.rs`
        clause.

- [ ] Step 3: Remove the engine's AI code: the `aiEnabled` switch, pass 3, the
      `embeddings` table and the ranking's index readers
  - Done when:
    - `crates/engine/src/ai.rs` is gone, with `pub mod ai`.
    - `crates/engine/src/index.rs` has no `run_embed_scan`, `EmbedTodo`,
      `EmbedRow`, `RankRow`, `embeddings_missing`, `write_embeddings`,
      `embeddings`, `rank_rows`, `rank_dirs` or `drop_stray_embeddings`.
    - A fresh index has no `embeddings` table, and an older index loses it on
      open.
    - The 30-day eviction still spares a folder holding a viewed file or a
      reject.
    - `reqwest` and `rustls` are no longer dependencies of `handpic-engine`,
      and both lock files are regenerated and pass `--locked`.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Independent of Steps 1 and 2. `crates/app` calls none of this, so no app
      source changes are expected; confirm with a grep before starting.
    - Schema, following "Bumping `SCHEMA_VERSION` can strand an old
      per-version column guard" in `docs/agents/shared-crates.md`:
      - Bump `SCHEMA_VERSION` to 27 and add `26` to the accepted-version list
        in `migrate`.
      - Remove the `embeddings` table and its `embeddings_dir` index from the
        `CREATE` batch.
      - Drop the table for the versions that had it, with a guard keyed to
        those versions (`(20..27)`), not to `version != SCHEMA_VERSION`.
      - No `VACUUM` in the migration (see Decisions).
      - Extend the version history doc comment with v27.
      - Fixtures: a fixture faking a version before v20 must stop running
        `DROP TABLE embeddings;` (line 3523), because the table no longer
        exists after `open`. A fixture faking v20-v26 has to create the table
        back to prove the drop. Re-check every migration test's version
        asserts.
      - Replace `a_v19_database_gains_the_embeddings_table_...` with a test
        that a v26 database holding embeddings rows opens at v27 without the
        table and keeps `files`, `ratings`, `views`, `folders` and
        `folder_counts`.
    - `RANK_DIRS` is also the eviction's spare rule. Keep the SQL, rename the
      constant and reword its doc and `evict`'s (no "ranking"). Drop the
      public `rank_dirs` and its test. The eviction tests near lines 6512-6563
      assert through `index.embeddings(..)` and `rank_dirs()`; rewrite those
      assertions on rows that remain.
    - Remove `embeddings` from the table lists of `reconcile`, `evict_folder`,
      `clear` and the rename / split carry loops (lines 1644-1753), and from
      their docs. Remove the embedding tests (lines 4144-4460) and the
      embedding parts of the rename / split tests (near lines 6346-6460).
    - The `views` table, `mark_viewed` and `set_last_viewed` stay. Only reword
      comments that say `views` feeds the ranking.
    - `ai.rs` imports `index::with_suffix`. Check whether it has another user
      and remove it only if this step orphans it.
    - `crates/engine/Cargo.toml`: remove `reqwest` and `rustls`. `sha2` stays
      (`update.rs`). Check the dev-dependencies for ones only `ai.rs`'s tests
      used.
    - Regenerate `Cargo.lock` and `crates/app/Cargo.lock` with
      `cargo metadata` in each workspace, then verify with
      `cargo metadata --locked`. Never edit a lock file as text.
    - Leave `handpic_core::scan::embed_all` and the core modules in place;
      Step 4 removes them. They stay compiled as public items.
    - Docs:
      - `docs/agents/shared-crates.md`: delete the section "The third pass:
        `embeddings` is its own table, and the pass is not "scanning"". Fix
        the table list at line 71, the embedding-pass remark near line 364,
        `embed_threads()` near line 812 and `rank_rows` near line 1476.
      - `docs/agents/app.md` lines 743-754: the `ai::Ai` seam and the
        `handpic_engine::ai` log target.
      - `CLAUDE.md`, the engine paragraphs: the `views` clause ("which gives
        the ranking its negatives"), "filled in three passes" and the
        `run_embed_scan` clause, "the second and third lower still", and the
        `src/ai.rs` clause.

- [ ] Step 4: Remove the core's embedding, genre and ranking modules, their
      data and the CLI's embedding commands
  - Done when:
    - `crates/core/src/embed.rs`, `genre.rs`, `rank.rs`,
      `crates/core/genres/`, `crates/core/ranking/` and
      `crates/core/models/LICENSE-siglip2` are gone.
    - `scan::embed_all` is gone.
    - `handpic-cli` has no `embed` or `embed-compare` command.
    - `tools/package/notices.sh` no longer ships `LICENSE-siglip2`.
    - The full-size eye sharpness (`fullsharp.rs`) gives the same values as
      before.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 3 is merged (the engine imported `embed`, `rank` and
      `embed_all`).
    - `crates/core/src/fullsharp.rs` uses `embed::{resample, Filter}`
      (Bicubic, one channel). `resample`, `coefficients` and the test
      `coefficients_match_pil_on_a_downscale` must survive, moved verbatim.
      - Decide the new home: private in `fullsharp.rs`, or a small
        `resample.rs`.
      - If `Filter::Bilinear` ends up unused in a private enum, clippy's dead
        code check fails; either keep the enum public in its own module or
        drop the unused variant.
      - The resampling output must not change. The `full_eye_sharp` values in
        the index are not versioned for it, so there is no `FACES_VERSION`
        bump; the existing `fullsharp` tests must pass unchanged.
    - `crates/core/src/lib.rs`: remove the three `pub mod` lines.
      `crates/core/src/scan.rs`: remove `embed_all` and the `embed` import.
      `Priority::Lowest` stays (pass 2 uses it).
    - `decode::decode_upright_near` and `faces::upright_rgb` stay (`faces.rs`,
      `decode.rs`, the CLI and the engine use them).
    - `crates/core/Cargo.toml`: no dependency is expected to go (`serde` /
      `serde_json` by `i18n.rs`, `chrono` by `sequence.rs`, `tract-onnx` by
      the face models). Verify, and regenerate the lock files with
      `cargo metadata` only if something does.
    - `crates/cli/src/main.rs`:
      - Remove `embed_cmd`, `embed_compare`, `embed_line`, `parse_embeddings`,
        `compare_embeddings`, the `EMBED_*` constants, their usage lines and
        the test `embed_lines_round_trip_and_compare_by_path`.
      - Remove helpers only they used (check `working_set` and similar).
      - `previews` stays: it only decodes. Reword its doc comment ("the
        embedding's input").
    - Docs:
      - `docs/agents/tract-onnx-inference.md`: the intro's `embed.rs` mention
        and the section "The image embedding: RGB NCHW, PIL's resampling, a
        DCT-scaled decode". Keep the general tract findings from the candidate
        survey (quantized exports, the load-from-path memory peak), reworded
        without the removed symbols, with their archived sources.
      - `docs/agents/shared-crates.md` near line 1065 if Step 3 left it.
      - `docs/agents/app.md` line 325: the notices list.
      - `CLAUDE.md`: the core `src/embed.rs`, `src/genre.rs` and `src/rank.rs`
        clauses.

- [ ] Step 5: Clean up `todo.md` and sweep the remaining mentions
  - Done when:
    - `todo.md` no longer holds these items:
      - "Core: the ranking's pull toward the prior is untested for one to
        three culled folders"
      - "Core: the ranking's reject weight and confidence levels are choices,
        not fits"
      - "Core: portrait bursts lean negative under the per-genre ranking
        offsets"
      - "Engine: the AI model's footprint (resident plan, kept 378 MB file,
        190 MB fp16 option)"
      - "Core: soccer and macro genres need labeled positives to come back"
      - "Core: keep or remove the ignored `detector_tract.rs` test left by
        ball-game-ranking-features"
      - "Core: a plan to drop the genre from the personal ranking (scope
        decision pending)"
      - "Core: the shipped ranking weights score single frames at chance"
      - "Engine: decide the fate of the engine code only the Tauri app
        reached"
      - "Core: decide whether `G0` as-is at per-folder q5-q10, sorted last, is
        worth shipping"
      - "Core: re-run the reject-filter evaluation once enough in-app rejects
        and views exist"
    - The item "Core: test the scan's read slot in `embed_all` and the
      post-read cancel check" keeps only its post-read cancel part, retitled.
    - "Core: decide whether a bad-photo cue shown to the human replaces the
      removal filter", "Core: decide whether a second bad-photo labeling round
      for the cue readings is wanted" and "Agents: the archived plan scripts'
      import paths broke when their folders moved" stay.
    - `rg -i 'embed|genre|rank_|siglip|aiEnabled'` outside `docs/plans/` and
      `CHANGELOG.md` returns only unrelated hits (embedded previews and
      resources, `include_str!`, the index's migration history and its v27
      drop).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Steps 1 to 4 are merged.
    - Run the acceptance grep first and fix what the earlier steps left.
      Known leftovers:
      - `docs/agents/app.md` near line 2064: the rule "no AI toggle ... a
        seeded `aiEnabled` key stays in the store, unread".
      - `docs/agents/app.md` near line 4645: the latency-gate rule's "AI (pass
        3 embeddings, genres, the ranking, the face and eye models)".
      - `tools/ci/local.test.sh` line 79: the fixture path
        `crates/core/genres/old.py`. Replace it with another path the
        classifier treats the same way; read `tools/ci/local.sh`'s rules
        first so the expectation does not change.
    - Delete `crates/core/tests/detector_tract.rs` together with its todo item
      (see Decisions), and reword the mention in
      `docs/agents/tract-onnx-inference.md` if it names the file.
    - Check each remaining todo item that lists `crates/core/src/rank.rs`,
      `genre.rs`, `embed.rs`, `ai.rs` or `crates/core/genres/` under Files and
      fix or drop the reference.
    - README.md and README.ja.md mention no AI feature (only "an AI assistant"
      as an MCP client), so no change is expected; confirm.

## Decisions

The user approved these with the plan (2026-11-21):

- **The acceptance grep's scope.** Hits in `docs/plans/` (archived and live
  plans, `review-history/`), `CHANGELOG.md` (generated by release-please) and
  the index's migration history (the v27 drop, its version history comment and
  test) stay as history.
- **No `VACUUM` when dropping `embeddings`.** The pages go to the freelist and
  return at the next eviction `VACUUM` or Clear Cache. The size cap counts
  pages in use, so it is unaffected. A `VACUUM` in the migration would hold the
  writer lock at launch (about 220 ms per 100 MB) for a one-time gain.
- **The downloaded model file and the stored `aiEnabled` key stay.** A machine
  that ran the Tauri app with AI on may hold the 378 MB `vision_model.onnx`
  under that app's data directory; no code knows its path after Step 3. The
  settings store is never rewritten for retired keys, the convention
  `docs/agents/app.md` already records.
- **A stored `sortOrder` of `recommended` keeps mapping to capture order**, as
  a rule for a legacy value, so a store seeded from the Tauri app keeps opening
  in capture order.
- **The seven-tool MCP mode goes (Step 2)**, as its own PR so it can be
  reverted alone. Only the engine's own tests use it.
- **The MCP tool list change (Step 1) is not marked breaking.** `refit_ranking`
  disappears (35 to 34 tools) and `recommended`, `genres` and `ai_enabled` stop
  being declared; a client got a refusal for them before.
- **`crates/core/tests/detector_tract.rs` goes** with its todo item: the
  ranking features it served are gone.
- **Todo items about the bad-photo evaluations.** The `G0` shipping item and
  the reject-filter re-run item go, since `G0` is fit on columns that include
  the 32 embedding dimensions the app can no longer produce. The bad-photo cue
  item (sharpness decile or a missing AF face, both still in the index), the
  second labeling round item and the archived plan scripts' import paths item
  stay.
- **The general tract findings from the embedding survey stay** in
  `docs/agents/tract-onnx-inference.md`, reworded, because the guide also
  serves "adding another ONNX model".

## Risks

- **The resampling move (Step 4).** `fullsharp.rs` depends on `embed.rs`'s
  `resample`. A change in its output would shift stored `full_eye_sharp` values
  and the `Face focus` filter's `Low` tier without a version bump. The step
  moves the code verbatim and relies on the existing tests.
- **The migration fixtures (Step 3).** Every fixture that fakes an older schema
  is built from a fresh `open`, so removing a table changes what each must drop
  or re-create. The guide's audit of every guard and fixture on a bump applies
  in full.

## Progress

- Step 1 (2026-11-21): removed `refit_ranking` (34 tools), `SortBy::Recommended`,
  `FilterArgs.genres` and `SettingsArgs.ai_enabled` from the engine's MCP
  server, and the four `not_in_app` features and their Coverage rows from the
  app's companion; the docs name only compare and file rename as refused.
- Step 2 (2026-11-21): `Mcp::new` now lists every tool with the one
  `INSTRUCTIONS` (the former `NATIVE_INSTRUCTIONS`); `Mcp::with_all_tools`,
  `Companion::seven` and the Tauri instructions are gone, and
  `with_screenshot` still adds `screenshot` only when called.
