# Learnings

## Step 1

- Every heading in lists A and B matched `main` at `1cad6de8` exactly once; no item had to be found by its text instead.
- As in the previous plan, a Python script located each item by its exact `###` line (one match asserted per anchor), replaced the named sentences with one-match asserts and dropped or replaced boxes by their index among the item's top-level `- [ ] ` lines. The neighbours stayed byte-for-byte, and a check for a `###` line without a blank line before it and for doubled blank lines found nothing new (`todo.md`'s line 4, `### App: real-device Ctrl+C and window check of the app:watch task` right under `## Cross-cutting / other`, already had no blank line before it on `main` and was left alone).
- In the Bash tool, a heredoc holding a long Python script with nested quotes and backticks failed to parse (`unexpected EOF while looking for matching '`); writing the script to the scratchpad with the Write tool and running it with `python -I` worked.
- D1: the plan first put the new paragraph between the "not seen:" sentence and its two bullets, which left that colon before a paragraph of things that were seen. Review round 1 flagged it; the paragraph now sits ahead of the "Seen in the running app" paragraph, so the colon is directly before the bullets.
