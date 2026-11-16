# Learnings: native-os-title-bar

## Step 1

- The revert was done by hand, not with `git revert 75cedfd8`: the window
  options in `main()` now use `..Default::default()` for both
  `TitlebarOptions` and `WindowOptions`, and `h_flex`, `img`,
  `WindowDecorations`, `TitleBar` and `TITLE_BAR_HEIGHT` were the only
  imports left unused.
- `menu_max` is now `viewport height - paddings - 40 px`; on Windows the
  paddings are zero, so it equals the pre-#1100 cap.
- `theme.rs` still pins `title_bar` / `title_bar_border`; its comment no
  longer says the app draws a `TitleBar` (worded without the type name so the
  plan's grep over `crates/native/src` stays empty).
- The strip's two `on_mouse_down` comments now say what the subtraction is
  for (`Root`'s padding under client-side decorations) rather than naming the
  removed client-side request.

## Deferred issues (todo candidates)

- **Pending manual check (Windows, `mise run native:dev`)**: the agent could
  not drive the GUI. Launch the app and confirm the OS title bar shows with
  the title "Handpic" and no app-drawn bar; the preview, tree, meta pane,
  strip and status row fill the client area from its top; the window moves,
  maximizes / restores, minimizes and closes normally; the taskbar and
  Alt+Tab read "Handpic"; a long Sort / Filter menu stops 40 px short of the
  window's height. Step 1's checkbox was ticked on the automated criteria.
  Source: plan Step 1's last Done-when; files `crates/native/src/main.rs`.
- **todo.md cleanup at wrap-up**: the section "Native: real-device checks of
  the title bar (native-title-bar)" is obsolete after this revert, and the
  native-themes items that list "the `TitleBar`" among the surfaces to check
  need that word dropped. Source: the plan's Trade-offs ("`todo.md` is left
  to the wrap-up"); file `todo.md`.
