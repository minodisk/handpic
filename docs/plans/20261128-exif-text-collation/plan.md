<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Order the filter's `Camera` / `Lens` items like ICU root collation

## Purpose

The filter menu's `Camera` and `Lens` sections are ordered by
`crates/app/src/exif.rs`'s `compare`, whose `Order::Text` arm is a
`to_lowercase` compare with a plain compare as the tie-break. The Tauri app
used `localeCompare` (ICU root collation), and `todo.md` holds the open item
"App: `Camera` / `Lens` order is a case-insensitive compare, not the Tauri
`localeCompare`" asking for the two to be compared.

The comparison is done (see "Findings" below): the two orders differ on real
camera and lens names, in punctuation only. This plan makes the `Order::Text`
arm follow ICU root collation closely with a small rule of its own, without
the `icu_collator` crate and its collation data, and closes the todo item.

### Findings (made before this plan; do not redo)

- Source of names: exiftool, read-only, over
  `D:\Photos\samples\{ARW,CR3,NEF,RAF,ORF,DNG}` (2647 files): 554 distinct
  `Camera` names ("Make Model", joined as the engine's `exif::exif` joins
  them) and 348 distinct `Lens` names.
- Reference order: Node's `Intl.Collator` (default locale ja-JP; the same
  order as `en` for these names).
- The current compare differs from it on real names in two places:
  - `NIKON CORPORATION NIKON Z5_2` must come before
    `NIKON CORPORATION NIKON Z50_2` (`_` sorts before the digits).
  - `24-70mm F2.8 DG DN | Art 019` must come before
    `24-70mm F2.8 DG DN II | Art 024` (`|` sorts before the letters).
- No real name holds a non-ASCII character.
- The rule below was prototyped in JavaScript: it gives `localeCompare`'s
  order on all the real names, and on an artificial list except for `Ø`.

## Steps

- [x] Step 1: Replace the `Order::Text` compare with the ICU-root-like rule,
      document it and delete the todo item
  - Done when:
    - `compare`'s `Order::Text` arm in `crates/app/src/exif.rs` orders by the
      rule under "Implementation approach"; the `Order::Number` arm and the
      number-before-text arms are unchanged.
    - New unit tests in `crates/app/src/exif.rs` assert that sorting each of
      these lists (given shuffled or reversed) yields exactly this order,
      which is `localeCompare`'s:
      - `["a b","a_b","a-b","a:b","a.b","a(b","a)b","a/b","a&b","a+b","a|b","a1","ab","aB","Ab","AB"]`
      - `["e","E","é","É","è","ê","ë","f"]`
      - `["resume","Resume","résume","résumé"]`
      - `["Z5","Z5 II","Z5_2","Z50","Z50_2","Z5II"]`
      - `NIKON CORPORATION NIKON Z5_2` before `NIKON CORPORATION NIKON Z50_2`
      - `24-70mm F2.8 DG DN | Art 019` before `24-70mm F2.8 DG DN II | Art 024`
    - The existing tests of the module (among them
      `exif_key_orders_camera_and_lens_by_their_text` and
      `groups_come_in_menu_order_each_sorted`) pass unchanged.
    - `crates/app/Cargo.toml` has `icu_normalizer` as a direct dependency and
      the diff of `crates/app/Cargo.lock` adds no `[[package]]` entry (only
      `icu_normalizer` in `handpic-app`'s dependency list).
    - `docs/agents/app.md`'s filter-menu rule (the sentence "`Camera` and
      `Lens` by a case-insensitive compare, ties by the plain one (Tauri used
      `localeCompare`; no collator here)", around line 2188) describes the new
      rule, how it was compared (exiftool over the samples, Node
      `Intl.Collator`, the two real differences, no non-ASCII real name) and
      the residual (`Ø`, `ß`, `Æ`, `Ł` after `z`).
    - The `todo.md` section "### App: `Camera` / `Lens` order is a
      case-insensitive compare, not the Tauri `localeCompare`" (heading, body
      and its `#### TODO`) is deleted and nothing else in `todo.md` is touched.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/exif.rs`, `crates/app/Cargo.toml`,
      `crates/app/Cargo.lock`, `docs/agents/app.md`, `todo.md`, this plan.
      `CLAUDE.md` does not mention this compare (it only says `src/exif.rs`
      builds the per-folder EXIF groups), so it needs no change; the
      `docs/humans` usage guide does not state the order either.
    - Dependency: `icu_normalizer` 2.4.0 is already in `crates/app/Cargo.lock`
      and linked (through `url` → `idna` → `idna_adapter`, which turns on its
      `compiled_data` feature with `default-features = false`). Declare it as
      `icu_normalizer = { version = "2", default-features = false, features = ["compiled_data"] }`:
      the crate's default features also turn on `utf16_iter`, whose
      `utf16_iter` and `write16` crates are not in the lock today, so plain
      `icu_normalizer = "2"` would add packages. Check the lock diff after the
      edit. `crates/app` is its own workspace; the root `Cargo.lock` is not
      affected.
    - API (seen in the registry source of 2.4.0, `src/lib.rs`):
      `DecomposingNormalizerBorrowed::new_nfd()` is a `const fn` under
      `compiled_data` and `normalize(&self, text: &str) -> Cow<str>` needs no
      further feature. Confirm against the source while implementing.
    - The rule, for each of the two strings:
      1. NFD the string.
      2. Walk its chars. A combining mark in U+0300..=U+036F contributes
         only a secondary weight: its index + 1 in
         `[U+0301, U+0300, U+0306, U+0302, U+030C, U+030A, U+0308, U+030B, U+0303, U+0307, U+0327, U+0328, U+0304]`
         (ICU's order), else `0x100 + code point`. Any other char
         contributes a primary: the index of its lowercase in
         `" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz"`
         (ICU root's order of printable ASCII), else
         `0x10000 + lowercase code point`; a secondary of 0; and a tertiary
         of 0 when the char is its own lowercase, else 1.
      3. Compare the whole primary sequences lexicographically, then the
         whole secondary sequences, then the whole tertiary sequences
         (lowercase first), then the plain `str` compare.
    - `char::to_lowercase` can yield more than one char (for example `İ`);
      decide how such a char maps to one primary (for example its first
      lowercase char) and cover it with the comment, since the prototype used
      JavaScript's `toLowerCase` on single chars. No real name hits this.
    - Keep it a private helper next to `compare` (a sort key or a compare of
      two `&str`); `found.sort_by(compare)` sorts a few hundred labels at
      most, so no caching is needed.
    - In `docs/agents/app.md`, keep the paragraph's `Rule:` style, state that
      this is an approximation of ICU root collation and not a collator, and
      that `icu_collator` is the next step only if a real name is found that
      this still orders differently.
    - `todo.md` is edited by other sessions: remove only that one section and
      rebase rather than overwrite on a conflict.
    - Conventional Commit, for example
      `fix(app): order the filter's Camera and Lens items like ICU root collation`.

## Trade-offs and risks

- `icu_collator` not taken (decision delegated, the recommended option): it
  would give the exact `localeCompare` order, including the residual below,
  but brings the ICU collation data into `crates/app`. The light rule matches
  on every real name of the samples; `icu_collator` is the fallback if a real
  name still orders differently.
- Known residual: letters with no canonical decomposition (`Ø`, `ß`, `Æ`,
  `Ł`) sort after `z` instead of by their base letter. No camera or lens name
  of the samples holds one.
- Scope of verification: the rule's tables were verified for printable ASCII
  and the thirteen listed combining marks. Other scripts (for example CJK or
  Cyrillic names) fall back to code point order after ASCII, which is not
  ICU's order; none occur in the samples.
- The secondary and tertiary levels are compared as whole sequences, a
  simplification of ICU's per-level comparison that agrees on the tested
  lists; an untested accent / case combination could still differ.
- Declaring `icu_normalizer` with default features would add `utf16_iter` and
  `write16` to the lock; the plan declares it with `compiled_data` only. If
  that set does not build, the implementer chooses between adding
  `utf8_iter` (already in the lock) and accepting new packages, and records it.

## Progress
