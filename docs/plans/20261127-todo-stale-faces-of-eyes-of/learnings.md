# Learnings

## Step 1

- The line numbers in the plan (as of `739a17ee`) had drifted: the
  `shared-crates.md` section sat at lines 1048-1095 and the `todo.md` item at
  lines 1523-1531 by the time of the implementation. Re-running the
  `git grep` first, as the plan says, found the same mentions.
- The boxes are scaled in the app by `mark::face_marks`, called from
  `overlay.rs` with `FacesResponse::width` / `height` as the preview size, so
  `tract-onnx-inference.md` now names those two instead of "the UI".
- Per the user's preference only `mise run fmt` and `mise run lint` ran
  locally (comments and Markdown only); the PR's GitHub checks cover the rest.
