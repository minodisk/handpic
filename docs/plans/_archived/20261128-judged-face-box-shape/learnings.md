# Learnings

## Step 1

- Outcome B. Over the 39 counted pairs (24 same face, 10 different person,
  3 spurious, 2 unclear) the different people's aspect ratio between the
  boxes (1.002 to 1.258) and long-side ratio (1.017 to 1.931) sit inside the
  twins' ranges (1.001 to 1.481, 1.011 to 1.753). Above the highest
  neighbour only 4 of 24 twins fall (17%), far from the 20 the rule needed.
  See `measurement.md`.
- The scratch subcommand reproduced the archived table exactly: the same 39
  pairs at the same overlap values, after the same count filters. The
  filters matter: three more pairs in range (`_DSC3443`, `_DSC4113` and
  `_DSC4194` at 640x448) score under `FACE_CONFIDENCE` and no count takes
  them.
- One release build of `handpic-cli` (about 4.5 min from a cold `target/`,
  moved to the background by the harness) and one 9 s run over 36 files on
  3 threads were enough. The scratch code (`boxm`) was reverted before the
  commit; a copy is in the session scratchpad only.
- No code changed, so neither `FACES_VERSION` nor `ALLFACES_VERSION` moved.
  The `todo.md` section was deleted and the `shared-crates.md` section now
  records the double count as accepted.
