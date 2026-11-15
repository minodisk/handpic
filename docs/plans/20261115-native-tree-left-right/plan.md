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

# Native folder tree: Left / Right page the strip, Space toggles a folder

## Purpose

In the GPUI native app (`crates/native`), the folder tree takes `ArrowLeft` /
`ArrowRight` while it has the keyboard (expand / collapse / step into the
first child / step up to the parent, the WAI-ARIA tree pattern). The user
wants those two keys to keep moving between the strip's thumbnails whether or
not the tree has the keyboard. After this work, with the tree holding the
keyboard: Left / Right move the strip's current file, Up / Down / Home / End
still move the tree cursor, `Space` expands or collapses the cursor's folder,
and `Enter` still opens it. The Tauri app (`crates/app`) is not changed.

## Decisions (made with the user)

- Native app only; the Tauri app keeps its Left / Right tree keys.
- Passthrough is keyed by action name (`previous` / `next` join
  `TREE_PASSTHROUGH`), matching the existing gate.
- Plain Space always toggles and is never a type-ahead character (type-ahead
  can no longer go past a space in a folder name).
- Modified arrows (`shift+arrow`, `alt+arrow`) stay swallowed while the tree
  has the keyboard.
- User-facing docs (`docs/humans/usage.md`, README) describe the Tauri app and
  are left unchanged.

## Steps

- [x] Step 1: Let `previous` / `next` through the tree gate and move expand /
      collapse to Space (native app only)
  - Done when:
    - With `tree_keys` on, `arrowleft` / `arrowright` are no longer consumed
      by `tree_key_down`, and the `previous` / `next` actions run
      (`go(Move::Previous | Move::Next)`); the tree keeps the keyboard
      afterwards (`tree_keys` stays true, the cursor ring stays drawn), so
      Up / Down / Home / End still move the tree cursor.
    - Plain `space` while the tree has the keyboard toggles the cursor row's
      expansion: expands a collapsed expandable row, collapses an expanded
      expandable row, and does nothing on a row that is not expandable or
      when the cursor is not among the drawn rows. It is consumed by the tree
      either way (never type-ahead, never a keymap action).
    - `Enter` still opens the cursor's folder.
    - `TreeKey::Left` / `TreeKey::Right`, their arms in `tree_key`, their
      tests and the now-unused `TreeCommand::Focus` are gone.
    - New tests in `tree.rs` cover the Space toggle (collapsed expandable ->
      `Expand`, expanded -> `Collapse`, expanded and still listing ->
      `Collapse`, leaf -> `None`, expanded leaf -> `None`, missing or stale
      cursor -> `None`); tests in `treekeys.rs` cover `previous` / `next`
      passing the gate and the other culling actions still swallowed.
    - `CLAUDE.md` and `docs/agents/native-app.md` describe the new keys and
      state that this departs from the Tauri app.
    - `mise run ci:local` passes locally (it runs `fmt native`,
      `clippy native` and `test native` for a `crates/native` change) and
      the PR's GitHub checks pass.
  - Implementation approach:
    - Routing as it is today: the tree's keyboard is the `tree_keys` flag on
      the view, not GPUI focus. `key_down` in `crates/native/src/main.rs`
      calls `tree_key_down` first and, when that returns false, runs the
      key's action only if `treekeys::tree_gate(action)` is `Gate::Run`.
      Both places stop Left / Right today: `tree_key_down` maps
      `arrowleft` / `arrowright` to `TreeKey`, and `TREE_PASSTHROUGH` lacks
      `previous` / `next`.
    - `crates/native/src/treekeys.rs`: add `"previous"` and `"next"` to
      `TREE_PASSTHROUGH`. Keep the gate keyed by action name. Update the
      module doc (it now also lets the strip's previous / next through and
      differs from `treekeys.ts` there). Update the tests
      `lets_only_the_window_and_folder_actions_through` (expected list and
      name) and `swallows_the_culling_actions` (which lists `"next"` today).
    - `crates/native/src/tree.rs`: `TreeKey` becomes `Space` / `Enter`.
      `tree_key` for `Space`: `None` when `!row.expandable`, else
      `Collapse(path)` when `row.expanded`, else `Expand(path)`. Remove
      `TreeCommand::Focus`. Update `tree_key`'s doc comment. Replace the
      `right_*` / `left_*` tests with the Space tests, reusing the `keyed`,
      `collapsed` and `expanded` helpers; update
      `a_missing_or_stale_cursor_does_nothing` to iterate `Space` / `Enter`.
    - `crates/native/src/main.rs`, `tree_key_down`: drop the `arrowleft` /
      `arrowright` arms and the `TreeCommand::Focus` arm; map `"space"` to
      `TreeKey::Space` before the `keys::typed_char` branch. `shift+space`
      is left as it is. Update the doc comments that name the keys.
    - Nothing needs to be done to keep the tree's keyboard: `go` does not
      touch `tree_keys`.
    - Space has no default binding in `crates/backend/src/shortcuts.rs`; the
      tree takes it first while it has the keyboard, so the keymap and the
      backend need no change.
    - Docs:
      - `CLAUDE.md`, the `src/tree.rs` description: "`tree_key`'s Left /
        Right / Enter" becomes Space / Enter; the `src/treekeys.rs`
        description: add the strip's previous / next and soften "a port of
        `treekeys.ts`".
      - `docs/agents/native-app.md`, section "The tree takes the keyboard
        through one flag in the view": what `tree_key_down` takes (Up / Down,
        `home` / `end`, `enter`, `space`, type-ahead) and what `tree_gate`
        lets through (now also `previous` / `next`); state that this differs
        from the Tauri app on purpose (the user's request) and that Space is
        no longer a type-ahead character.
      - `docs/agents/native-app.md`, the settings rule listing "the folder
        tree's keys (arrows, Home / End, Enter, Escape, type-ahead)": becomes
        Up / Down, Home / End, Enter, Space, Escape, type-ahead.
      - `docs/agents/native-app.md`, the comparison with gpui-component's
        `Tree`: reword so it no longer implies ours handles Left / Right.
    - Not verifiable headless: the running app's behavior (Left / Right
      paging with the ring still on the tree cursor, Space toggling). Record
      it as a pending hand check in `learnings.md`.

## Trade-offs and risks

- If the user rebinds `previous` / `next`, the new keys page from the tree
  (unless printable, which type-ahead takes first), and a Left / Right
  rebound to another action is swallowed.
- Keyboard moves to the parent / first child are gone; the cursor reaches
  them only through Up / Down, Home / End and type-ahead.
- `tree.rs` and `treekeys.rs` are ports of `tree.ts` / `treekeys.ts`; the
  module docs and `native-app.md` must record the divergence so a later
  parity pass does not revert it.

## Progress
