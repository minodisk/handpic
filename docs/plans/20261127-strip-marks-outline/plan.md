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

# Strip marks: one dark outline, the stars'

## Purpose

On screen (Windows 11, 2026-11-23, finding C of
[the device check results](../_archived/20261123-windows-device-checks-mcp-visual/results.md))
the strip's stars have a dark outer outline and its face focus icons do not.
The user wants the strip's marks drawn one way, and wants them to look like the
stars.

After this work the stars, the face focus icon, the closed-eyes mark and the
burst badge are all drawn the stars' way: the mark over one dark copy of
itself, 1 px down and right. The todo.md item is closed, and a check by the
user's eye is left in its place.

Decision rationale, to be carried into the PR body:

- The user, looking at the screen, wanted the marks to look like the stars.
- The stars' outline keeps a mark readable over the bright parts of a photo.
- The halo is not visible on screen, so matching the halo instead would make
  the stars less readable.

What is known about the cause, from reading the code (inferred, not measured):

- `paint_marks` paints the stars' shaped line once in black at `(+1, +1)` and
  then in `STARS_COLOR` on top.
- `paint_face_icon` (the face focus icon in slot 0, the closed-eyes mark in
  slot 1) paints eight black `paint_svg` copies at `overlay::HALO`'s offsets
  and then the colored icon. `paint_burst_badge` does the same with text.
- GPUI does not drop those copies, as far as its source shows
  (`gpui-pre` 0.3.12): `Window::paint_svg` inserts one `MonochromeSprite` per
  call outside any layer; `Scene::insert_primitive` gives each a draw order
  above every primitive it intersects; the nine sprites share one atlas tile
  and the sort is stable, so the colored one is drawn last; the Windows
  monochrome sprite shader outputs the color at the mask's alpha, so black is
  drawn like any other color.
- So the likely cause is the look and not a missing draw: eight 1 px copies of
  a 12 px icon with 1 px strokes merge into a soft dark patch that also fills
  the icon's inside, with no crisp edge like the one the stars' single copy
  gives a filled glyph. This is not confirmed; Step 1 confirms or corrects it
  on a capture before it changes the drawing.

## Steps

- [x] Step 1: Draw the face focus icon, the closed-eyes mark and the burst
      badge over one 1 px dark copy, as the stars are
  - Done when:
    - A capture of the unchanged build, taken as described below, shows what
      the eight dark copies under a face focus icon look like at device
      pixels, and the PR body states the cause it shows (the inferred cause
      above confirmed, or the corrected one).
    - `paint_face_icon` (both slots) and `paint_burst_badge` paint one black
      copy at `(+1, +1)` and then the mark, as `paint_marks` does for the
      stars. `main.rs` no longer uses `overlay::HALO`.
    - `overlay::HALO` and `overlay.rs`'s `paint_label` are unchanged: the `f`
      overlay's face labels still use the halo.
    - The doc comments of `paint_face_icon`, `paint_burst_badge` and
      `BADGE_COLOR` in `crates/app/src/main.rs`, and the passages of
      `docs/agents/app.md` that describe the eight copies under the burst
      badge and the face focus icon (the theme section's "the halos", the
      burst badge rule, and "A canvas paints an icon with `paint_svg`"),
      describe the new drawing.
    - A capture of the changed build is saved in this plan folder (PNG),
      linked from the PR body, taken through the MCP companion's `screenshot`
      tool, and the PR body says which of the four marks it shows.
    - The PR body carries the decision rationale from the Purpose section.
    - In `todo.md`, the item `### App: the strip's stars have a dark outer
      outline and its face focus icons do not` is deleted and an item for the
      user's check is in its place: the user checks on screen that the strip's
      stars, face focus icon, closed-eyes mark and burst badge each have the
      same dark outline and read over bright and dark parts of a photo.
    - The PR's GitHub checks pass.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/app/src/main.rs`, `docs/agents/app.md`, `todo.md`, this
      plan. `CLAUDE.md` does not mention the halo or the stars' dark copy, so
      it needs no change; check again against the final diff.
    - Keep the change surgical: the two paint functions' loops over
      `overlay::HALO` become one dark paint at `(+1, +1)`. Positions, sizes,
      colors, the paint order before the rejected dim and the slot layout
      stay as they are. If the three call sites share the offset, a single
      constant next to `MARK_SIZE` is enough; do not add a helper that
      abstracts text and SVG painting together.
    - The stars' method is usable for `paint_svg` as far as the source shows:
      a second sprite of the same tile in black, one pixel away. Verify it on
      the capture. If the capture of the unchanged build shows that black SVG
      copies are not drawn at all, a single copy will not show either: choose
      another means that looks the same as the stars' copy, and record which
      and why in `learnings.md` and the PR body.
    - No unit test is planned: the change is paint calls that need a `Window`,
      and there is no pure rule in it. If a pure part appears during
      implementation (for example the copy's position), test it.
    - Capture setup. Other sessions run the app at the same time, and the
      user's real index, settings and photo folders must not be touched:
      - Build with the `screenshot` feature (`mise run app:dev:screenshot`,
        see `docs/agents/app.md`), with cargo's `-j 2`; the host is low on
        memory.
      - Make a scratch folder. Copy or hardlink the photos of
        `D:\Photos\tests\20261127-c5\marks` into a folder of its own inside
        it; copy the `.xmp` / `.dop` sidecars, never link them, as the app
        writes them.
      - Launch with its own `--index`, `--settings` and `--log` paths in the
        scratch folder and `--mcp-port 47870`. The settings store needs
        `mcpEnabled` on for the server to start; `docs/agents/app.md` has the
        rules for a test run with MCP.
      - Stars can be set on the scratch copies through `set_judgment`.
      - Ask `screenshot` for a long edge at least the window's, so the image
        is not scaled down and the 1 px copy can be judged.
      - Take the capture of the unchanged build first, from the same build
        tree, so the second build is incremental.
      - Stop the app you launched when done.
    - Whether the test photos give a burst badge (frames within
      `BURST_GAP_MS`) or a closed-eyes mark is not known. Show the marks the
      photos give, and name in the PR body any of the four the capture could
      not show.
    - Colors, legibility and the final look stay the user's to judge on the
      running window; that is the todo item this step leaves.

## Trade-offs and risks

- The root cause is inferred from the source, not measured. The reading says
  the eight copies are drawn. If the capture of the unchanged build shows they
  are absent, the plan's premise that one black SVG copy will show is wrong
  too, and the step falls back to another means as described in its approach.
- The stars' method is one copy down and right, not a ring. Adopting it
  exactly gives the icon and the badge less dark cover than eight copies
  would, if those do show. The other option is to give all four marks,
  the stars included, the eight-copy ring. That changes the stars, which the
  user named as the look to match, and by the rationale above would make them
  less readable. This plan takes the stars' method.
- No unit test covers the change. The proof is the capture and the user's
  check on screen.
- The `f` overlay's face label keeps its eight-copy halo. It is drawn over the
  preview, not the strip, and is outside this request.
- The capture may not show the burst badge or the closed-eyes mark if the test
  photos give neither. The user's on-screen check covers them.

## Progress

- 2026-11-27: Step 1 done. `paint_face_icon` and `paint_burst_badge` now draw
  one black copy at `MARK_SHADOW` under the mark, as the stars do. The cause
  was confirmed on the capture. See `learnings.md` and the `before*` /
  `after*` captures in this folder.
