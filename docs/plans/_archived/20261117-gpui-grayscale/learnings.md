# Learnings

## Step 1

- `gpui-pre` 0.3.12's `Window::paint_image` takes a `grayscale: bool` per
  sprite, so graying the photo is one argument at the two preview-path call
  sites (the fit image in `main.rs`, `zoom::Paint::paint`'s `draw`); no CPU
  conversion and no second texture.
- `cx.on_blur(&handle, ..)` fires only when the handle was the leaf of the
  previous focus path and is not the leaf of the new one, so a move of the
  focus to the settings layer, a dialog button, a `PopupMenu` or an `Input`
  releases the hold. The tree takes the keyboard through a flag
  (`tree_keys`), not a focus handle, so a click in the tree does not blur the
  view. Read from the sources only; the GUI was not run here.
- `ModifiersChangedEvent` carries only the new modifiers, so `Hold` keeps the
  last ones it saw to tell a modifier going up from one going down.

## Deferred issues (todo candidates)

- Pending manual check (on the user), Windows, the GPUI app
  (`crates/native`, `mise run native:run` or the built binary): the step's
  checkbox was ticked on the automated criteria (unit tests of `Hold`,
  `mise run ci:local`); the GUI was not run by the implementer. Steps and
  expected results, from the plan's Step 1 Done-when:
  1. Open a folder, hold `g` in the fit view: the photo turns gray; release:
     color returns. Repeat in 1:1 (`z`), the placeholder and the crop both
     gray.
  2. With `f` and `e` on (and `d`), hold `g`: the AF mark, face boxes, mesh,
     labels, the peaking layer and the performance graph stay colored; the
     strip, the panes and the empty-state text are unaffected.
  3. Hold `g` long enough to auto-repeat: no flicker.
  4. Rebind `grayscale` to `ctrl+g` in the settings; hold Ctrl+G and release
     Ctrl first, then again releasing `g` first: color returns on the first
     release either way.
  5. Hold `g` and Alt+Tab away and back: the photo is in color.
  6. Hold `g` and open the settings (`ctrl+,`) or a right-click menu, release
     `g`, close it: the photo is in color (the view's `on_blur` released it).
  7. Paging, `z`, `f`, `e` and `d` behave as before while `g` is held.
  Related files: `crates/native/src/grayscale.rs`, `crates/native/src/main.rs`,
  `crates/native/src/zoom.rs`.
