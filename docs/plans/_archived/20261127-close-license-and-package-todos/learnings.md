# Learnings

## Step 1

- The edit was made by heading match, not line number: at implementation time the capital-executable section was at line 1381 and the license section at 1399, not the 1420 / 1438 the plan recorded, since `todo.md` had shrunk in between.
- `mise run fmt` and `mise run ci:local` were not run locally for this Markdown-only change (the user's standing rule leaves the checks to the PR's GitHub Actions); the spacing around the deleted section was checked by eye in the diff.
