# Learnings

## Step 1

- Item 1: the guard is a pure `menu::enabled(which, folder_open, view_only)`
  that `strip_header` feeds `menu_button`'s existing `enabled` flag with, so
  the no-folder case reuses the view-only sort's dimmed, listener-free look.
  Pinned by `menu.rs`'s `neither_button_takes_a_press_with_no_folder`; no UI
  test simulates a click on the button.
- Item 2: `status_row` always adds a second `StatusBar::right` child: the
  circle, or `div().size_3().flex_none()` (the circle's `Size::Small` is
  `size_3` in gpui-component 0.9.1). No test; the rule is in `app.md`.
- Item 3: GPUI's `LineWrapper::truncate_line(.., TruncateFrom::Middle)`
  through `cx.text_system().line_wrapper(font, size)` does the shortening
  (`main.rs`'s `fitted_name`), so no truncation code of our own. The test
  platform's `NoopTextSystem` gives every ASCII glyph an advance of 0.6 em
  (6.6 px at 11 px), a usable monospace model, so the rule is pinned through
  `Ui` in `uitest.rs` (`a_strip_name_wider_than_its_cell_is_shortened_in_the_middle`).
  `truncate_line` sums per-character widths and ignores kerning, so a 2 px
  inset per side (`NAME_INSET`) is left; whether it is needed (or enough) on
  real fonts is unverified.
- No Windows by-eye check was run in this step, so items 2 and 3 (and the
  dimmed buttons of item 1) stay Inferred in `app.md`.

## Deferred issues (todo candidates)

- Pending manual check (Windows 11, real device; the step's checkbox was
  ticked on the automated criteria). Basis: the plan's Step 1 Done-when,
  items 2 and 3 being visual; implementation in `crates/app/src/main.rs`
  (`status_row`, `fitted_name`, `strip_header`). Steps: run a debug build
  with a fresh `--index` and an explicit `--settings` test file on a copy
  under `D:\Photos\tests\` holding long file names (e.g.
  `DSC-RX100M2_DSC00416.ARW`).
  Expected: (1) with no folder open, `Sort ▾` and `Filter ▾` are dimmed and
  a click opens no menu; (2) the position text (`1 / 600`) does not move
  when the progress circle appears (cold scan) or goes; (3) a long name in a
  strip cell reads like `DSC-RX100M2_D…416.ARW`, fully inside the cell with
  no glyph clipped at either edge, at the default strip height and after
  dragging the strip shorter and taller; a short name is centered and
  unchanged. Then mark the three rules Verified in `docs/agents/app.md`.
