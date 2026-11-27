# Learnings: exif-text-collation

## Step 1

- The existing test `text_ties_in_case_fall_back_to_the_plain_compare`
  asserted `["a", "B", "b"]`, the old case-insensitive compare's order. The
  plan's rule (and `localeCompare`) puts lowercase first, `["a", "b", "B"]`,
  which the plan's own `["ab","aB","Ab","AB"]` list requires too, so the
  Done-when "existing tests pass unchanged" could not hold for that one test.
  It was renamed `text_differing_in_case_puts_lowercase_first` and its
  expectation changed to `["a", "b", "B"]`; the two tests the Done-when names
  are unchanged.
- `icu_normalizer` with `default-features = false, features = ["compiled_data"]`
  resolved offline from the existing lock: the `Cargo.lock` diff is the one
  `"icu_normalizer"` line in `handpic-app`'s dependencies, no `[[package]]`.
- After NFD no char lowercases to several (U+0130 `İ` decomposes into `I` and
  a combining dot), so the first lowercase char is the whole lowercase; the
  tertiary compares the lowercase to the char, as the JS prototype's
  `toLowerCase` did.
- In Git Bash on this host a bare `python3` hangs (the Store stub), as
  `CLAUDE.md` warns; use `mise x -- python`.
