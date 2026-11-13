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

# GPUI held-key preview

## Purpose

In the GPUI native app (`crates/native`), holding Left / Right on the strip
shows no preview until the key is released, in the fit view and in the 1:1
view (`z`) alike. Both chains keep one read in flight and throw away a result
that is no longer the current file's: `Pager::finish` and `loaded()` for the
preview, `Zoom::page` and `Zoom::arrived` for the crop. Whenever one read +
decode outlasts the key repeat (about 30 ms), every result during the hold is
stale, so nothing is drawn until the keys stop.

After this work a held key flips through frames in both views: a stale result
is drawn before the current one is requested, so the drawn frame lags the
cursor by at most one load. Each chain still has exactly one read in flight,
nothing is queued, and the number of reads per key sequence does not change.

## Steps

- [x] Step 1: Draw a stale preview result before chasing the current file
      (fit view)
  - Done when:
    - Holding an arrow key flips through previews in the fit view; the drawn
      frame lags the cursor by at most one load, and intermediate files are
      still skipped.
    - On release the current file is shown with its overlays, on-demand
      analysis, dwell / viewed marking and latency `page` line exactly as
      before.
    - The number of preview loads per key sequence is unchanged: a stale
      result sends exactly one request for the current index, a current
      result sends none, and nothing is queued.
    - The focus mark, face boxes, mesh, 1:1 placeholder and the peaking
      overlay never draw on an image with another file's data. The 1:1 view
      behaves as before this step.
    - `pager.rs` has unit tests for the changed `finish` (see below), and
      `mise run ci` and `mise run native:check` pass (`ci` does not run the
      native workspace's clippy and tests).
    - `docs/agents/native-app.md` has an entry for the new behavior.
    - The PR description notes the manual check on Windows by the user (a
      held key flips frames; release shows the current file with its
      overlays) as pending.
  - Implementation approach (as far as it is known; omit if unknown):
    - Files: `crates/native/src/pager.rs` (`finish`, its tests, the `Pager`
      doc comment, which says a held key "never queues stale decodes" and
      should now also say a stale result is drawn), `loaded()` in
      `crates/native/src/main.rs`, and `docs/agents/native-app.md`.
      `CLAUDE.md` does not describe `pager.rs`, so its layout paragraph is
      not touched. `send()`, `go()`, `refilter`, `overlay.rs`, `zoom.rs`,
      `peaking.rs` and `hold.rs` should need no change in this step; if one
      does, record why in `learnings.md`.
    - Put the decision in `Pager::finish` so it is testable without GPUI:
      given the result's index, whether the read succeeded, and the index
      whose image is on screen (from `self.shown`, `None` when no image or a
      failure is shown), it returns what to do with the result and the index
      to send next, if any. The four outcomes:
      - current, `Ok`: the full path as today, nothing to send, `in_flight`
        cleared;
      - current, `Err`: the failure path as today, nothing to send,
        `in_flight` cleared;
      - stale, `Ok`: draw the frame only, then send the current index
        (`in_flight` stays set); but draw nothing when the image on screen
        already belongs to the current file (A shown, press to B, press back
        to A before B arrives: drawing B would flash the wrong file over the
        right one, which today's code does not do);
      - stale, `Err`: draw nothing, show no failure, print nothing, send the
        current index. The names and the exact types are the implementer's.
    - A stale frame does only this in `loaded()`:
      - the status line, built as for the current file (extract the
        `format!` rather than duplicating it if both arms need it);
      - `self.shown = Some((index, Shown::Read(..)))`. `refilter` rewrites the
        status line's `n/total` from `self.shown` and re-requests when
        `shown != self.current_path()`, so `self.shown` must name the file
        whose image and status line are on screen. The meta pane therefore
        follows the frame on screen, as its doc comment says;
      - the image swap with `retire(old)`;
      - `self.marks.show(Some((stale_path, orientation)))`. `Marks::mark`,
        `wanted_faces` and `wanted_eyes` gate on `shown == current`, so this
        is what keeps the `f` overlay off a stale image, including when the
        user steps back onto an earlier file while a load is out. Leaving
        `marks.shown` at the previous file would draw that file's marks on
        the stale image in that case;
      - `self.hold.hold(loaded.paused)`: the frame is painted, and the paint
        closure releases the hold on every paint;
      - `cx.notify()`, then `self.send(current, received, cx)`.
    - A stale frame must not: call `self.zoom.shown` (it stamps the facts
      with the current `seq`, which would give the current page the stale
      file's focus point and orientation), call `analyze_current` or
      `start_dwell`, take `self.key`, or set `self.pending`. `self.key` is
      taken only by the current file's result, so the latency `Page` still
      measures from the last keypress to the current file's pixels.
    - Unit tests to add or adapt in `pager.rs` (the three existing `finish`
      tests change with its signature): a run of stale results under a held
      key yields one send per result and never a second request while one is
      out; a stale `Ok` draws, a stale `Err` does not and still sends; a
      stale `Ok` does not draw when the current file's image is on screen; a
      current `Ok` and a current `Err` clear the in-flight marker and send
      nothing; after the hold ends the next `request()` works.
    - Verify during implementation, by reading rather than assuming:
      `zoom.preview()` is `None` for every stale frame (each `go` and each
      moved `refilter` calls `zoom_page`, which bumps `seq`), so the zoomed
      canvas never paints a stale image as the placeholder; the peaking paint
      draws an overlay only for the source image id and size it was computed
      for, so a stale frame gets its own edges or none.
    - `docs/agents/native-app.md`: add an entry in the file's existing form
      (a `###` heading with its `(Measured)` / `(Inferred)` / `(Hit)` tag and
      `- Rule:` lines) stating that a stale result is drawn before the
      current index is requested, which side effects are the current file's
      only, and why `marks.show` and `self.shown` follow the image while
      `zoom.shown` does not. Tag it `(Inferred)` until the user's manual
      check is in. The "Peaking costs a held key nothing measurable" entry's
      note that a held key on RAF shows 4-6 pages in 5 s is a dated
      measurement of recorded `page` lines; leave it, and say in the new
      entry that `page` lines are still recorded only for the current file.
    - Check `docs/humans/*.md` and `README.md` for a description of the
      native app's held-key behavior; if one exists, update it and its `.ja`
      pair in the same PR.
    - No same-session A/B latency gate is run for this plan.

- [x] Step 2: Draw a stale crop before chasing the current one (1:1 view)
  - Done when:
    - Holding an arrow key while zoomed flips through 1:1 crops; the drawn
      crop lags the cursor by at most one crop read, and intermediate files
      are still skipped.
    - On release the current file's crop is shown, and what belongs to the
      current page's crop only still happens only for it: the `zoom` timing
      line of a `z` press, the resize re-cut, and the reuse of a held crop
      when `z` is toggled or the viewport settles.
    - The number of crop reads per key sequence is unchanged: one in flight,
      nothing queued, a stale crop sends exactly one request for the current
      page, and only while zoomed.
    - The canvas never mixes two files: a crop is never drawn over another
      file's placeholder, and another file's `full` size never scales the
      placeholder. The crop's peaking overlay is only its own.
    - A stale read that failed shows no error; a failed read of the current
      page shows its error and leaves no other file's crop on screen.
    - `zoom.rs` has unit tests for the changed model (see below), and
      `mise run ci` and `mise run native:check` pass.
    - `docs/agents/native-app.md` and the `zoom.rs` sentence of `CLAUDE.md`'s
      layout paragraph are updated where they describe what the model holds.
    - The PR description notes the manual check on Windows by the user (a
      held key flips crops while zoomed; a single press and `z` off / on
      still behave) as pending.
  - Implementation approach (as far as it is known; omit if unknown):
    - Assumes Step 1 is merged: the placeholder is then still only ever the
      current file's preview, because only the current file's result calls
      `zoom.shown` and `zoom.preview()` filters on the current `seq`.
    - Files: `crates/native/src/zoom.rs` (`Held`, `page`, `arrived`,
      `toggle`, `settled`, the `Arrived` enum and its doc comments, tests),
      and `zoom_page`, `crop_arrived` and `zoom_paint` in
      `crates/native/src/main.rs`. `zoom::send`, `zoom::read`, `Paint::paint`
      and `peaking.rs` should need no change.
    - Keep the change in the pure `Zoom<C>` model, which is already generic
      and unit-tested without GPUI:
      - `Held` records the `seq` its crop was read for;
      - `page()` no longer releases the held crop on a press (releasing it
        and drawing the next stale one would flicker between a crop and
        black at the key repeat); an empty listing still leaves the view and
        releases it;
      - `arrived()` with another page's `seq` and an `Ok` crop holds that
        crop, releasing the one it replaces, and requests the current page
        while zoomed; with an `Err` it draws nothing, reports no failure and
        requests the current page, as today;
      - `arrived()` with the current page's `seq` is as today, except that a
        failure also releases a held crop of another page;
      - `toggle()` and `settled()` skip the read only when the held crop is
        the current page's and for this viewport; a stale held crop never
        counts as "already have it";
      - `in_flight` handling is unchanged.
    - In `main.rs`: `zoom_paint` draws the held crop whatever page it is for,
      and builds the placeholder (and passes it `crop.full`) only when no
      crop is held or the held crop is the current page's; the model should
      expose that as one question rather than `main.rs` comparing `seq`s.
      `crop_arrived` keeps storing `zoom_pending` and calling `zoom_resized`
      only for the current page's crop, and calls `self.fail` only for the
      current page's failure.
    - Check the three callers of `zoom_page` (`go`, the listing, a moved
      `refilter`): with `page()` keeping the crop, a folder switch keeps the
      old crop on screen until the new one arrives, as the fit view keeps
      its image. Confirm nothing indexes `self.files` with the old crop's
      page.
    - Unit tests to add or adapt in `zoom.rs`: a press keeps the held crop
      and asks for the new page once; a run of presses with stale arrivals
      yields one request per arrival and holds each stale crop, releasing
      the previous; a stale crop held does not satisfy `toggle` or `settled`
      for the current page; the placeholder question is false while another
      page's crop is held and true for none or the current page's; a stale
      `Err` holds nothing new and requests; a current `Err` releases a stale
      held crop; a stale arrival while not zoomed requests nothing; an empty
      listing releases the crop.
    - `docs/agents/native-app.md`: extend Step 1's entry, or add one next to
      "1:1 is one JPEG pixel per device pixel", with the rules above (the
      crop on screen may be another page's; the placeholder and the crop are
      always one file's). Update `CLAUDE.md`'s `src/zoom.rs` sentence only as
      far as "the `Zoom` state machine ... one read in flight" needs it.

## Trade-offs and risks

- What the 1:1 canvas shows between a press and the current crop. Planned:
  the held crop stays, even another page's, until the current crop arrives,
  and the placeholder is drawn only with no crop or the current page's. A
  single press while zoomed therefore goes old crop, new crop; today it goes
  black, placeholder, crop. Alternative: show the current file's placeholder
  as soon as its preview arrives and hide the stale crop. That keeps today's
  single press, but where the preview load is shorter than the key repeat a
  held key alternates between each page's upscaled placeholder and an older
  page's crop.
- Stepping back to a file while zoomed. A current with its crop, press to B,
  press back to A: A's crop is another page's by `seq`, so B's crop replaces
  it when it arrives and A is read again. Planned: accept that flash; it is
  B's data alone. Alternative: tag the held crop with the file instead of
  only the `seq`, which widens `Zoom` and `Request`.
- During a zoomed hold the status line and the meta pane follow the preview
  chain and the canvas follows the crop chain, so they can name different
  files, each at most one of its own loads behind the cursor. Making them
  agree would couple the two chains; not planned.
- A folder switch or re-listing while zoomed keeps the old crop until the
  new one arrives. Alternative: release it on a listing change, which needs
  `page()` to tell a press from a listing.
- Stepping back onto the file whose stale preview is on screen (fit view):
  the image is right and the `f` overlay draws what is already known for it,
  but the on-demand analysis, the dwell and the latency record wait for that
  file's own reload. Skipping the reload would change the load count and
  move the current-file side effects into `go()`; not planned.
- More frames are drawn and presented during a hold, and with peaking on
  each drawn frame or crop asks for a layer (still one job in flight, a
  replaced layer never computed). The cost to the read in flight was not
  measured.
- `Pager::finish` taking the read's success and the shown index widens a
  small API. The alternative keeps `Option<usize>` and puts the stale-`Err`
  and no-flicker rules in `loaded()`, where they have no unit test.

## Progress

- (2026-11-13) Step 1 complete
- (2026-11-14) Step 2 complete
