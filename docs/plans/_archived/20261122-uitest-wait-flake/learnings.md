# Learnings

## Step 1: Raise the UI tests' `WAIT` to 30 s

### Diagnosis

- The failing test: `uitest::a_filter_choice_narrows_the_files_and_lands_on_the_anchor`
  (`crates/app/src/uitest.rs`), timing out at `WAIT` (10 s) with
  `scan Some(("analyzing", 3, 4)), running false`.
- Failing commits on `main`: d6e1b4ce (ubuntu), accb630d and 86bb0180
  (windows); 6ce7797e passed.
- It is the only UI test that opens a 4-file folder. Pass 2 runs 3 workers,
  so the 4th file starts only after one of the first three ends.
- In a debug build each tiny DNG takes 0.75 to 1.3 s of compute. The open
  takes about 7.6 to 8.9 s when the suite runs on 2 cores, leaving 1 to 2 s
  of margin under 10 s.
- The scanning UI tests added in #1228 and #1226 raised the concurrent load
  enough to cross 10 s often.
- `ReadSlots::acquire` and `yield_to_pause` wait under 20 ms; #1228's
  prefetch guard does not run during this open (`scanning()` is checked
  first). So no app-side wait explains the time.
- Pinned to 2 cores, `main` failed 3 runs of 9. With `WAIT` at 30 s it
  passed 16 of 16, each run 11.0 to 11.9 s.

### Rejected alternatives

- Fewer files in the test: the 4th file is what makes a pass 2 worker hand
  over, so coverage would change.
- Serializing the UI tests: a slower suite for every run.
- A separate deadline for the drop's `remove_within`: it follows `WAIT` to
  30 s as the smallest diff; removal normally takes about 11 ms, so only a
  held folder fails later.

### Docs

- `docs/agents/app.md` had two rules against raising the wait. They stay,
  with this one measured raise recorded, so a later timeout is diagnosed
  from its log lines rather than answered by another raise.
