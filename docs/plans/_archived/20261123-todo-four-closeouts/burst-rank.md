# Within-burst ranking: AF-frame window against the eye-midpoint window

Step 2's spot-check of the `todo.md` sub-item "Spot-check whether the
sharpness ranking within a burst changes now that Sony frames with face
tracking are scored on the camera's AF frame instead of YuNet's eye midpoint".

## Method

- `handpic-cli features` (release build of this branch, 24 threads) over the
  Sony ARW folders `D:\photos\2026\2026-09-19` (282 files),
  `D:\photos\2026\2026-09-13-a` (622) and `D:\photos\2026\2026-09-13-b` (232).
- Compared, on the face-tracked frames (`eye_af` = `1`, i.e.
  `sharpness::eye_af_frame` gives a frame): `sharpness`, the stored score,
  which on those frames is the AF frame's window, against `eye_mid`, the
  window between the eyes of the cue's face
  (`score_preview(preview, None, None, &[face])`), present when that face is
  at or above `sharpness::FACE_CONFIDENCE`.
- Bursts and statistics by [`burst_spearman.py`](burst_spearman.py):

  ```sh
  mise x -- python burst_spearman.py one all.tsv sharpness eye_mid eye_af=1
  ```

  Per folder, frames in capture order, consecutive frames no more than
  1000 ms apart (`crates/app/src/burst.rs`); a burst counts when at least 3 of
  its face-tracked frames carry both values; ties take average ranks.
- Not the exact pre-#334 pipeline (Trade-offs 6): before #334 the scan chose
  the face from a whole-preview detection, while `eye_mid` takes the cue's
  face, the one nearest the AF point. Good enough for a spot-check.

## Result

- 1136 files, 715 face-tracked, 546 of them with a confident cue face.
- 69 bursts used (269 frames), none skipped as flat.

| Folders | Bursts | Median rho | p10 rho | Min rho | Same top frame |
| --- | --- | --- | --- | --- | --- |
| 2026-09-19 | 25 | 0.800 | -0.380 | -0.600 | 17/25 (68%) |
| 2026-09-13-a, -b | 44 | 0.500 | -0.357 | -1.000 | 26/44 (59%) |
| All three | 69 | 0.500 | -0.420 | -1.000 | 43/69 (62%) |

Lowest-correlation bursts (first..last file of the frames used):

| rho | Top frame | Frames | Burst |
| --- | --- | --- | --- |
| -1.000 | differs | 4 | `2026-09-13-a/_DSC9203.ARW.._DSC9207.ARW` |
| -1.000 | differs | 3 | `2026-09-13-a/_DSC9732.ARW.._DSC9734.ARW` |
| -0.600 | differs | 5 | `2026-09-19/_DSC1830.ARW.._DSC1834.ARW` |
| -0.500 | differs | 3 | `2026-09-13-a/_DSC8374.ARW.._DSC8378.ARW` |
| -0.500 | differs | 3 | `2026-09-13-a/_DSC8724.ARW.._DSC8729.ARW` |
| -0.500 | differs | 3 | `2026-09-19/_DSC2629.ARW.._DSC2634.ARW` |
| -0.500 | differs | 3 | `2026-09-19/_DSC2860.ARW.._DSC2862.ARW` |
| -0.400 | differs | 4 | `2026-09-13-a/_DSC7443.ARW.._DSC7448.ARW` |
| -0.257 | differs | 6 | `2026-09-13-b/_DSC1099.ARW.._DSC1105.ARW` |
| -0.200 | differs | 5 | `2026-09-13-a/_DSC8059.ARW.._DSC8066.ARW` |

The bursts are short (3 to 6 usable frames), so a single swapped pair moves
rho a lot; whether the swaps are between near-equal values was not checked.

## Conclusion

Not negligible: median rho 0.500 (bar 0.9) and the same top frame in 62% of
bursts (bar 80%). Per Trade-offs 2 (a) the AF-frame scoring stays as it is;
the good-photo mark reads `full_eye_sharp`, not this score. A todo candidate
is in [learnings.md](learnings.md).
