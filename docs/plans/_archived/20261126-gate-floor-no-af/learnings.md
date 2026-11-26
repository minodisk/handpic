# Learnings

## Step 1

- The measurement matched the archived prediction: `L1005161.DNG` counts all
  7 faces at the 0.6 floor (4 before). The only frames newly at the default
  gate are the three swing group photos of `2026-02-01`; on the two
  mostly one-or-two-person Leica folders the lower floor moved 22 and 51
  gate counts but none to 5 (N >= 4 went 0 -> 0 and 1 -> 2), so pass 3's
  extra work there is nil at the default threshold.
- The three AF sets showed 0 changed gate counts and every other `features`
  column was byte-identical, so the change touches the gate alone.
- `features` takes RAW file paths as well as folders, so the first-N slices
  were passed as 300 and 100 file arguments; the Windows command line held
  them.
- `handpic-cli faces <file> <png>` draws the whole-image search's boxes and
  prints their scores on a frame without an AF point, enough to class each
  newly passing frame.
- `FACE_CONFIDENCE` became unused in `scan.rs`'s non-test code once the `None`
  arm stopped passing it; the import moved into the test module.

## Deferred issues (todo candidates)

- Pass 3 still takes its candidates from `FACE_CONFIDENCE` (0.8)
  (`allfaces::candidates`, `crates/core/src/allfaces.rs`), so on
  `2026-02-01/L1005161.DNG` the gate counts 7 but pass 3 judges at most the 4
  faces at 0.8 or above (3 of its faces score 0.64 to 0.75), and `Similar
  faces` exceeds `Faces judged` by 3; `L1005172.DNG` likewise has 3 faces
  between 0.6 and 0.8 and `L1005173.DNG` 2. Basis: this plan's trade-offs
  section and Step 1's measurement (`measurement.md`). Whether to lower the
  candidates' floor is a separate change.
- The todo.md item "Core: the default group threshold N = 5 passes no posed
  group photo of the measured folders (all-faces-check)" can be closed: three
  posed group frames of `2026-02-01` now reach the default gate
  (`measurement.md`).
