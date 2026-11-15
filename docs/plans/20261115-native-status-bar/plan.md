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

# Native status row on gpui-component's StatusBar

## Purpose

The GPUI app's status row (`crates/native/src/main.rs`, `status_row`) is a
hand-built flex `div` with a hand-drawn 2 px progress bar on its top edge.
gpui-component `=0.9.1`, already a dependency, ships `StatusBar`
(`src/status_bar.rs`) and `ProgressCircle`
(`src/progress/progress_circle.rs`). This plan renders the row through
`StatusBar` with the component's look (a top border, a fill of its own, 12 px
muted status text, its vertical padding) and shows the running operation's
progress as a `ProgressCircle` inside the bar instead of the top-edge bar.
The status text and the `Sort ▾` / `Filter ▾` buttons with their popovers
opening above them keep working.

The user confirmed reversing the rule recorded in
[native-app.md](../../agents/native-app.md), "The status row stays a plain
`div`, not gpui-component's `StatusBar`" (PR #1072), chose the component's
defaults over keeping today's look, and chose the circle over the top-edge
bar. This plan rewrites both sections of that guide.

## Steps

- [x] Step 1: Render `status_row` through `StatusBar` with its defaults, show
      the progress as a `ProgressCircle` in it, map the theme tokens, and
      update the recorded rules and canvas size
  - Done when:
    - `status_row` in `crates/native/src/main.rs` builds the row with
      `gpui_component::status_bar::StatusBar`, without overriding its
      padding, border, fill, text size or text color.
    - The progress is a `gpui_component::progress::ProgressCircle` in a
      `StatusBar` slot, present only while `progress::shown` yields a
      `(done, total)`, its value from `progress::fraction`. The 2 px
      top-edge bar (`progress_bar`) and any wrapper around the row are gone.
    - `crates/native/src/progress.rs`'s `fraction` and `shown` and their
      tests are unchanged in behavior.
    - `crates/native/src/theme.rs`'s `map` sets `status_bar` and
      `status_bar_border` to the app's Neutral dark tokens, and
      `maps_the_neutral_dark_tokens` asserts both.
    - The status text, the `Sort ▾` / `Filter ▾` buttons (lit state, disabled
      Sort in a view-only folder, hover) and their popovers (opening upward,
      right edge on the button's, not clipped, dismissed by a press outside)
      work as before.
    - Showing or hiding the circle changes neither the row's height nor
      where the status text starts.
    - `docs/agents/native-app.md` describes the new construction: the
      "status row stays a plain `div`" section and the "One progress bar
      sits on the status row's top edge" section are rewritten, and the
      preview canvas size is the newly measured value wherever the old one
      (1380 x 852, and the figures derived from it) is recorded as the
      current size.
    - `CLAUDE.md`'s `crates/native` description of `src/progress.rs` no
      longer says the window draws the bar "along the status row's top
      edge"; it says where the indicator is drawn now.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
    - Manual check (the user, or the implementer if a display is available):
      launch the native app, open a folder, and confirm the row shows as a
      bar with a top border, its fill and 12 px muted text; open and dismiss
      both menus, including a tall `Filter ▾` menu; during a scan, watch the
      circle fill through pass 1, start again for pass 2 and disappear at
      the end, with the status text and the buttons not moving; run a Move
      to Trash on several files and watch the circle; confirm a long status
      text does not push the circle or the buttons out; confirm an error
      text is still readable in the muted color.
  - Implementation approach (as far as it is known; omit if unknown):
    - Change only `crates/native`, `docs/agents/native-app.md` and
      `CLAUDE.md`. The Tauri app is frozen.
    - `StatusBar` (0.9.1) is a `RenderOnce` `h_flex` with `left`, center
      (`child` / `children`) and `right` regions. Its root has `items_center`,
      `gap_2`, `py_1`, `px_2`, `border_t_1` in `cx.theme().status_bar_border`,
      `bg` from `cx.theme().tokens.status_bar`, `text_xs` and
      `muted_foreground`. Each region is an
      `h_flex().overflow_hidden().items_center().gap_2()`. The center is
      centered when both `left` and `right` hold something, end-aligned with
      only `left`, start-aligned otherwise.
    - Slot mapping: the status text as the center `child` (with no `left`
      and a `right`, the center is `flex_1` and start-aligned, as today);
      in `right`, the circle first (only while shown), then `Sort ▾`, then
      `Filter ▾`, each its own `.right(..)`. The gaps in the region are its
      8 px (`gap_2`), where the buttons are 4 px apart today.
    - Do not put the circle in `left`: a non-empty `left` next to the
      `right` buttons turns the center to `justify_center`, so the status
      text would jump to the middle of the row while the circle shows.
    - `ProgressCircle::new(id)` needs an element id. `.value(..)` takes
      0–100, so pass `progress::fraction(done, total) * 100.`. Size it with
      `Sizable`'s `.small()` (`Size::Small`, 12 px, the row's text size;
      `Sizable` is already imported in `main.rs`). Verify the row's height
      is the same with and without it: it should be, since the buttons are
      taller than 12 px.
    - Color: pass `.color(rgb(PRIMARY_COLOR))` (the bar's indicator color
      today). Without it the circle reads the theme's `progress_bar`, which
      `theme.rs` does not map and which was derived from `Default Dark`'s
      primary at load. The component draws the track as that color at 20 %
      opacity and has no separate setter, so the track is no longer
      `MUTED_FILL`.
    - Replace `progress_bar()` with one method returning the optional
      circle (named for what it draws now), keeping the same
      `progress::shown(self.trashing..., None, self.scan_status.running())`
      call. Remove what the removal orphans; check whether `MUTED_FILL` is
      still used elsewhere before touching it (it has another use today).
      Update `progress.rs`'s doc comments only where they say "bar" of the
      drawing; leave its functions and tests.
    - Do not use `.loading(true)`: nothing here is indeterminate. A total of
      0 (pass 2's start) shows the empty track ring, as the empty bar did.
    - The circle's value always animates: `ProgressCircle` runs it through
      `transition` over the theme's `motion.duration_normal` (180 ms), keyed
      by the element id, and requests animation frames while it runs. Check
      during a scan that (a) the drop from pass 1's full ring to pass 2's 0
      is acceptable to look at, and (b) the extra frames do not visibly
      slow paging during a scan; record what is seen in `native-app.md`.
      Do not zero `duration_normal` to stop it: the dialogs read it too.
    - Theme tokens: in `theme.rs`'s `map`, set `colors.status_bar` to
      `PANE_COLOR` and `colors.status_bar_border` to `BORDER_COLOR`, as the
      file does for every other token the app's components read.
      `Theme::update` reconciles a field edited on `colors` into
      `theme.tokens`, which is what `StatusBar`'s `bg` reads; verify on
      screen that the fill follows the mapping.
    - `menu_button` (the buttons already set their own 12 px text and
      colors), `popover`, `menu_surface`, `src/menu.rs` and the
      `max_menu_height` argument (`viewport height - 40 px`) do not change.
      `Deferred` draws with no content mask in `gpui-pre` 0.3.12 (recorded
      in `native-app.md`), so the regions' `overflow_hidden` should not clip
      the popovers; verify this on screen. Also verify that a tall menu
      still fits above the taller row; adjust the 40 px only if it does not.
    - Verify what a status text longer than the row does (wrap or clip): the
      center region is an `overflow_hidden` flex row, where today's wrapper
      is a block `div` with `flex_1` and `min_w_0`. Keep that wrapper as the
      center child only if the text otherwise pushes or overlaps the right
      region.
    - Measure the preview canvas in the default 1600 x 1000 window after the
      change (expected about 9 px less height than 852, from `py_1` and the
      1 px border; the width should stay 1380) and record the measured
      value. `native-app.md` holds the figure in "The sort and filter menus
      are deferred popovers over the window" (1380 x 852) and derives
      1160 x 852 from it in the tree pane's section; update both. Leave the
      dated benchmark figures (1275x852 / 1278x852 fit sizes) as the
      measurements they were.
    - Docs, `native-app.md`:
      - Rewrite "The status row stays a plain `div`, not gpui-component's
        `StatusBar`" into the new rule: the row is a `StatusBar` with its
        defaults, the two mapped tokens, the slot mapping and why the circle
        is not in `left`, what to re-check on a gpui-component bump.
      - Rewrite "One progress bar sits on the status row's top edge" for the
        circle: keep the `progress.rs` model, the order of operations, the
        `folder_op` hook and the list of runs without a progress callback;
        replace the two-`div` drawing, the `absolute` / `relative` rule and
        the "adds no `notify` or timer" rule with what holds now (in the
        row's flow inside the right region, no change to the row's height
        or the canvas, the value animating). Its paragraph on why
        `Progress` is not used stays true of the linear `Progress`; say
        that `ProgressCircle` shares the 20 % track and the animated value
        and is used by the user's decision.
      - Adjust the sentence "The status row is a flex row: ..." to match,
        and mark what was measured versus inferred as the guide does
        elsewhere.
    - Docs, `CLAUDE.md`: in the `crates/native` paragraph, the sentence
      "`src/progress.rs` the window's one progress bar (...), which the
      window draws along the status row's top edge" becomes one that names
      the progress circle in the status row; change nothing else there.
    - Commit as `feat(native): ...` per Conventional Commits; the row's look
      changes.

## Trade-offs and risks

- **Where the circle goes (recommended: first in `right`, left of
  `Sort ▾`).**
  - First in `right` (planned): a fixed spot, the status text never shifts,
    no space is reserved while nothing runs. While shown, the center region
    is 20 px narrower (12 px circle + 8 px gap), which only matters to a
    text that fills the row.
  - Before the status text, in the center: showing it shifts the text 20 px
    right and hiding it shifts it back. Avoiding that means an always-there
    12 px placeholder, which indents the text permanently.
  - After the status text, in the center: the text does not shift, but the
    circle moves whenever the text's width changes, and the scan's suffix
    changes on every progress message.
  - In `left`: not viable with the buttons in `right`, because the center
    then centers the status text.
- **Circle size (recommended: `Size::Small`, 12 px).** It matches the 12 px
  text and stays under the buttons' height, so the row does not grow.
  `Size::XSmall` (8 px) leaves a 1.2 px stroke that is hard to read;
  `Size::Medium` (16 px) is close to the buttons' height and risks growing
  the row.
- **The circle reintroduces what the guide rejected in `Progress`.**
  `native-app.md` rules out the linear `Progress` because its track is the
  indicator's color at 20 % and its value always animates over 180 ms,
  requesting frames and drawing a drop (pass 1 to pass 2, a rescan) as a
  shrink. `ProgressCircle` has both properties. The old rule "the bar adds
  no `notify` or timer" no longer holds: during a scan the circle requests
  animation frames after every progress message. The step checks the look
  and the paging cost; if either is unacceptable, the options are keeping
  the hand-drawn bar (in a wrapper around the `StatusBar`) or accepting it.
- **Less precise than the bar.** A 12 px ring shows the fraction far more
  coarsely than a bar spanning the window; the status text's scan suffix
  still carries the counts.
- **Mapping the `status_bar` tokens in `theme.rs` (recommended, planned)
  versus leaving the component's fallback.** Mapped (`status_bar` =
  `PANE_COLOR` `#171717`, `status_bar_border` = `BORDER_COLOR`, white at
  10 %), the row follows the app's tokens, is covered by the theme test and
  does not change with a gpui-component bump. The fill equals the fallback's;
  the border is the app's translucent one instead of a solid `#262626`.
- **The preview canvas shrinks by about 9 px in height.** The fit scale of
  every preview drops slightly, and the recorded 1380 x 852 figure changes.
  Accepted by the user's choice; the step measures the real value.
- **The status text becomes 12 px in `MUTED_COLOR`** instead of 16 px white.
  Errors and refusals shown there are less prominent. A text color override
  on the `StatusBar` would fix that; not planned.
- **The gap between `Sort ▾` and `Filter ▾` grows from 4 px to 8 px**, the
  region's own gap.
- **Long status text** may wrap or clip differently in the center region.
  The step checks it and keeps today's wrapper only if needed.
- **Popover clipping** was inferred from sources only; if the regions'
  `overflow_hidden` clips the deferred popovers on some platform, the step
  does not meet its acceptance criteria.
- **gpui-component bumps** can now change the row's height, padding, text
  size and the circle's size or animation; the rewritten doc sections should
  say to re-measure on a bump.

## Progress
