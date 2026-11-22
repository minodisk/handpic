# Learnings

## Step 1

- `q` still quits in the default keymap. The plan suspected the strip box's "quit with `q`" was stale; it is not, and the run quit every instance with it (the log showed the quit handlers' drains), so the reworded boxes keep that wording.
- The `page` / `zoom` timing lines go to stdout, not to `Handpic.log` (`--log`); the run read them from the redirected stdout. `docs/agents/app.md` already has this rule for the `page` lines (Timings).
- A synthetic held key was made of repeated key-downs (90 at 33 ms) with one key-up, since a single synthetic key-down does not auto-repeat.
- The `scanning N/total` cadence was judged on burst captures of a screen region (the status line or the strip, one capture every 80 to 200 ms, stacked with the ms offset), not on MCP `get_status` polls, which came about 0.4 s apart.
- Pass 1 on files already in the OS cache ends before the debug window takes keys, so the steered-scan check (an `End` right after the window opens) and the placeholder phase cannot be seen on hardlinks of files earlier runs read. Those checks need files that are not cached (`robocopy /J` copies used once, the rule `docs/agents/app.md` already gives for a cold pass 1). Both narrowed boxes now say so.
- `docs/agents/app.md` got nothing: both lessons above are already rules there.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the strip item's narrowed box 2): open a 1000+ RAW folder of `robocopy /J` copies not in the OS cache with a fresh index and watch the gray placeholders turn into thumbnails from the current cell outward while `scanning N/total` climbs. Not exercised on 2026-11-22 (the cells in view were filled by the first capture). Already in `todo.md` under `### App: real-device checks of the strip, the scan progress, the resume and the resync`; Step 1 was ticked on the other sub-points. Basis: `results.md` box 2 (1). Files: `crates/app/src/main.rs`, `crates/app/src/thumbs.rs`, `crates/app/src/focus.rs`.
- Pending manual check (Windows, the same item's narrowed box 3 (b)): on such uncached copies with a fresh index, press `End` right after the window opens and expect the thumbnails near the last cell within a second or so while `scanning N/total` still climbs, then the middle filling after a wheel there. Not exercised on 2026-11-22 (pass 1 had ended on cached files). Already in `todo.md` in the same item. Basis: `results.md` box 3 (b). Files: `crates/app/src/focus.rs`, `crates/core/src/scan.rs`.
- The strip cells of `NIKON_D70_Nikon.nef` / `CGO3P_YUN00007.dng` read `panic while encoding the thumbnail of \\?\D:\...`, the internal panic wording with the Windows verbatim path. Added to `todo.md` as `### App: a strip cell of a file whose thumbnail encode panics reads the internal panic text and the Windows verbatim path`. Basis: `results.md`, the malformed-JPEG section. Files: `crates/core/src/scan.rs`, `crates/app/src/strip.rs`, `crates/app/src/context.rs`.
- The 1:1 view of a file whose crop read fails is a plain black canvas (`z` hides the failed-preview message, no text of its own). Added to `todo.md` as `### App: the 1:1 view of a file whose crop read fails is a plain black canvas`. Basis: `results.md`, the malformed-JPEG section. Files: `crates/app/src/zoom.rs`, `crates/app/src/main.rs`, `crates/app/src/empty.rs`.
