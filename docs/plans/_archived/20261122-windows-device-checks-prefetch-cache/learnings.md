# Learnings

## Step 1

- A click on a tree row gives the tree the keyboard, and `q` then goes to the tree, so the instance does not quit. Press `Escape` first (it hands the keyboard back), then `q`. Opens through MCP `open_folder` do not move the keyboard. Basis: `results.md`, Incidents.
- Until the previous test instance has exited, a new one on the same `--mcp-port` logs `could not listen on 127.0.0.1:<port>` (`os error 10048`) and runs without MCP. Check the previous PID is gone before launching the next.
- A folder open prints no `page` line; only a key press is timed. A folder switch is judged from burst captures, comparing each frame's preview region with the first and the last frame.
- A burst taken through PowerShell `Start-Job` starts about a second late; run it in the same process and start the action from a second process (a delayed Python MCP call) instead.
- The release build runs the launch update check (`--auto` is off) and offers the latest release to the `0.0.0` build; harmless for a check, but it is a network request.
- Every box passed; nothing was deferred.
- The branch started one commit behind `origin/main` (#1232, which deleted the `strip position in the status bar` item right after the Clear Cache item), so `git diff origin/main --stat` listed that PR's files too, and the rebase conflicted in `todo.md` where both deletions touched. Resolved by keeping both deletions. Rebase onto `origin/main` before checking the diff criterion.
