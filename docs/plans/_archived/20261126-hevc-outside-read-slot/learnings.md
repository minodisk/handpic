# Learnings: run the HEVC decode outside the read slot

## Step 1

- The split: `reader::read_preview_undecoded` / `read_full_undecoded` return
  `(Arw, Undecoded)`, and `Undecoded::decode` runs `hevc::to_jpeg` for an
  HEVC image and `jpeg::neutralize_orientation` for every image, the old
  `neutral`. `Undecoded`'s fields stay private, so `Codec` is not part of the
  new API. `read_preview` / `read_full` are the two halves in a row.
- Scan sites: pass 1's `extract_unless` decodes right after the slot drops and
  before its cancel check, so a decode failure still reaches the
  `read_metadata` fallback with the same message. Pass 2's
  `extract_analysis_unless`, `extract_face_counts_unless` and `full_sharps`
  decode after the read's `yield_to_pause` and cancel check, so a preview's
  pause holds the decode back too and a canceled file skips it.
- `full_sharps` cannot reach an HEVC decode through the scan:
  `has_larger_jpeg` is false for `arw.hevc`. The slot test reaches it anyway
  by handing `full_sharps` a hand-made `Arw` with a larger JPEG and the HDR PQ
  fixture's path, which `read_full_undecoded` re-parses.
- The slot test uses a `#[cfg(test)]` thread-local hook in `reader`
  (`tests::AT_HEVC_DECODE`) run as an HEVC decode starts; it records the
  cap 1 gate's free permits (1 at all four sites).
- App: `load` is now `load_with(.., finish)`, the decode a closure so the test
  can stretch it; the HEVC decode's time is counted in the `page` line's
  `decode` (noted in `docs/agents/app.md`'s Timings). The pause is still
  handed to `PaintHold` (Option A).
- Measurement not run: it needs a local release build of the app (or the
  engine), which the "CI on GHA" note and this host's memory rule out, and
  `handpic-cli scan` runs uncapped. Its samples and method went to `todo.md`
  as their own item; `docs/agents/shared-crates.md` says it is pending.

## Deferred issues (todo candidates)

- Pending measurement (already written into `todo.md` in this step, as the
  caller asked, so the wrap-up need not add it again): pass 1's ms/file at
  `READ_SLOTS` 2 on a few hundred copies of the HDR PQ CR3 samples in
  `D:\Photos\samples\CR3`, before and after the split, from a release app
  build with a fresh `--index` (`scan extract:` line). Basis: this plan's
  Done-when; files `crates/core/src/scan.rs`, `docs/agents/shared-crates.md`.
