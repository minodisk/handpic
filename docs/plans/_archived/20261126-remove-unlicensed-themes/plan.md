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

# Remove the unlicensed shipped themes

## Purpose

The app ships 21 theme files vendored from gpui-kit (Apache-2.0), each adapting
a third-party color scheme. The 2026-11-23 license check
(`crates/app/themes/LICENSE`) could not determine the terms of three of them,
and found two under Creative Commons Attribution-ShareAlike 4.0. The decision
recorded in `todo.md` on 2026-11-23 is:

- remove `alduin.json`, `adventure.json` and `kibble.json`;
- keep `harper.json` and `fahrenheit.json` under share-alike with attribution.

This plan carries that decision out and closes both items of the `todo.md`
section "App: remove the three shipped themes with no license and keep the two
CC BY-SA 4.0 themes with attribution (native-themes)". After it, every theme
the app ships has a known license whose notice the packages carry. It lands as
a normal PR in the next regular release, not a hotfix.

Background: [the native-themes plan](../20261115-native-themes/plan.md).

## Steps

- [x] Step 1: Remove the three themes, state the two CC BY-SA 4.0 themes'
      terms, and close the todo
  - Done when:
    - `crates/app/themes/alduin.json`, `adventure.json` and `kibble.json` are
      deleted and `theme.rs`'s `FILES` embeds the remaining 18 files.
    - `crates/app/themes/LICENSE` has no row, note or mention of Alduin,
      Adventure, Adventure Time or Kibble, and its notes are numbered without
      gaps.
    - That `LICENSE` gives, for Harper and for Fahrenheit, the author, the
      source URL, the license name "Creative Commons Attribution-ShareAlike
      4.0 International" with `https://creativecommons.org/licenses/by-sa/4.0/`,
      and what was changed from the original. It says they are kept under
      share-alike with attribution, and no "open decision" wording remains.
    - `docs/agents/app.md` says 18 theme files, has no "Not settled" /
      "undecided" passage, and describes the two CC BY-SA 4.0 themes as kept
      with attribution and the three as removed.
    - A test in `theme.rs` shows that a stored name of a removed theme
      (`"Alduin"`, `"Adventure"`, `"Adventure Time"`, `"Kibble"`) reads as
      `NEUTRAL`, and that `names` of either mode offers none of them.
    - `grep -rniE "alduin|kibble|adventure" crates/ docs/agents docs/humans README.md README.ja.md CLAUDE.md tools`
      matches only that test, and the sentences that record the removal, if
      they name the three.
    - The `todo.md` section "App: remove the three shipped themes with no
      license and keep the two CC BY-SA 4.0 themes with attribution
      (native-themes)" is deleted whole (heading, body, `Files:` and both
      TODO items), and no other `todo.md` item is touched.
    - `README.md` and `README.ja.md` name CC BY-SA 4.0 for Harper and
      Fahrenheit on their themes line, in sync (see Trade-offs 1).
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - **`crates/app/src/theme.rs`**
      - Change `const FILES: [&str; 21]` to `18` and drop the three
        `include_str!` lines.
      - Change `assert_eq!(FILES.len(), 21)` in
        `every_embedded_file_is_a_theme_set` to `18`.
      - No other test names a removed theme. The existing ones use Catppuccin,
        Jellybeans, Ayu and Tokyo Night.
      - The fallback needs no code change: `Setting::from_setting` resolves
        each name through `shipped(name, mode)` and uses `NEUTRAL` otherwise.
      - Add the removed names to the existing
        `an_unknown_or_wrong_mode_name_is_neutral` test, or add one small test
        beside it. All four theme names are `Adventure`, `Adventure Time`,
        `Alduin` and `Kibble`; try each in both the `light` and `dark` field.
      - Extend the doc comment on `FILES` ("a bump of the crate copies them
        again") to say the three files are left out on purpose and why, so a
        re-vendoring does not bring them back.
    - **`crates/app/themes/LICENSE`**
      - Remove the three rows from the author / url table and from the
        per-file license table, and notes (1), (2) and (9).
      - Note (9) leaned on note (1) for "the collection is MIT, each theme's
        terms belong to its author". Notes (5) and (7) still cite the
        collection's `CREDITS.md`, so keep that fact where they need it.
      - Renumber the remaining notes and their references in the table.
      - Reword the MIT list's last bullet to "iTerm2-Color-Schemes, the
        collection Fahrenheit and Harper come from".
      - In the two CC BY-SA notes, replace "Whether an adaptation under
        share-alike terms may ship in this app is an open decision." with the
        decision: kept, distributed under CC BY-SA 4.0 with this attribution.
      - Expand the closing "Harper (Jan T. Sott) and Fahrenheit ..." paragraph
        so each theme has all of these:
        - the title;
        - the author: Jan T. Sott (idleberg) for Harper; fcpg for
          vim-fahrenheit, ported to iTerm2 by jonascarpay;
        - the source: `https://github.com/idleberg/Harper-iTerm2` and
          `https://github.com/fcpg/vim-fahrenheit`, both reached through
          `https://github.com/mbadolato/iTerm2-Color-Schemes`;
        - the license name and link;
        - the change made: the palette adapted into a gpui-kit theme file by
          gpui-kit, copied here unchanged;
        - a statement that `harper.json` and `fahrenheit.json` are
          distributed under CC BY-SA 4.0 and not offered under Handpic's
          `MIT OR Apache-2.0` (see Trade-offs 3).
      - Keep the upstream README's own wording ("4.0 Unported") only inside
        the quotation.
      - Adjust the header paragraph, which says the files are under gpui-kit's
        Apache-2.0, so it does not contradict the statement for the two
        files. A sentence there may record that three of gpui-kit's 21 files
        are not copied.
      - `tools/package/notices.sh` ships this file as `LICENSE-themes`
        unchanged, so the packages pick the new text up with no script change.
    - **`docs/agents/app.md`**
      - Line ~94: "gpui-kit's 21 theme files" becomes 18, with a clause that
        three are left out for their undetermined license.
      - Lines ~327-340 ("The license notices are due at packaging"): replace
        "Not settled, and still shipped (2.0.1 already ships all 21):" and its
        two bullets with the settled state. Keep the fact that 2.0.1 shipped
        all 21 only if it reads as history.
    - **`README.md` / `README.ja.md`**: the line "The themes vendored from
      gpui-kit: Apache-2.0, ..." (`README.md` ~466, `README.ja.md` ~202) gains
      the CC BY-SA 4.0 exception for Harper and Fahrenheit, both files in the
      same PR.
    - **`CLAUDE.md`**: its theme description gives no count and names no
      theme, so no change is expected; re-check with the grep.
    - **Left alone**
      - Archived plans, `docs/plans/review-history/**` and `learnings.md`
        files that mention 21 files or the removed themes are history.
      - `companion.rs`, `settingsmodal.rs` and `settingsview.rs` read the
        list through `theme::names`.
      - `todo.md` line ~1145, in the Appearance tab real-device checks item
        (out of scope), which says "21 theme files (36 themes)".
    - **`todo.md`**: delete only the section named above.
    - **Commit and PR**: Conventional Commits, for example
      `fix(app): remove the three themes with no determined license`. The PR
      body carries `Plan:` and `Step: 1`. It is a normal PR to `main`; no
      release is cut for it.

## Trade-offs and risks

1. **README license line.** The plan amends `README.md` and `README.ja.md` to
   name CC BY-SA 4.0 for the two kept themes, because "Apache-2.0" alone is
   no longer the whole truth and attribution should be findable. (Chosen by
   the user on 2026-11-26.)
2. **The "21 theme files (36 themes)" sentence at `todo.md` line ~1145.** It
   belongs to another, out-of-scope item and describes what native-themes
   vendored at the time. Left untouched. Whoever runs those device checks will
   see 32 themes in the lists, not 36.
3. **Which license the two kept files carry.** gpui-kit distributes
   `harper.json` and `fahrenheit.json` under Apache-2.0, but they adapt
   CC BY-SA 4.0 works, and share-alike requires an adaptation to be under the
   same license. The `LICENSE` states that these two files are distributed
   under CC BY-SA 4.0 and are not offered under Handpic's `MIT OR Apache-2.0`.
   The app only reads the files as data, so the rest of the app's license is
   not expected to be affected. (Approved by the user on 2026-11-26.)
4. **A user on a removed theme gets Neutral silently.** The stored `theme`
   value keeps the old name until the user picks again; the `Select` shows
   Neutral. No migration or in-app notice. A `fix(app):` PR title makes the
   removal visible in the release notes.
5. **Re-vendoring.** A bump of `gpui-component` copies the files again, which
   would bring the three back unnoticed. The exclusion is recorded in the
   `FILES` comment and in `docs/agents/app.md`. The `FILES.len()` assertion
   catches a count change but not which files.

## Progress

- 2026-11-26: Step 1 done: the three files removed, `FILES` at 18, the LICENSE attribution for Harper / Fahrenheit, the docs and READMEs updated, the todo section deleted.
