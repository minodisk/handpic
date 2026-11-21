# Learnings

## Step 1

- Heading drift: none. The branch was at `origin/main` (`762d70f5`) when the step ran, and every heading the plan names (lists A, B and the C anchors) matched `todo.md` exactly once, so no item had to be found by its text.
- The meta pane item (A2) has no `#### Background` heading; it runs from its `###` line to the next `###` like the others, so it was deleted whole.
- `## Cross-cutting / other` is followed directly by its first `###` with no blank line in `main`; that is outside the plan's lists and was left as is.
- The edits were applied by a script that locates each item by its exact `###` line and asserts one match per anchor, which kept the neighbouring items byte-for-byte.
