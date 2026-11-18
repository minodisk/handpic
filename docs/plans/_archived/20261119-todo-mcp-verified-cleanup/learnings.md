# Learnings

## Step 1

- Every heading the plan names was still present exactly once at `7619ce6d`, so no item had to be guessed.
- The whole-item and whole-box deletions were made in one `sed` pass by line number (line addresses in one `sed` invocation refer to the input, so the order of the ranges does not matter), after checking each range's first and last line; the reworded boxes and the merged notes were then applied as exact string replacements.
- The line numbers the merged A3 note cites (the heading at line 71 and the quoted-title reference at line 3227 of `docs/agents/native-app.md`) were checked against the file at `7619ce6d`.
