# Learnings: todo.md docs closeouts

## Step 1

- The settings view's refusal line is `SettingsView::status`, set by `edit`,
  `commit_group` and `failed` (the `Choose…` dialog error goes through
  `failed` too) and drawn in red as the last child of the settings box. Five
  places in `docs/agents/app.md` and one in `CLAUDE.md` had called it the
  status line; `app.md`'s "The status line under the box shows `NOT_SAVED`"
  was left as is, since "under the box" already names the settings' own line.
- `cameras.ja.md` was brought in line mechanically: the Japanese rows whose
  first column the English tables lack were dropped, the `1920x1280` row was
  copied from the English, and a diff of every table row (headers aside) came
  back empty. Both files now have 18 blank-line-separated blocks. The new
  policy paragraph is fresh Japanese prose for the user to review in the PR.
- In the develop README the "needs approval" edges from `merger` back to the
  main agent were dropped from both flowcharts rather than relabeled: under
  the standing approval the release-shaping stop happens before `merger` is
  started, so no edge from `merger` carries it. The sequence diagram shows the
  stop as an `opt` block ahead of the dispatch.
- The "loosen the approval judgment itself" mitigation of the `/goal` note was
  dropped with the stop it addressed: an ordinary flagged path no longer stops
  the run, and the release-shaping paths are not meant to be loosened.
- A heredoc passed through the Bash tool collapsed a doubled backslash in a
  Python string literal to one; a `lambda` replacement in `re.sub` sidestepped the escaping.
