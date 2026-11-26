# Learnings

## Step 1

- Reproduction attempt (not kept): a UI test that wrote 400 JPEGs of
  1600 x 1200, then 300 of 4000 x 3000 (the `jpeg` gradient helper), opened
  the folder on a fresh index without waiting for idle and, every 5 ms until
  pass 1 ended, checked each on-screen cell (12 in the test window) for a
  committed row (`thumbnail` on the view's reader) with no image in `Thumbs`.
  The longest such gap was 58 ms, then 32 ms, both a `missing` cell between
  its `Progress` and its re-read; nothing stayed gray. Pass 1 took about 2 s
  there against about 90 s on the device (`scan extract … in 89963ms`), so
  the test platform did not reach the device's timing. With a consistent
  reader, the strip's `missing` / `ready` bookkeeping recovers in every
  interleaving traced, so the remaining suspects are a reader error (suspect
  1), an image not painted (suspect 3) or the cells' rows not being written
  early at all (the device saw 1, 2, 10 and 11 filled at `scanning 492/500`,
  which would also fit a scan order that missed 3 to 9).
- So the fallback shipped: `thumbs::read` warns on any read error other
  than rusqlite's `Query returned no rows` (`no_row`, tested against a real
  `Index::thumbnail` miss so a rusqlite message change fails the test), and
  `Handpic::thumbnail` logs a `NotScanned` answer for a cell the scan
  reported: `retried=true` at `info` for the expected first answer from a
  read that ran before the commit, `retried=false` at `warn` for a cell left
  missing until the pass ends (`Strip::after_ready`, kept in the new `reported` set because
  `ready` is consumed by the first answer).
- `python3` in Git Bash hung, as `CLAUDE.md` warns; use `perl` or
  `mise x -- python`.
- A first `cargo test` wrapped in `timeout 590` was killed mid-link on a
  cold `crates/app/target`; the build alone took longer than that.

## Deferred issues (todo candidates)

- Pending manual check (Windows, the user's): re-run the 500-JPEG
  fresh-index open (`jpeg500`, debug build, 1600 px window, `--log <path>`)
  and, while cells in view stay gray during pass 1, read `Handpic.log` for
  `thumbnail read failed, the cell waits for the scan:` and
  `thumbnail not scanned after the scan reported it: … retried=false`. Either
  line names the cause; neither for the gray cells points at the scan order.
  The `todo.md` item's TODO box was rewritten to this check; Step 1's box was
  ticked on the fallback's automated criteria. Files:
  `crates/app/src/thumbs.rs`, `crates/app/src/main.rs`, `crates/app/src/strip.rs`.
