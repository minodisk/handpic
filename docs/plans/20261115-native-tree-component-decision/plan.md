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

# Record the decision not to adopt gpui-component's Tree

## Purpose

The native app's folder tree (`crates/native`: `main.rs`'s `tree_pane` /
`tree_row`, `tree.rs`, `treekeys.rs`) is hand-built on a `uniform_list`.
gpui-component 0.9.1 ships a `Tree`, so the question of replacing ours with
it will come up again. It was examined against the 0.9.1 sources on
2026-11-15 and declined. This plan writes that decision and its reasons into
`docs/agents/native-app.md`, so a later agent does not redo the investigation
or start the replacement. No code changes.

## Steps

- [x] Step 1: Add the "not replaced by gpui-component's `Tree`" section to
      `docs/agents/native-app.md`
  - Done when:
    - A new `###` section sits directly after "### The folder tree is a
      `uniform_list` over rows kept in the view" and before "### The panes
      are one `Panels` value, stored under `panels`".
    - The section states the decision (the folder tree is not replaced by
      gpui-component 0.9.1's `Tree`, nor by gpui-base's unstyled `Tree`),
      the reasons listed below, what the component would have given, and
      what stays possible later.
    - The "gpui-component is initialized; ..." section carries the pointer
      described below.
    - `mise run ci:local` passes locally (a changed `.md` selects lychee)
      and the PR's GitHub checks pass.
  - Implementation approach:
    - Only `docs/agents/native-app.md` changes (plus this plan's files). No
      file under `crates/` changes, and `CLAUDE.md`'s Layout paragraph does
      not need an edit: no module is added or changes its role.
    - Follow the file's section style: a `###` title that states the fact,
      tagged `(Inferred)` (the reasons come from reading sources only;
      nothing was built or measured), a short opening paragraph, then the
      reasons as bullets, with `- Rule:` bullets for what a later change
      must do. A possible title: "The folder tree is not gpui-component's
      `Tree` (Inferred)". Other sections are referred to by their quoted
      title, not by a Markdown link; do the same.
    - Name the sources once, with the date: `gpui-base` 0.9.1's
      `src/tree.rs` (`TreeItem`, `TreeState`, the unstyled `Tree`) and
      `gpui-component` 0.9.1's `src/tree.rs` (the styled wrapper), read on
      2026-11-15. The crates are in the cargo registry
      (`~/.cargo/registry/src/*/gpui-base-0.9.1`,
      `~/.cargo/registry/src/*/gpui-component-0.9.1`); re-read a claim there
      before writing it if in doubt.
    - Reasons to state, each verified in those sources during planning:
      - `TreeItem::is_folder()` is `!self.children.is_empty()`. A folder
        not listed yet has no children, so it would need a dummy child to
        get an expander and to be toggled at all (`toggle_expand` returns
        early on a non-folder). Filling the children after a lazy listing
        or a disk change means `TreeState::set_items`, which sets both
        `selected_ix` and `right_clicked_ix` to `None`.
      - A full click on a row always selects it and toggles its expansion
        (`on_entry_click`: `selected_ix = Some(ix)`, then `toggle_expand`).
        Ours opens the folder on a click on the name and toggles only on
        the expander, whose `on_click` stops propagation.
      - It has its own `FocusHandle` (`track_focus`) and the `"Tree"` key
        context. The native app keeps one window focus and routes keys
        through the `tree_keys` flag and `treekeys::tree_gate` (see "The
        tree takes the keyboard through one flag in the view").
      - Its keys differ. `up` / `down` wrap at the ends (`on_action_up`
        goes from the first row to the last, `on_action_down` from the last
        to the first); ours stop (`tree::step`). `left` only collapses an
        expanded folder and does nothing on a collapsed row, so it never
        moves to the parent. There is no Home / End and no type-ahead.
        `Confirm` toggles the selected folder instead of opening it, and
        `gpui-base`'s `tree::init` binds only the four arrows in the
        `"Tree"` context, so Enter does nothing there unless the app binds
        it.
      - It tracks one selected index and one right-clicked index. The
        folder tree needs the keyboard cursor and the open folder's row
        apart.
      - The styled wrapper's `context_menu` builds a gpui-component
        `PopupMenu`, not `context.rs`'s items (checked, disabled, shortcut)
        the strip's menu shares. "The sort and filter menus are deferred
        popovers over the window" already says why `PopupMenu` is not used;
        point to it rather than repeating it.
      - A `TreeItem` holds its whole subtree (`children: Vec<TreeItem>`)
        and its expansion in an `Rc<RefCell<..>>`. That state would
        duplicate `tree.rs`'s `Tree`, which must stay for the lazy listing,
        the failed listings, `respell` and the rename.
    - State what it would give: virtualization and the scrollbar, which the
      tree already has (`uniform_list` plus `vertical_scrollbar`, #1068, see
      "The strip, the tree and the meta pane draw the theme's scrollbar"),
      and gpui-base's accessibility roles (`Role::Tree`, `Role::TreeItem`,
      `aria_selected`, `aria_expanded`).
    - State what was also declined: gpui-base's unstyled `Tree`, which has
      the same click behavior, focus handle and key context (the styled one
      wraps it).
    - State what stays possible and is not adopted: drawing only the rows
      with `gpui_component::list::ListItem` (the theme's list colors), as a
      small later step.
    - A fitting `- Rule:` bullet: a `gpui-component` version bump that
      prompts a second look re-reads the points above in both crates'
      `src/tree.rs` before reopening the decision; the listed behavior is
      0.9.1's.
    - Pointer: in "gpui-component is initialized; the settings modal and
      the tree's inline rename draw from it", add one sentence after the
      list of the components drawn, saying that the folder tree is not the
      component's `Tree` and naming the new section by its quoted title.
      Do not touch the archived plan
      (`docs/plans/_archived/20261114-gpui-component-adoption/`).
    - English only. Wrap lines as the file does (about 78 columns).

## Trade-offs and risks

- **Enter.** In `gpui-base` 0.9.1, `tree::init` binds only `up`, `down`,
  `left` and `right` in the `"Tree"` context; `on_action_confirm` is
  registered and toggles a folder, so it runs only if the app binds a key to
  `Confirm` in that context. The section words the reason that way (the
  user's first report said Enter toggles; corrected with the user).
- **Tag.** `(Inferred)` is used because every reason comes from reading
  sources. Nothing was built against the component, so behavior that only
  shows at run time (focus hand-over on a click, how the `"Tree"` context
  interacts with the app view's context) is not verified.

## Progress
