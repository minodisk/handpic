# todo

## Cross-cutting / other
### App: the keymap capture refusal is drawn in the settings body, but app.md says the status line

#### Background

`docs/agents/app.md`'s keymap section ("The keymap is edited in the modal and replaced whole") says a refused capture key shows its refusal on the status line. On Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`, item 14), `+` then `home` and `+` then `end` showed `"home" is taken by this app` (and the same for `end`) in red in the settings body; the full-window settings layer covers the status line, and the keymap was unchanged. It may be the settings' own line rather than the status line. Plan: `docs/plans/_archived/20261127-windows-device-checks-c5/plan.md`. Files: `docs/agents/app.md`, `crates/app/src/settingsview.rs`.

#### TODO

- [ ] Check in `crates/app/src/settingsview.rs` which line draws a refused capture key's refusal, and correct the rule in `docs/agents/app.md`'s keymap section to match.

### Core: the AF path's sharpness luma covers the whole frame though the score reads only its window

#### Background

af-luma-reuse (`FACES_VERSION` 17) scores a file with a trusted AF point on the luma of the cue's RGB decode. The measurement (`docs/plans/_archived/20261128-af-luma-reuse/luma-reuse.md`, "Saving per file") showed the full-frame `candidate::luma` takes a median 3.1 ms of the 6.4 ms grayscale decode it replaced on a 1616x1080 ARW preview, and much more on a 4416x2944 RAF preview. `sharpness::laplacian_variance` reads only the window's pixels, so a luma of just the score's window (paths 0 and 1, and path 2's eye window) would give bit-identical scores. That needs no ranking check and no `FACES_VERSION` bump, and saves about 3 ms more per ARW file.

Files: `crates/core/src/scan.rs` (`score_rgb`), `crates/core/src/sharpness.rs` (`score_gray`, `window_at`), `crates/core/src/candidate.rs` (`luma`).

#### TODO

- [ ] Compute the luma over the score's window only, check the scores are bit-identical to the full-frame luma on a handful of files (ARW and one larger preview), and update the docs that quote the saving, including one sentence in the `FACES_VERSION` 13/17 paragraph of `docs/agents/shared-crates.md` saying the luma is windowed.

### Core: the luma-reuse score's ranking on non-Sony AF bursts is unchecked

#### Background

`FACES_VERSION` 17 (af-luma-reuse) applies the luma reuse to every file with a trusted AF point: RAF, CR3, NEF, ORF and DNG as well as ARW. Only Sony bursts were measured, because plan Trade-offs 2 decided (a) to decide on Sony alone (`docs/plans/_archived/20261128-af-luma-reuse/luma-reuse.md`, "Limits"). The luma-vs-Y score difference on a RAF preview was the largest measured earlier (GX680, 1.1%).

Files: `docs/plans/_archived/20261128-af-luma-reuse/luma-reuse.md`, `crates/core/src/scan.rs`.

#### TODO

- [ ] When real bursts of another make are on the host, run `lumascore.patch`'s subcommand (in `docs/plans/_archived/20261128-af-luma-reuse/`) and `burst_spearman.py one` on them, and record the within-burst Spearman and the top-frame share. If the ranking holds, update the "other makes' AF bursts were not checked" sentence in `docs/agents/shared-crates.md`; if it breaks, restrict the reuse by make and bump `FACES_VERSION`.

### App: the strip's Copy Path copies the Windows verbatim prefix

#### Background

Seen on 2026-11-28 (Windows 11) while checking the right-click menus' keys (`docs/plans/_archived/20261128-context-menu-keys/results.md`): the strip menu's `Copy Path` on `_DSC1495.ARW` put `\\?\D:\Photos\tests\20261128-cmk\photos\_DSC1495.ARW` on the clipboard, while the tree menu's `Copy Path` put `D:\Photos\tests\20261128-cmk` (no prefix). The strip's handler in `crates/app/src/main.rs` (`context::Action::CopyPath` next to `RevealFile`, which does use `context::shown_path`) joins the listed paths as they are. Files: `crates/app/src/main.rs`, `crates/app/src/context.rs`.

#### TODO

- [ ] Pass each path through `context::shown_path` before the strip's `Copy Path` joins them, with a test, so the clipboard holds `D:\...` as the tree's does.

### Engine: real-device check that an undo of Move to Trash leaves no `$I` file on Windows (undo-recycle-index-cleanup)

#### Background

`undo-recycle-index-cleanup` makes the engine delete the Recycle Bin's `$I` partner of every item that `restore_all` really restored, in `restore_listed`'s chunk closure and in `restore_item` (`crates/engine/src/trash.rs`, `remove_index_file`). The unit tests cover only the `$R` to `$I` name mapping (`index_file_of`); no test touches the real Recycle Bin. `mise run ci:local` was stopped by host memory pressure and not re-run, so the PR's GitHub checks verified the build. The delete itself was never exercised against a real Recycle Bin through the app. Plan: `docs/plans/_archived/20261128-undo-recycle-index-cleanup/plan.md`. Files: `crates/engine/src/trash.rs`.

#### TODO

- [ ] On Windows, count the `$I` files in the volume's `$Recycle.Bin\<SID>\` folder. Run the app with `mise run app:dev:screenshot` (private `--index`, `--settings` and `--log` under a scratchpad folder, `--mcp-port 47842`) on fixtures copied to a temporary folder. Move files to the Trash, undo, and count again. Expect the count unchanged and the files back. Not run in the implementing session: the host was under memory pressure that had already killed a local `ci:local`, so the app build was deferred.

### App: confirm the split-deferred-open test fails without its fix

#### Background

`split-deferred-open` redirects a deferred open of a folder a `Rename by Capture Date…` split removed to the first date folder (`date_folder_ran` in `crates/app/src/folderops.rs`). Its new UI test `a_held_click_on_a_folder_a_split_removed_opens_the_first_date_folder` (same file) is meant to fail without the fix: the open of the missing `inner` runs and `view.dir` never ends with `2026-11-27`, so it times out in `wait_until`. That failing run was never done, because the local test build was killed for lack of memory; CI only shows the test passes with the fix. Plan: `docs/plans/_archived/20261127-split-deferred-open/plan.md`.

#### TODO

- [ ] On a host with memory to spare, revert the `gone` lines in `date_folder_ran` and run `cargo test --bin handpic folderops::tests::a_held_click_on_a_folder_a_split_removed` in `crates/app`. Confirm it fails (times out in `wait_until`), then restore the fix and confirm it passes.

### App: real-device Ctrl+C and window check of the app:watch task

#### Background

The `app:watch` mise task (the native-watch feature) runs the debug GPUI app under watchexec and rebuilds and restarts it on a `.rs`, `.toml` or `.json` change under `crates/app`, `crates/core` and `crates/engine`. It was checked by hand on Windows (Git Bash), but only from a non-interactive shell under `timeout`. That run covered the watchexec install, the launch, the restart on a source touch, no `os error 5` on the relink, no restart from the build's own output, and a folder argument holding spaces passed on every restart. The agent could see only processes and the app's log. Two things were never exercised: pressing Ctrl+C in a real terminal, and the window itself. A forced `taskkill /F` of `watchexec.exe` leaves `cargo` and the app running, because the job object is not kill-on-close. Ctrl+C is a different path (the console's ctrl event), so it needs a real terminal. Plan: `docs/plans/_archived/20261115-native-watch/plan.md`.

Files: `mise.toml` (`[tasks."app:watch"]`)

#### TODO

- [ ] On Windows (Git Bash), run `mise run app:watch` in a terminal and wait for the app's window. Press Ctrl+C in that terminal. Expect watchexec, `cargo` and `handpic.exe` all to exit (none left in Task Manager) and the prompt to return.
- [ ] On Windows (Git Bash), run `mise run app:watch`, save a `.rs` file under `crates/app/src`, and confirm the window reappears and repaints after the restart (the earlier check saw only the process and its log).

### Core: real-device check of the `.dop` flag on a PhotoLab virtual copy

`sidecar-and-keymap-decisions` Step 1 made `dop::read_flag` read a pick on any `Items` entry and `dop::write_rating` write `ShouldProcess` to every item, stamping every item's `ModificationDate`. The tests build a two-item `.dop` from a one-item fixture; no real virtual-copy sidecar was tried, and whether PhotoLab honours the per-item `ModificationDate` is unverified. The step's checkbox was ticked on the automated criteria. Plan: `docs/plans/_archived/20261123-sidecar-and-keymap-decisions/plan.md`. Files: `crates/core/src/dop.rs`, `docs/agents/photolab.md`.

#### TODO

- [ ] Pending manual check, with PhotoLab: give a frame a virtual copy in PhotoLab, close PhotoLab, change the frame's flag in Handpic (with `.dop` sidecars on), reopen the folder in PhotoLab and confirm it shows the new flag on both the master and the virtual copy. If it does not, file the per-item `ModificationDate` rule as the suspect.

### App: real-device checks of the four display fixes (todo-four-display-fixes)

#### Background

`todo-four-display-fixes` made a strip cell whose thumbnail encode panics read the panic's own message (libjpeg's) with no path, showed `The 1:1 view of this file could not be shown.` over the 1:1 canvas when the current file's crop read fails, reworded the `Rewrite Sidecars from Index…` dialog's third row to `N files with no judgment on record (skipped)`, and drew the strip no taller than leaves the preview `stripheight::PREVIEW_MIN` in a shorter window, the user's height kept. Unit tests cover each rule; nobody has seen them in the running app. The step's checkbox was ticked on the automated criteria only. Existing error rows keep the old text until Clear Cache or a change of the file (`EXTRACTOR_VERSION` was not bumped), so start from a fresh `--index`. Plan: `docs/plans/_archived/20261123-todo-four-display-fixes/plan.md`. Files: `crates/core/src/scan.rs`, `crates/app/src/zoom.rs`, `crates/app/src/empty.rs`, `crates/app/src/sidecars.rs`, `crates/app/src/stripheight.rs`, `crates/app/src/main.rs`.

Run each check with `--settings <fresh test store>`, `--index <fresh test index>` and `--log <test file>`, never the real store.

#### TODO

- [ ] On Windows, open a folder holding `NIKON_D70_Nikon.nef` and `CGO3P_YUN00007.dng` (malformed embedded JPEGs). Expect each strip cell and its hover text to read the libjpeg reason (`libjpeg fatal error: Not a JPEG file: starts with ...`) with no `panic` wording and no path.
- [ ] On Windows, press `z` on those files. Expect `The 1:1 view of this file could not be shown.` centered over the canvas; paging to a good file shows its crop and no message, and `z` off shows the fit view's own state.
- [ ] On Windows, drag the strip to `MAX`, then drag the window's bottom edge up. Expect the preview canvas to stay at 240 px or more; dragging the edge back down gives the strip its height back, and after a relaunch the stored height is still `MAX`.
- [ ] On Windows, run `Rewrite Sidecars from Index…` in a folder with scanned but never-judged files that have no sidecar. Expect the third row to read `N files skipped (no judgment on record, or a sidecar larger than 4 MiB)` (reworded by `sidecar-and-keymap-decisions` Step 2).

### App: measure shared_height against the strip wrapper's bounds on Linux (client-side decorations) and macOS

#### Background

`todo-four-display-fixes` draws the strip no taller than leaves the preview `stripheight::PREVIEW_MIN`, using `shared_height` in `crates/app/src/main.rs` (the viewport height less the client-side frame's top and bottom padding, `TITLE_BAR_HEIGHT` and `STATUS_ROW_HEIGHT`). The step's measurement ran only on GPUI's test platform on Windows (`bounds=936 computed=936` on every frame, through a temporary `canvas` in the wrapper `div` the `StripDrag` listener sits on). Nobody has run it on a real Linux window with client-side decorations, where `window_paddings` is non-zero, or on macOS. A mismatch would leave the preview a few px off `PREVIEW_MIN` in a short window. Plan: `docs/plans/_archived/20261123-todo-four-display-fixes/plan.md`. Files: `crates/app/src/main.rs` (`shared_height`), `docs/agents/app.md` (the strip-height section's "unmeasured" sentence).

#### TODO

- [ ] On Linux with client-side decorations, print the `StripDrag` wrapper's bounds height next to `shared_height(window)` (a temporary `canvas`, as in the Windows measurement). Expect the two equal on every frame, or correct `shared_height`.
- [ ] On macOS, do the same measurement and expect the same result.
- [ ] Replace the "unmeasured" sentence in `docs/agents/app.md`'s strip-height section with the result.

### App: the macOS / Linux manual checks for Move to Trash are still open

The strip's `Move to Trash…` moves files through the engine's mover and records `trash::Runs` runs, and that mover has never been run by a human on macOS or Linux (the app checks in `### App: real-device checks of the folder tree, the strip's selection and right-click menu, and Move to Trash undo / redo` are Windows only). On macOS the mover is `trash::trash_file` (`crates/engine/src/trash.rs`, `undo-trash-rejected` #547), which calls `NSFileManager.trashItemAtURL` directly rather than the `trash` crate's `DeleteMethod::NsFileManager`; one point is specifically unverified: whether the Trash's "Put Back" entry is actually created, since the `trash` crate documents that on some macOS systems files moved through `NSFileManager` get no "Put Back" entry (trash-rs#14); dragging them out of the Trash still restores them. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): through MCP `trash_photos` with `mcpAllowDestructive` on, the preview read `Move 1 file to the Trash?` / `Move to Trash`. A confirmed run moved the RAW, its `.xmp` and its `.ARW.dop` to `~/.Trash` and the strip showed the neighbouring file. `undo` restored all three with the reject flag and the 2 stars kept in the index and both sidecars. With a file of the same name placed at the original location, the undo reported `already exists at the original location`, left the two sidecars in the Trash and did not touch the placed file. The right-click path, the dialog's `Cancel` and Finder's "Put Back" were not run. Files: `crates/engine/src/trash.rs`, `crates/app/src/trash.rs`, `crates/app/src/main.rs`. Sources: `docs/plans/_archived/20260928-undo-trash-rejected/learnings.md` Step 2 (#547) and `docs/plans/_archived/20261113-gpui-tree-trash/plan.md`.

#### TODO

- [ ] On macOS, right-click a strip cell and pick `Move to Trash…`: the dialog opens with `Move to Trash` / `Cancel`; Cancel leaves the folder untouched; after a confirmed move, a "Put Back" from the Trash restores the file with its judgment, and if no "Put Back" entry exists, dragging it out does. (On 2026-11-26 the dialog's text, the move of the RAW with its `.xmp` and `.ARW.dop` and the neighbour shown were seen through MCP `trash_photos`, with no dialog and no right-click.)
- [ ] Verify the whole flow on Linux: the dialog asks `Move 1 file to the Trash?`, Cancel leaves the folder untouched, confirming moves the RAW plus its `.xmp` and `.ARW.dop` to the Trash and the strip shows the neighbouring file, and a restore from the Trash brings the file back with its judgment (the `trash` crate's freedesktop backend has never been run here).

### Core: face/eye-aware focus check for culling

Face/eye-aware detection and scoring (`crates/core/src/faces.rs`, `crates/core/src/sharpness.rs`) now runs at scan time: sharpness is scored on the Sony eye-AF frame, else on the AF point, else on the eyes of a detected face when there is no trusted AF point, else on the sharpest tile; an AF point off the face no longer scores a bystander's eyes (see `docs/plans/_archived/20260922-face-aware-sharpness/` and `docs/plans/_archived/20260924-face-catch-state/`). Without a trusted AF point the whole-preview search now runs at a 640x448 input on a DCT-scaled decode, which raised small-face recall at about 1.7x the earlier per-file cost; closed eyes were found to need a second model (see `docs/plans/_archived/20261005-face-detection-recall-cost/`), which now judges the shown file's eyes on demand, outside the scan, for the meta pane's `Eyes` row (see `docs/plans/_archived/20261007-closed-eyes-detection/`). What remains:

#### TODO

- [ ] Judge every face at or above the 60 px floor on the frames under the
      group threshold too, so a bystander's blink shows there. The
      all-faces check (pass 3, `crates/core/src/allfaces.rs`,
      `docs/plans/_archived/20261123-all-faces-check/`) already judges
      every such face, with worst-of counts in the meta pane, the strip's
      closed-eyes mark and the filter, but only on frames whose gate count
      reaches `groupMinFaces` (default 5); a frame with two to four faces
      still shows the judged face alone. Each face is one more model run
      (~35 ms). Still open too: a check of the accuracy on non-subject
      faces, which `eyes-truth.md` did not label. Files:
      `crates/engine/src/index.rs` (`allfaces_todo`), `crates/app/src/group.rs`.
- [ ] Measure the good-photo mark's precision on labels. Decision B rests
      on the user's stars and the mark rate; no labeled set was made. Done
      when 100-150 faced AF frames drawn stratified around the cuts and at
      random are labeled blind (`ok`, `miss` with `blur` / `focus` /
      `closed`, `turned`, `unsure`; the agent first, the user's review
      final) and the precision and coverage of the mark at its cuts are
      written with a Wilson interval, with `full_eye_sharp >= SOFT_BELOW`
      alone as the baseline (the procedure is in `docs/plans/_archived/20261008-burst-keep-score/plan.md` Step 5). Files: the plan
      folder of that work, `crates/engine/src/index.rs` if the cuts move.
- [ ] Re-check the good-photo cuts after the closed-eyes and head-pose label
      reviews (the items in the sections below). If the user's review
      moves `EYES_CLOSED_EAR` in `crates/core/src/eyes.rs` or changes the
      pose labels' frontal / oblique boundary, re-read `GOOD_EYE_EAR` (and
      `EYES_WIDE_OPEN_EAR` in `crates/app/src/meta.rs`, which anchors the
      openness against the core's `EYES_CLOSED_EAR`) and `GOOD_MAX_YAW` on
      the rated frames of `D:\photos\samples\ARW\good-mark-2026-10-09\`.
      Files: `crates/engine/src/index.rs`, `crates/app/src/meta.rs`,
      `crates/core/src/eyes.rs`.
- [ ] Faces rolled past 90 deg (a baby lying head-down,
      `D:\photos\2026\2026-07-11\_DSC2638.ARW`) get a mesh fitted as if
      upright, and the eye line cannot de-rotate them: `faces.rs` orders
      YuNet's eye points by image x, so the eye-line roll folds into -90..+90
      deg. They need another roll source (a mesh pass on rotated crops,
      MediaPipe's tracking loop). Not planned; see
      `docs/plans/_archived/20261010-mesh-roll/` (Decision). Files:
      `crates/core/src/faces.rs`, `crates/core/src/eyes.rs`.
- [ ] Review the mesh misfit labels: open the sheets
      `D:\Photos\tests\2026-10-09-mesh-roll\sheets\s00.png` to `s15.png`
      and compare each tile to its row of
      `docs/plans/_archived/20261010-mesh-roll/misfit-truth.md` (`before` / `after`
      `on` or `off` with its kind, and `yunet ok` / `yunet off`). The
      Decision's shares rest on the agent's labels; if labels change,
      re-run `compare.py` in that plan folder and update `compare.md` and
      the Decision if a number moves.
- [ ] Suggest the sharpest-eye frame within a burst group. Measured and not
      shipped (`docs/plans/_archived/20261008-burst-keep-score/`, Decision 1, `results.md` and `fit.md`): the user
      picks within a burst by timing, not by a technical feature; the
      burst's length, not sharpness or the eyes, predicts a kept scene
      (79% held out at 15 or more frames, which the burst band's count
      already shows, against about 56% for the best technical rule); and
      the non-picks are mostly technically fine. Revisit only with a new
      signal; the per-frame good-photo mark above covers "is this a miss".
- [ ] AF on a person in the background gives a sharp face and a false Good:
      the measure says the eyes of the face nearest the AF point are sharp,
      not that it is the subject. Files: `crates/core/src/candidate.rs`.
- [ ] Back-of-head and upturned faces find no face in the crop, so their
      Face focus state is Neutral (white), even when the camera tracked
      them. Files: `crates/core/src/candidate.rs`, `crates/core/src/faces.rs`.

Related: `crates/core/src/sharpness.rs`, `crates/core/src/faces.rs`, `crates/core/src/arw.rs`, `crates/engine/src/index.rs`, `crates/app/src/burst.rs`.

---

### Core: the review of the closed-eyes labels is still open

`closed-eyes-detection` (`docs/plans/_archived/20261007-closed-eyes-detection/plan.md`) set the closed-eyes threshold (0.137) and measured its AUCs on a truth set the agent labeled. The user's review of its `closed` faces is still open. Files: `crates/core/src/eyes.rs`, `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`.

#### TODO

- [ ] Review the faces labeled closed: for each file in the "Faces with a closed eye" list of `docs/plans/_archived/20261007-closed-eyes-detection/eyes-truth.md`, open its tile `D:\Photos\tests\2026-10-07-closed-eyes\tiles\<stem>.png` (or the crops under `arw\` / `dng\`) and confirm that the eye marked `c` shows no iris; note any that are open. If labels change, re-run `handpic-cli eyes` on `labeled-paths.txt` and `scratch\auc.py` there, and re-fit the threshold and the slope in `crates/core/src/eyes.rs` if the best F1 moves.

### Core: the review of the head-pose labels is still open

`head-pose` (`docs/plans/_archived/20261007-head-pose/plan.md`) measured the angles against 190 labels (159 readable) that the agent made from the face crops: yaw class and direction, pitch, roll, or `x`. The user's review is pending, and the plan's Decision rests on these labels. Step 2 was ticked on the automated criteria (the labels exist, the measurement and Decision are written, `mise run ci` passes).

Files: `docs/plans/_archived/20261007-head-pose/pose-truth.md`, `docs/plans/_archived/20261007-head-pose/pose-results.md`.

#### TODO

- [ ] Also check hand check 16 of `docs/plans/_archived/20261008-burst-keep-score/results.md`
      (`D:\photos\2026\2026-07-05\_DSC2445.ARW`): a profile looking up behind a ball
      reads pitch -68 deg. Decide whether `crates/core/src/pose.rs` should call that
      unknown or whether the label stands.
- [ ] On Windows, open the sheets `D:\Photos\tests\2026-10-07-head-pose\sheets\s00.png` to `s16.png` (4 tiles across, row by row in the `#` order of the `pose-truth.md` table; crops in `crops\`) and compare each tile to its row. Note any label to change. If labels change, re-run `aggregate.py` (in `D:\Photos\tests\2026-10-07-head-pose\`) and update `pose-results.md` and the plan's Decision if a number moves.

### Agents: confirm `pr-runner` waits for a review-round child's hand-back on a real `/pr` run

#### Background

The `long-wait-timeouts` fix is confirmed on 240 `pr-runner` and 225 `merger`
transcripts: every `wait-pr-actionable.sh` / `wait-post-merge-runs.sh` call
passed `timeout: 600000`, and the runs the harness still moved to the
background were `TaskStop`ped and re-run in the foreground. What remained was
`pr-runner` polling (`wait-pr-actionable.sh`, `sleep`, `until` / `while`,
`git ls-remote`, `gh pr view`) while a child it had dispatched was still
running. The `pr-runner-child-handback` plan added a rule to
`.claude/agents/pr-runner.md` that the runner ends its turn after an `Agent`
dispatch and waits for the child's hand-back.

An audit of the `pr-runner` transcripts from 2026-11-13 to 2026-11-27 found
158 runs with 207 dispatches (`pr-check-fixer` 84, `pr-conflict-resolver`
123), every one of which ended the turn and waited for the hand-back with no
polling in between. `pr-review-planner` and `pr-review-addresser` were never
dispatched, so the review-round path is still unexercised. Plan:
`docs/plans/_archived/20261128-agent-rules-from-transcript-audit/plan.md`

#### TODO

- [ ] On the next real `/pr` run that reaches a review round, read the
      `pr-runner` subagent transcript (the subagent `.jsonl` files under the
      Claude Code project directory for that session) and confirm that after
      each `pr-review-planner` / `pr-review-addresser` dispatch the runner
      ended its turn, the next event is the "[Subagent hand-back]" message,
      and there is no `wait-pr-actionable.sh`, `sleep`, `until` / `while`,
      `git ls-remote` / `git fetch` or `gh pr view` call in between. If not
      clean, adjust `.claude/agents/pr-runner.md`.

### Agents: confirm `pr-runner` script calls carry no `cd` prefix and are not doubled on a real `/pr` run

#### Background

The `pr-runner-verbatim-script-calls` plan extended `.claude/agents/pr-runner.md` §4 so that every script under `.claude/skills/**/scripts` it runs, the `wait-pr-actionable` call above all, is run exactly as written in its code block as a standalone Bash call: no pipes, no `tail`, no redirects, no `; echo exit=$?`, no `${PIPESTATUS[...]}`, no `;` / `&&` chaining. The rule exists because of PR #671, where `bash .claude/skills/pr/scripts/wait-pr-actionable.sh 671 | tail -12; echo exit=${PIPESTATUS[0]}` missed the `Bash(bash .claude/skills/pr/scripts/*)` allow rule and waited about 62 minutes before it ran. The script then printed `ACTION=ready` at about 150 s, but the call did not return until the 600 s timeout, most likely because a lingering child held the pipe open. A bare re-run returned in 2 s. Plan: `docs/plans/_archived/20261004-pr-runner-verbatim-script-calls/plan.md`

An audit of the `pr-runner` transcripts from 2026-11-13 to 2026-11-27 found none of the 3,137 script calls decorated with a pipe, `tail`, a redirect, `${PIPESTATUS[...]}` or `sleep`. What remained: 17 runs (127 calls) prefixed `cd <worktree> &&` though the cwd is already the worktree (still 3 of 24 runs after commit 739a17ee), 4 calls chained with other commands, and 2 runs that sent the same wait call twice in one message. The `agent-rules-from-transcript-audit` plan added two prohibitions to §4: no `cd` prefix, and never the same script call twice in one message. Plan: `docs/plans/_archived/20261128-agent-rules-from-transcript-audit/plan.md`

Files: `.claude/agents/pr-runner.md`

#### TODO

- [ ] On real `/pr` runs after the `agent-rules-from-transcript-audit` PR merges, read the `pr-runner` subagent transcripts (the subagent `.jsonl` files under the Claude Code project directory for that session). Confirm that no `.claude/skills/**/scripts` call carries a `cd` prefix and that no script call is sent twice in one message. If either recurs, strengthen the wording in `.claude/agents/pr-runner.md` §4.

### Core: Nikon crop-mode AF point offset is verified only on centered points

Crop-mode NEFs (Z6III DX / 1:1 / 16:9, Z50II 1:1 / 16:9 on raw.pixls.us)
showed that `AFInfo2`'s `AFImageWidth` / `AFImageHeight` stay at the
uncropped size while the JPEGs hold the crop. `nef::af_point` now reads the
crop from the MakerNote's `CropHiSpeed` / `CropArea` and moves the point by
the centered offset into the crop's frame, the AF area unscaled, a point
outside the crop giving none (`docs/plans/_archived/20261128-nikon-crop-af-point/plan.md`,
`docs/agents/raw-metadata-parsing.md`). The metadata confirms the crop is
the sensor's center, unscaled, but every sample's AF point was at the
center, where offset and proportional scaling agree. Files:
`crates/core/src/nef.rs`.

#### TODO

- [ ] Find a crop-mode NEF (DX, 1:1 or 16:9) whose AF point is off-center
      and confirm that the mapped point lands on the subject, i.e. that
      `AFAreaXPosition` / `AFAreaYPosition` stay in the uncropped frame;
      switch `nef.rs` to another mapping if not.

### Core: widen camera support from public sample RAW files

The user no longer owns the Sigma fp L or BF and cannot shoot new
samples, so public sample files are the verification source for more
cameras. Two kinds exist: raw.pixls.us (CC0; a few files per camera,
often flat test scenes that are weak for AF-point checks; CC0 means a
small file could be committed as a fixture, but tests prefer synthetic
bytes as in `crates/core/src/arw.rs`) and review-site sample galleries
(real scenes, good for AF checks, but not redistributable, so local
verification only). Riffle reads ARW, DNG, NEF, CR3, RAF and ORF today (README.md
"RAW formats"). The work splits into tiers, cheapest first:

1. More DNG-writing cameras (Pentax, Ricoh GR, other Leica bodies,
   phones). DNG reading already exists, so only preview/EXIF
   extraction needs verifying on samples.
2. AF point from MakerNotes that exiftool already decodes (Canon
   `AFInfo`, Nikon `AFInfo2`, Fujifilm `FocusPixel`, Olympus
   `AFPointSelected`, Panasonic `AFPointPosition`). Tag meaning and
   coordinate system are known; samples only confirm. This presupposes
   the RAW container is readable (tier 3), except for bodies that
   write DNG.
3. New RAW containers (CR3 = ISOBMFF, NEF, RAF, ...). Each needs a new
   parser next to `crates/core/src/arw.rs`; implementation outweighs
   verification.

AF data exiftool does not decode needs inference from many off-center
samples, as done for the Sigma BF `0x0147` in
`docs/plans/_archived/20260924-sigma-bf-af-point/plan.md`. Files:
`crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `README.md`.

#### TODO

- [ ] Tier 1: verify preview/EXIF extraction on DNG samples of the
      bodies raw.pixls.us did not hold in the 2026-11-27 survey: the
      Leica M11 family beyond the M11-P, the Q3 and the SL3, the Pentax
      KF (its samples there are PEF only), the Ricoh GR IV, and ProRAW
      from the iPhone 13 or later; add each working body to the
      `docs/humans/cameras.md` table. The Pentax, Ricoh GR, Leica and
      phone bodies raw.pixls.us holds are done and listed there.
- [ ] Tier 2: read the AF point from the exiftool-decoded MakerNote
      tags above, confirming on samples, for bodies whose container
      Riffle can already read. Canon `AFInfo2` (EOS bodies) and Nikon
      `AFInfo2` versions `03xx` / `04xx` (Z bodies) are done
      (`crates/core/src/cr3.rs`, `crates/core/src/nef.rs`). Left: the
      Nikon DSLRs' `AFInfo2` `0100` / `0101` (D850, D500), whose AF point
      is a grid point name rather than a position, and a sample with an
      AF position from the Nikon Z 8 and the Canon EOS R6 (their
      raw.pixls.us samples carry none). Fujifilm `FocusPixel` on the RAF
      bodies is done too (`crates/core/src/raf.rs`, in the embedded JPEG's
      frame); an autofocus sample from the X-T3 and the GFX 100 is still
      missing (theirs are manual focus). Next: the OM
      System / Olympus ORF bodies (`crates/core/src/orf.rs`): CameraSettings
      0x030a `AFTargetInfo` on the OM bodies (a 640x480 frame with the focus
      and selected areas) and 0x0305 `AFPointSelected` (percentages) on the
      Olympus bodies. Every raw.pixls.us sample puts the point near the
      center, so the origin and orientation of the frame are unconfirmed; an
      off-center landscape and a portrait sample are needed (see the Step 2
      survey in `docs/plans/_archived/20260930-olympus-orf/learnings.md`).
- [ ] Check the Sigma BF AF point's open assumptions against public BF
      samples: portrait orientation, manual-focus behavior, and the
      1000x667 scale (see the `SIGMA_BF_AF_GRID_W` doc comment in
      `crates/core/src/arw.rs` and the archived plan's
      [Trade-offs and risks](docs/plans/_archived/20260924-sigma-bf-af-point/plan.md#trade-offs-and-risks)).

### Core: Pentax and Ricoh DNGs show no lens name

#### Background

Found in the 2026-11-27 raw.pixls.us survey. The DNGs of the Pentax K-1, K-1 Mark II, K-3 Mark III, K-3 Mark III Monochrome, K-70 and KP and of the Ricoh GR III and GR IIIx carry no Exif lens name; the lens is recorded only in the Pentax MakerNote's `LensType`, which Handpic does not read, so the meta pane shows no lens for them (`docs/humans/cameras.md` says so).

Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `docs/humans/cameras.md`, `docs/humans/cameras.ja.md`.

#### TODO

- [ ] Read the lens from the Pentax MakerNote's `LensType` on the Pentax and Ricoh DNGs (exiftool's `Pentax.pm` holds the id table), check it against `exiftool` on the raw.pixls.us samples, and drop the "no lens name" sentence from both cameras docs.

### Core: Pentax and Ricoh DNGs read the full-size JPEG for every thumbnail and preview

#### Background

Found in the 2026-11-27 raw.pixls.us survey. The Pentax and Ricoh GR DNGs hold no small JPEG preview: IFD0 has only a 160x120 uncompressed image, and the one JPEG is the full-size one at the end of the file (2.9 to 6.4 MB on the samples, 6000x4000 to 7360x4912). Handpic therefore reads and decodes that JPEG for every thumbnail and every preview of these bodies. The Pentax MakerNote also holds a preview of about 52 KB, which Handpic does not use. Nobody has measured what this costs on a folder of such files.

Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`.

#### TODO

- [ ] Measure pass 1 and a page turn on a folder of Pentax or Ricoh DNGs against a body with a small preview. If the cost shows, read the Pentax MakerNote's preview for the thumbnail (its size first: under 1280 px on the long edge it cannot serve as the preview), as the Leica M (Typ 240) MakerNote preview is read.

### Core: Leica M10 shows no aperture, and four phone DNGs show no shooting settings

#### Background

Found in the 2026-11-27 raw.pixls.us survey.

- The Leica M10 sample has no `FNumber` and no `ApertureValue`; its aperture (5.6) is only in the Leica MakerNote, which Handpic does not read for it, so the meta pane shows no aperture (`docs/humans/cameras.md` says so). The M10-R records `ApertureValue` and is fine.
- On the HUAWEI ANE-LX1, OnePlus A6003, Xiaomi M2010J19CG and Google Pixel 2 XL DNGs, `reader::read_metadata` returns the make and the model but none of the shooting fields (exposure, aperture, ISO, focal length, exposure bias, capture time), although `exiftool` reads them. These four also have no embedded JPEG preview, so they are outside what Handpic supports and are not listed; the missing fields may still point at an Exif layout other DNGs share. The cause is not known.

Files: `crates/core/src/arw.rs`, `crates/core/src/reader.rs`, `docs/humans/cameras.md`, `docs/humans/cameras.ja.md`.

#### TODO

- [ ] Read the Leica M10's aperture from its Leica MakerNote, check it against `exiftool`, and drop the "no aperture" sentence from both cameras docs.
- [ ] Find where `exiftool` reads the shooting fields of the four phone DNGs from, and decide whether Handpic should read them there. Done when the cause is written here or fixed.

### Docs: cameras.ja.md lags behind cameras.md

#### Background

`docs/humans/cameras.ja.md` still has the policy paragraph from before #656 ("2010 年より前"), about 30 pre-2016 Fujifilm and FinePix rows, and two extra rows in the embedded JPEG size table, all of which `docs/humans/cameras.md` no longer has. Found while adding the 2026-11-27 raw.pixls.us survey's DNG bodies, which touched only the rows and paragraphs that change added.

Files: `docs/humans/cameras.md`, `docs/humans/cameras.ja.md`.

#### TODO

- [ ] Bring `docs/humans/cameras.ja.md` in line with `docs/humans/cameras.md`: replace the policy paragraph, delete the delisted rows and the two size rows, and fix the sentences that name them.

### Core: M-RAW RAF files are unverified

#### Background

No raw.pixls.us sample of the listed Fujifilm bodies is an M-RAW (multi-image) RAF: the M-RAW header words at `0x48` / `0x4c` are zero on all 46 samples of the `fujifilm-raf` Step 1 survey (`docs/plans/_archived/20260930-fujifilm-raf/learnings.md`). `raf::parse` reads the one embedded JPEG and ignores the M-RAW header, which ExifTool suggests is enough, but no such file has been opened. Files: `crates/core/src/raf.rs`.

#### TODO

- [ ] Open an M-RAW RAF (from a body that writes one) and confirm the preview, the 1:1 view and the EXIF rows.

The `raf-older-bodies` sweep of 228 RAFs from 87 bodies (`docs/plans/_archived/20260930-raf-older-bodies/learnings.md`) found no M-RAW either: `0x48` is zero on all of them.

### Docs: Sony AF point and face tracking are unconfirmed for lack of samples

#### Background

The `sony-arw-coverage` verification (`docs/plans/_archived/20260930-sony-arw-coverage/plan.md`) marks these `–` in `docs/humans/cameras.md` because the raw.pixls.us samples don't show them:

- Every α9 III and α7CR sample is manual focus (`FocusMode` 0, point at the exact center, `FocusFrameSize` invalid). `trusted_focus` drops the point as designed, so both bodies show `–` for AF point and AF frame size.
- Face tracking: the raw.pixls.us samples hold no `AFTracking` 1. The local samples later checked in `sony-face-tracking-docs` (`docs/plans/_archived/20261001-sony-face-tracking-docs/plan.md`) record it on the α7 IV, α7R V, α7S III, α6700 and ZV-E1, so those show `✓`. The α1, α9 III, α7C II and α7CR stay `–` because no sample of them recorded it.

Files: `docs/humans/cameras.md`.

#### TODO

- [ ] Find an AF-C sample of a person from the α9 III and α7CR (portrait orientation if possible), and turn their `–` for AF point / AF frame size into `✓` if the point lands on the subject.

### Engine: PhotoLab Uuid lookup on macOS: real-device checks and external volumes

`crates/engine/src/photolab.rs` looks up the registered Uuids on macOS too
(`docs/plans/_archived/20261005-photolab-uuids-macos/plan.md`), verified by
hand on macOS (PhotoLab 10, 2026-10-05; see `docs/agents/photolab.md`). Paths
under `/Volumes` get no lookup yet, so a fresh `.dop` there still gets random
Uuids and may become a virtual copy.

#### TODO

- [ ] Real-device check: give a source a virtual copy in PhotoLab and confirm
      the master is the lowest `ZDOPINPUTITEM.Z_PK`.
- [ ] External volumes: open a folder on `/Volumes/X` in PhotoLab, find how it
      records the volume (`ZTYPE`, `ZNAME`, `ZUNIQUEID` in `ZDOPFOLDER`), then
      extend `folder_names` / `query`'s macOS mapping in
      `crates/engine/src/photolab.rs`.

### Engine: PhotoLab after a file or folder rename

From Step 3's check of the PhotoLab Uuid lookup (`crates/engine/src/photolab.rs`
queries `Sources` by `FolderId` and `Name`; `crates/core/src/dop.rs` writes
the RAW's `Name` into the `.dop`). The Tauri app's `rename_file` moved the
`.dop` unchanged (the app has no file rename yet, `not_in_app`),
so its inner `Name` still named the old file and PhotoLab's database still
held the old name. A folder rename (`crates/engine/src/rename.rs`) leaves
PhotoLab's database holding the old folder.

#### TODO

- [ ] Find out (checked with PhotoLab) whether PhotoLab re-matches a renamed
      RAW with its moved `.dop` or imports it as a new image / virtual copy,
      and either patch the `.dop`'s `Name` on rename or document what to
      expect.

### Core: read the cue-labels round once the user has labeled it

#### Background

The `cue-labels` plan (`docs/plans/_archived/20261128-cue-labels/plan.md`) prepared a blind labeling round for the cue readings (head pose, closed eyes and the rest), which the user asked for on 2026-11-28 after the `bad-photo-labels` Decision left it open (`|pitch|` > 45 read 3.2x the strict base rate on 19 frames, interval [0.0, 40.5]). The round is 900 shuffled RAW copies in `D:\Photos\tests\2026-11-28-cue-labels\sample`, with `manifest.tsv` and `strata.tsv` beside it (outside the repository), drawn by `sample.py` from the 2026-10-12 app dump in three strata on the dump's readings: `pitch45` (`|pitch|` > 45, 300 of 434), `closed` (EAR <= 0.137, 100 of 1,068) and `rest` (500 blind controls of 14,968, per folder). The question is "OK to discard?", as in `discard-labels`: rating 1 = keep, unmarked = OK to discard.

What a reader of the labels needs: the population is the 19 folders of `D:\Photos\2026` still intact since the dump (the eight culled folders hold only what the user kept), less the 2,400 frames of the three earlier rounds, 16,470 frames; and today's app reads fewer poses than the dump (in `2026-09-27-a`, 16 of 94 dump `pitch45` frames still have a pitch), so the today's-reading bins have wide intervals. The question, the readings, the bars and the follow-up rule were pre-registered before any label in `docs/plans/_archived/20261128-cue-labels/cues.md`; the guide is `docs/plans/_archived/20261128-cue-labels/labeling.md`; the run's checks, the manifest's SHA-256 and the strata table are in `docs/plans/_archived/20261128-cue-labels/learnings.md`.

This waits on the user.

#### TODO

- [ ] The user labels the sample folder per `labeling.md` and says "done".
- [ ] Then write a plan that exports the labels (`labels.py`, as the archived rounds' ones), reads today's values of the sampled copies, computes the pre-registered readings of `cues.md` and writes the Decision by its follow-up rule, changing no bar.

### App: `uitest::opens_with_no_folder` hung on `app (ubuntu-latest)`

#### Background

Run 45091844682 attempt 1, `app (ubuntu-latest)` (job 137424185183, 2026-11-23, cold cache), logged `test uitest::opens_with_no_folder has been running for over 60 seconds` at 01:17:55 and was cancelled at the job's 30-minute limit at 01:32:39. The rerun passed. Found by the `ci-apt-mirror-timeout` scan of rerun and failed CI runs (plan: `docs/plans/_archived/20261127-ci-apt-mirror-timeout/plan.md`); how to read such a job is in `docs/agents/github-actions-workflows.md` ("A job that hits its timeout"). The UI test harness now has a watchdog: a harness phase past 60 s writes `UI test watchdog: phase ...` with that test's log lines to stderr and aborts the test binary. What was ruled out (the bounded waits of the harness and the quit handlers) and the unbounded `join`s still to suspect are in the plan, `docs/plans/_archived/20261128-uitest-watchdog/plan.md` ("Trade-offs and risks"). Files: `crates/app/src/uitest.rs`.

#### TODO

- [ ] When the watchdog next fires in CI, read the phase and the log lines from the job log, find the cause and fix it. Done when the cause is fixed.

### Core: re-measure the import copy pipeline against Explorer on a fast destination

#### Background

The import-from-card feature's `riffle-cli copybench` measured the copy pipeline (8 MiB x 4 buffers, per-file `sync_all`) only against `D:` (a Crucial BX500, DRAM-less, 96 % full). That drive capped every method at 8-22 MB/s, including `robocopy`, while the card read alone ran at 135 MB/s. The numbers cannot tell the methods apart, and Explorer itself was never timed. The defaults stay at 8 MiB x 4, and `FILE_FLAG_SEQUENTIAL_SCAN` and dropping `sync_all` showed no gain on that drive. Plan: docs/plans/_archived/20261106-import-from-card/plan.md.

Files: `crates/core/src/copy.rs`, `crates/cli/src/main.rs`, `docs/humans/performance.md`.

#### TODO

- [ ] Copy the same fresh, never-read files from the card into a destination on a faster or emptier drive (the NVMe `C:`) by `handpic-cli copybench <card-dir> <dst-dir>` and by an Explorer copy, with idle time between runs, and record the MB/s. Done when the table in `docs/humans/performance.md` has the Explorer row and the 8 MiB x 4 defaults and the per-file `sync_all` are confirmed or changed.


### Core: the cold-disk cost of pass 2's full-size eye read is unmeasured

#### Background

`full-size-eye-check` Step 3 measured pass 2 on `D:\Photos\2026\2026-11-07` (3638 ARW) with a warm file cache: a measured frame cost about 51 ms more (mean and median), and the whole `riffle-cli features` run went from 80 s to 98 s. The cost of the second read on a cold disk is unmeasured, and the plan's "Cost" trade-off names it as the case to watch. Plan: `docs/plans/_archived/20261108-full-size-eye-check/plan.md`. Files: `crates/core/src/scan.rs` (`full_eye_sharp`).

#### TODO

- [ ] Time `handpic-cli features` (or a pass 2 in the app) on a freshly mounted card or after a reboot, with the same 8 threads and release build as Step 3, and compare it with the warm figures. If the second read is a noticeable share of the cost, consider reusing the first read's bytes.

### App: a JPEG folder's strip cells in view stay gray until pass 1 ends

#### Background

Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): on `jpeg500` (500 JPEGs, hardlinks of cached files, fresh index, debug build) the strip cells in view at positions 3 to 9 stayed gray placeholders until pass 1 ended (`scan extract … in 89963ms`), while cells 1, 2, 10 and 11 had thumbnails early (seen at `scanning 492/500` and `498/500`); the hot list should have taken the cells in view first.

Files: `crates/app/src/focus.rs` (`scan_focus_paths`), `crates/app/src/main.rs` (`scan_focus_paths`, the `SCAN_FOCUS_DELAY` task), `crates/app/src/thumbs.rs`, `crates/engine/src/index.rs` (`run_scan`), `crates/core/src/scan.rs` (`ScanFocus`).

#### TODO

- [ ] On Windows, re-run the 500-JPEG fresh-index open (`jpeg500`, debug build, 1600 px window, `--log <path>`) and, while cells in view stay gray during pass 1, read `Handpic.log` for the two diagnostics lines added by `docs/plans/_archived/20261127-jpeg-strip-gray-cells/plan.md`: `thumbnail read failed, the cell waits for the scan:` (`crates/app/src/thumbs.rs`, a reader error other than no row) and `thumbnail not scanned after the scan reported it: path=… retried=false` (a `[WARN]` line; `retried=true` is the benign `info` case; `crates/app/src/main.rs` `thumbnail`, a cell left missing until the pass ends). Either names the cause to fix. With neither line for the gray cells, their rows were not written before the pass ended, so look at the scan order (the hot list) instead; a UI test on 300 generated JPEGs, whose pass 1 took about 2 s, filled every cell in view within 60 ms of its row (`learnings.md` of that plan).

### App: measure the scan pause in `load` on the SATA SSD with presses on files pass 1 has not read yet

`gpui-strip-index` Step 5's during-scan run stepped to the next file, which the hot list had already made the scan read (a cache hit of about 1 ms), so `ReadSlots::pause` around `read_preview` showed no effect at p50 / p95. Tauri's figure (14-16 ms read wait at cap 2 with the pause) is for a read of an unscanned file on the SATA SSD. On 2026-11-23 the comparison was made on the NVMe drive instead: release builds at cap 2, pause on against off, MCP `show_photo` jumps to files pass 1 had not read, cold copies of 1800 ARW, and the pause made no measurable difference (8.8-9.6 against 9.3 ms p50 at about 10 % CPU load; `docs/agents/app.md`, "A preview holds the scan's pause until its image is painted"). The SATA runs could not be taken: another process was reading `D:` at 250 to 480 MB/s throughout (`docs/plans/_archived/20261123-todo-measurements/learnings.md`, with the driver and the pause-off patch). Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/app/src/main.rs` (`load`), `docs/agents/app.md`.

#### TODO

- [ ] On the SATA SSD (`D:`), with nothing else reading it, repeat the 2026-11-23 method: equal cold copies (`robocopy /J`, one per run, under `D:\Photos\tests\`), a settled disk, pause on against off alternated, at least two runs each, and record `read` p50 / p95 next to Tauri's in `docs/agents/app.md`, saying whether the pause shortens the read there.

### App: `present` p95 rose from about 0.2 ms to 0.55-0.85 ms on ARW with the strip

In `gpui-strip-index` Step 5's A/B (`results.md`), `present`'s p95 on the ARW compare folder rose with the strip while RAF was unchanged. It is not gated and stays under 1 ms. Plan: `docs/plans/_archived/20261110-gpui-strip-index/plan.md`. Files: `crates/app/src/main.rs` (`paint_strip`).

#### TODO

- [ ] When a later strip change (marks, badges) lands, re-run the same-session A/B and check that `present` p95 has not grown past 1 ms; if it has, look at `paint_strip` first.

### App: by-eye checks of the scaled fit-view decode (fit-preview-downscale)

#### Background

`fit-preview-downscale` made the fit view decode the preview JPEG at the smallest `n/8` DCT scale that covers the preview canvas in device pixels (Step 2, `crates/app/src/fitscale.rs`), re-reading the current file after a debounce when the canvas grows (`refit`, `refitted`, `fit_settled` in `crates/app/src/main.rs`). Steps 2 and 3 were ticked on their automated criteria (unit tests, `--auto` runs, the Step 3 latency A/B, which checked the `page` lines' `decoded=` sizes). Nobody has looked at the window: the image quality at each scale, the sharpening after a resize, and the overlays and the 1:1 view over the smaller image are unverified. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`): on the RAF compare folder, a debug build, through MCP `set_overlays` and `screenshot`: the grayscale latch the `g` key's hold shares drew the photo gray in the fit view and in the 1:1 crop with the strip thumbnails and the peaking overlay in color; the peaking layer showed in the fit view and at 1:1, and after `set_panes` hid both sides and the strip (the canvas growing from about 1150x765 to 1400x935 logical px) it was redrawn at the new size, its lines thin and on the edges. The boxes below were not run: they compare against a build from before `d3794643` or need captures during a resize. Plan: `docs/plans/_archived/20261121-fit-preview-downscale/plan.md`. Files: `crates/app/src/main.rs`, `crates/app/src/fitscale.rs`, `crates/app/src/peaking.rs`, `crates/app/src/zoom.rs`.

#### TODO

- [ ] On Windows, a release build on `D:\Photos\tests\20261108-gpui-compare\RAF` with prefetch on, run from a terminal so the `page` lines show. Page through it, resizing the window so the `page` line's `decoded=` shows 3/8, 4/8 and 6/8 of `px=`. Expect no visible softness or aliasing against a `main` build from before `d3794643` at the same window size.
- [ ] Same folder: press `z`. Expect the placeholder at the same place and size as before `d3794643` (softer), then the crop; page while zoomed and expect each page's placeholder then crop.
- [ ] Same folder: `f`. Expect the face boxes, the landmark dots and the AF mark where a build from before `d3794643` puts them.
- [ ] Same folder: enlarge the window, hide the side panes (`toggleSides`) and drag the strip down. Expect the image soft for at most about 150 ms plus a decode, then sharp, with no flash, no meta pane flicker and the `f` marks kept.

### App: real-device checks of the strip's sort order, the filter and the filter / sort menus

`gpui-filter-sort` added the strip's sort order, the filter (flags, stars, labels, orientation, `Face focus`, `Eyes`, `Output`) and the status row's `Sort ▾` / `Filter ▾` menus to the GPUI app (`crates/app`). CI, the unit tests (`sort.rs`, `filter.rs`, `menu.rs`) and debug `--auto` runs covered the page order and the log lines headlessly. Clicks and key presses cannot be driven by `--auto`, so nobody has watched the strip's drawing, pressed a judgment key under a sort or a filter, or clicked a menu. On Windows 11 on 2026-11-22 the sort order, the filter through the judgment keys and undo, `Face focus` and `Eyes`, both menus and the `Output` section were exercised on a debug build with synthetic input (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`). On 2026-11-23 the MCP companion (a `--features screenshot` debug build, `get_view` and `screenshot`) covered the `Face focus` box, a pick then `Picked`, and `Sort ▾` rating on a folder longer than the strip (results in `docs/agents/app.md` and `docs/plans/_archived/20261123-todo-measurements/learnings.md`); what is left open is in the boxes below, the parts that need a real mouse or keyboard. The good set of `faces` is now `ILCE-1M2_sony_a1_ii_03` and `_DSC1758` (`DSC00405` is no longer Good; the rule changed after the check was written), the mark is the green `scan-face` icon rather than an `f` mark, and a click on a strip cell that closes the filter menu also selects the cell. Windows only. Plan: `docs/plans/_archived/20261113-gpui-filter-sort/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/sort.rs`, `crates/app/src/filter.rs`, `crates/app/src/menu.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build`, then run `target\debug\handpic.exe <folder> --index <fresh path> --settings <fresh .sqlite path> --import-settings <file>`, with a fresh index and settings store per run (the store is seeded from the `--import-settings` JSON only when new); "with `settings-x.json`" below means `--import-settings` that file. The folders and settings files below are copies under `D:\Photos\tests\20261113-filter-sort\`; use only those, and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`. Record what was seen in `docs/agents/app.md`, above all whether a click inside the deferred popover reaches its `on_click` before the strip's `on_mouse_down`.

#### TODO

- [ ] The menus (Steps 3 and 4): open `menu\raw` with `settings-name.json`. If `Filter ▾` can be made to open over a strip cell, a click on an item does not also select the cell under it (in the 2026-11-22 run the menu opened upward over the preview and the meta pane and never overlapped the strip). With the filter menu open, hold `→` on a real keyboard: the pager pages continuously.

### App: real-device checks of the EXIF filter sections and the refilter fixes

`gpui-exif-filter` added the per-folder EXIF sections (`Camera`, `Lens`, `Aperture`, `Shutter speed`, `ISO`, `Focal length`) to the GPUI app's filter menu, made the strip position (then the meta pane's `n / total`, now the status bar's, see `native-status-position`) follow a refilter that hides the shown file, and kept the anchor across an empty list (`filter::KeptAnchor`). CI, the unit tests (`exif.rs`, `filter.rs`, `menu.rs`) and one debug `--auto` run covered the build, the group order, the check semantics and the pure rules headlessly. Clicks and judgment keys cannot be driven by `--auto`, so no one has opened the menu, checked an item or pressed `p` under a filter. On Windows 11 on 2026-11-22, on a copy of `sort\raw` (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`), the menu listed `Camera`, `Aperture`, `Shutter speed`, `ISO` and `Focal length` with the folder's values in order and scrolled (no `Lens`: the compacts record none); camera items widened the strip (OR), an ISO item narrowed it (AND) and `Reset` cleared all; a copied-in ILCE-6400 RAW added its item, and deleting that only file dropped its check; under `untagged` a `p` changed the position from `3 / 5 (6)` to `3 / 4 (6)` in the first frame captured; `Reset` from a filter hiding every file came back to the file shown before; the settings file was untouched. On 2026-11-23 the MCP companion (a `--features screenshot` debug build, `get_view`, `set_filter`, `set_judgment` and `screenshot`) saw the `lens` section listed after `camera` in `filter_offers.exif` once pass 1 of a fresh index ended, the name follow the position under `untagged` after a `set_judgment` pick, the uncheck from a filter hiding every file come back to the file shown before, and six picks under `untagged` plus `Face focus` during pass 2 end on the current file's image (results in `docs/agents/app.md` and `docs/plans/_archived/20261123-todo-measurements/learnings.md`). What is left open is in the box below: no tool opens the menu. Windows only. Plan: `docs/plans/_archived/20261113-gpui-exif-filter/plan.md` (the pending manual checks in `learnings.md` in the same folder). Files: `crates/app/src/exif.rs`, `crates/app/src/filter.rs`, `crates/app/src/menu.rs`, `crates/app/src/main.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build`, then run `target\debug\handpic.exe <folder> --index <fresh path> --settings <file>`, with a fresh index per run. Use only copies under `D:\Photos\tests\` (a folder with several cameras, such as `D:\Photos\tests\20261113-filter-sort\sort\raw`), and never open a folder under `D:\Photos\` outside `D:\Photos\tests\`.

#### TODO

- [ ] On Windows, on a folder whose files record a lens, open `Filter ▾` and see the `Lens` section drawn after `Camera` (the sections' order and their appearance at the end of pass 1 were seen through `get_view` on 2026-11-23; the drawn menu was not).

### App: `Camera` / `Lens` order is a case-insensitive compare, not the Tauri `localeCompare`

The app's EXIF filter orders the `Camera` and `Lens` items by a case-insensitive compare with a plain compare as the tie-break, because `crates/app` has no ICU collator. Tauri used `localeCompare`, so the two may differ for punctuation or non-ASCII model names. Found in `gpui-exif-filter` (the plan's trade-off). Plan: `docs/plans/_archived/20261113-gpui-exif-filter/plan.md`. Files: `crates/app/src/exif.rs` (`compare`).

#### TODO

- [ ] Compare the two orders on a folder with punctuated or non-ASCII camera / lens names, against the reference Tauri build named under "App: real-device checks of the f focus mark, face boxes and face mesh" (an installed Riffle 1.10.0, or a build of the commit before #1150) or against `Intl.Collator` / `localeCompare` results for the same names, and, if they differ in a way that matters, order them with a collator (`icu_collator`) or a closer rule.

### App: real-device check of the Japanese IME in the settings' label name fields

`gpui-settings` gave the GPUI app (`crates/app`) its own settings store (a SQLite file seeded once from the Tauri `settings.json`), persisted the strip's filter and sort order in it, and ported the settings modal: the Culling, Sidecar, Keyboard Shortcuts and Import tabs, opened with Ctrl+, (Meta+, on macOS). CI and the unit tests (`settingsmodal.rs`, `settings.rs`, `textinput.rs`, `filter.rs`, the engine's `store.rs`, `sidecar.rs` `switch_to` and `import.rs` `today_example`) cover the pure models and the store. The store, the settings' shell, the Culling, Sidecar, Keyboard Shortcuts and Import pages, the text fields and the `Lightroom language` `Select` were exercised on Windows 11 on 2026-11-21; only the Japanese IME's composition in a text field was not. Windows only. Plan: `docs/plans/_archived/20261113-gpui-settings/plan.md` (the steps are in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/settings.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/settingsview.rs`, `crates/app/src/textinput.rs`, `crates/engine/src/store.rs`, `crates/engine/src/sidecar.rs`, `crates/engine/src/import.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` first. Work only on copies under `D:\Photos\tests\20261113-settings\` with a fresh `--index` per run and an explicit `--settings <test store>.sqlite` (plus `--import-settings <test json>` where a seed is needed); never the default index or store, never the real Tauri `settings.json`, and never a folder under `D:\Photos\` outside `D:\Photos\tests\`. In every command below, `...\` stands for `D:\Photos\tests\20261113-settings\`, and the run is `target\debug\handpic.exe <folder> --index ...\<index> --settings ...\<store> --log ...\Handpic.log`.

#### TODO

- [ ] On Windows, the Japanese IME in a label name field. Run on a folder of RAW copies with a fresh `--index`, `--settings` and `--log`, format XMP. Ctrl+, → Sidecar; with the Japanese IME on, type a name in the Yellow field: the composition is underlined, the candidate window sits at the field, Enter commits the composition (not the field), a second Enter commits the field.

### App: real-device checks of the folder tree and Move to Trash… left open (gpui-tree-trash)

`gpui-tree-trash` gave the GPUI app (`crates/app`) the left-hand folder tree (roots, expand, RAW count, watchers, the keyboard, a right-click menu with `Expand All` / `Collapse All`), the strip's multi-selection, the strip's right-click menu, `Move to Trash…` / `Delete Permanently…` with the hold during a scan, and Trash undo / redo. CI and the unit tests (`tree.rs`, `treekeys.rs`, `keys.rs`, `context.rs`, `selection.rs`, `strip.rs`, `judgment.rs`, `trash.rs`, `idle.rs`, `history.rs`, `resync.rs`, and the engine's `folders.rs`, `treewatch.rs`, `trash.rs`) cover the pure models. Only the `notify` 8.2.2 collapse / re-expand timing and the `SHParseDisplayName` rejection of the `\\?\` prefix were measured headless. On Windows 11 on 2026-11-22 the tree, its menus and the trash dialog, Explorer, the clipboard, the Recycle Bin and the keyboard focus model were exercised on a debug build with synthetic input (results in `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`); what is left open is in the boxes below. The run's findings are their own items: the Sort / Filter menus opening with no folder (settled: they do not open, see `docs/agents/app.md`), the Undo's count and reason after the Recycle Bin was emptied by hand (the item below), and the held rename's `Access denied` (under the inline rename item). Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`): an `open_folder` (the tree click's `open_from_tree`) at `analyzing 45/400` stopped the first folder's pass 2 and reopening it resumed the 354 files left, and a launch with no folder quit on the window's close; a `trash_photos` confirmed during pass 1 was held with `Move to Trash: waiting for the scan to finish`, the scan was canceled and the file moved, and with another folder opened before the scan ended the run was dropped (`dropped: another folder was opened`), the note cleared and the file stayed; a 3-star file restored by `undo` came back with its `.xmp` and its three stars in the strip, a run made in `a` and undone from `b` left `a` listing the file when reopened, and a `redo` reported `moving to Trash 0 / 1`, then `Moved 1 file to the Trash`. Not run: the dialog (an MCP-confirmed run opens none), the delete's judgment-undo and the Ctrl-click. Windows only. Plan: `docs/plans/_archived/20261113-gpui-tree-trash/plan.md` (the full steps and their expected results are in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/tree.rs`, `crates/app/src/treekeys.rs`, `crates/app/src/context.rs`, `crates/app/src/selection.rs`, `crates/app/src/trash.rs`, `crates/app/src/idle.rs`, `crates/app/src/history.rs`, `crates/engine/src/folders.rs`, `crates/engine/src/treewatch.rs`, `crates/engine/src/trash.rs`.

Build in `crates/app` with `PATH="/c/Users/daisu/AppData/Local/bin/NASM:$PATH" mise exec -- cargo build` first (rename a locked `handpic.exe` aside, never kill it). Work only on copies under `D:\Photos\tests\20261113-gpui-tree-trash\` with a fresh `--index` per run and an explicit `--settings` test file; never the Tauri index or the app's default index. The tree may list and watch the roots and the ancestors of a test folder; open (click the name of, or press Enter on) only folders under `D:\Photos\tests\`. Trash and delete only the copies.

#### TODO

- [ ] On Windows, the rest of the tree's disk following and its right-click menu (Step 3). Under `RUST_LOG=debug`, with `...\tree` collapsed, create `e` in Explorer: no re-list in the log. With a row's menu open, a right press on another row opens that row's menu. Switch to another app and back: no flicker or collapse; a USB drive plugged in meanwhile appears as a new root.
- [ ] On Windows, the rest of the tree's keyboard (Step 4). With the tree holding the keyboard, a click on the preview hands the keys back, and the right-click rules hand the keys back / keep them as in the learnings. After `Expand All` on `b`, a folder created in Explorer under an expanded one still appears.
- [ ] On Windows, the rest of the strip's right-click menu (Step 6). With the strip's menu open on `...\menu\`, a press on the preview or `escape` closes it, and a left click on a cell closes it and still pages.
- [ ] On Windows, the rest of Move to Trash… and Delete Permanently… (Step 7). With the dialog open, clicks on the tree, strip and status menus do nothing. On `...\trash-large` with a fresh `--index`, during pass 1 pick `Move to Trash…` and confirm: the dialog closes at once and `moving to Trash 0 / 1` shows briefly once the held run starts. Judge a file, then `Delete Permanently…` it: the earlier judgment is no longer undoable. (The waiting note, the held run and the drop by a folder open were seen through MCP on 2026-11-27, with no dialog.)
- [ ] On Windows, the rest of the strip's selection (Step 5). With several files selected, Ctrl-click a middle one: it drops out and the focus stays on the current file. (The restore with its stars, the cross-folder undo and the redo's progress were seen through MCP on 2026-11-27.)

### App: real-device check of the restored window on a second monitor (gpui-session-restore)

#### Background

`gpui-session-restore` made the GPUI app (`crates/app`) reopen the last folder and the window's position, size, display and maximized state on a launch without arguments (`session.rs`, the `lastFolder` and `windowBounds` keys of the settings store, written on open and in the quit handler). Step 1 was ticked on its automated criteria (the `session.rs` unit tests, `mise run ci`, `mise run app:check`) and a hand check on one Windows 11 display at 96 DPI: a moved and resized window, a maximized quit, a minimized quit, bounds off every display, an unknown display UUID, a relaunch without arguments, a renamed remembered folder, and `--auto 1 <other folder>`. A quit with `q`, a snapped window, a taskbar docked left, the paged file and `--auto` were seen on Windows 11 on 2026-11-21 and recorded in `docs/agents/app.md`; a second monitor, especially one with a different scale factor, was not available. macOS and Linux are covered by the CI build and the unit tests only. Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store. Record the results as `(Measured)` in `docs/agents/app.md` ("A launch restores the last folder and the window") and remove that section's "Not seen yet" bullet. Plan: `docs/plans/_archived/20261114-gpui-session-restore/plan.md`. Files: `crates/app/src/session.rs`, `crates/app/src/main.rs`, `crates/app/src/settings.rs`.

#### TODO

- [ ] On Windows with a second monitor, ideally with a different scale factor, move the window there, quit, relaunch, both windowed and maximized. Expected: the window comes back on that monitor, windowed and maximized respectively.

### App: real-device checks of the status row's StatusBar and progress circle

`native-status-bar` rendered the GPUI app's status row (`crates/app`) through gpui-component's `StatusBar` and replaced the 2 px top-edge progress bar (`gpui-progress-bars`) with a 12 px `ProgressCircle` in the bar's right region, left of `Sort ▾`. CI, the `progress.rs` and `theme.rs` unit tests, a measured preview canvas (1160 x 842 in the default 1600 x 1000 window) and screenshots of the row, a forced-shown circle and a long status text cover the build. The menus, a live scan (the circle's fill and the pass 1 to pass 2 restart), the Move to Trash run and paging cost during a scan were never exercised by hand. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`) the row showed as a bar with a top border and muted small text, and both menus, a tall `Filter ▾` too, opened upward with their right edge on the button's, unclipped, and closed on a press outside; on a cold 600-file folder pass 1 took about 1.1 s, too short to see the circle, which filled from empty through pass 2 (161 s) and went at the end; a 600-file `Move to Trash…` with the right pane hidden showed the run's progress in the circle, which went at the end. When the circle goes, the position text shifts right by about 20 px (settled: the row now always reserves the circle's slot, see `docs/agents/app.md`), and after the folder emptied the status line read `no RAW files in \\?\D:\Photos\tests\20261122-strip\cold` (its own item, after the Import dialog's `Source:` item). On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-7/results.md`) a cold 600-file folder's pass 1 (4.3 s) filled the circle from empty to full, and at pass 2's start it drained to empty within about 240 ms (frames 80 ms apart, consistent with the 180 ms animation) and filled again from empty, which was judged to read as a reset; paging during pass 2 with the circle filling took 12 to 29 ms keypress to pixels (`source=read`) against 5 to 21 ms after the scan (after a first 140 ms page, `source=prefetch`), judged not visibly slower on a debug build (paging during pass 1 was not taken); a 300-file `Move to Trash…` with the strip and the left pane hidden showed the run in the circle, which went at the end, with the canvas's edges unmoved, and the default layout's canvas was 1158 x 767 with the circle shown and gone; in a 563 px window a long error text was cut at its region's edge while the position and the circle kept their place, the error reading clearly in the muted color. On Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`) the canvas was recorded in the three other layouts, 1379 x 767 with the right pane hidden, 1600 x 767 with both side panes hidden and 1158 x 936 with the strip alone hidden, and did not resize when the circle appeared or went in any of them. What is left open is macOS and Linux, below. Plan: `docs/plans/_archived/20261115-native-status-bar/plan.md` (and `docs/plans/_archived/20261114-gpui-progress-bars/plan.md` for the removed bar).

Files: `crates/app/src/main.rs` (`status_row`, `progress_circle`), `crates/app/src/progress.rs`, `crates/app/src/theme.rs`.

#### TODO

- [ ] On macOS and Linux, repeat the menu, scan and Move to Trash checks above when at hand.

### App: real-device checks of the burst bands, badges, burst keys and reject the rest

#### Background

`gpui-bursts` ported the Tauri app's burst grouping to the GPUI app (`crates/app`): `burst.rs` groups the listing (`BURST_GAP_MS` 1000 ms), the strip paints a band behind each burst of two or more cells with a `position/size` badge, `burstPrevious` / `burstNext` / `burstFramePrevious` / `burstFrameNext` move through them, `rejectRest` rejects the rest of the current file's burst as one undo entry, and the MCP companion's `get_view` reports the current file's `burst`. CI and the unit tests cover the pure parts (`burst.rs` with every `burst.test.ts` case, the `reject_rest` change builder, `companion.rs`'s burst cases). The band's and the badge's look, the keys on a real keyboard, and `rejectRest`'s sidecars and undo on real files were never exercised, because the implementing session could not run the app's window. Plan: `docs/plans/_archived/20261114-gpui-bursts/plan.md`. Files: `crates/app/src/main.rs` (`paint_strip`, `move_burst`, `reject_rest`, `mcp_request`), `crates/app/src/burst.rs`, `crates/app/src/companion.rs`.

On Windows 11 on 2026-11-22 both checks passed on a Sony folder with `.xmp` sidecars (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`): the bands and the badges, dimmed on rejected members, the burst and burst-frame keys, `shift+x` under a Picked filter rejecting the hidden member with the current file kept and nothing advancing, its undo and redo, a singleton doing nothing, and `get_view`'s `burst`. Not run there: a Leica whole-second folder, the keys under a non-capture sort, in a JPEG folder and with the tree holding the keyboard, a `.dop`, and the two-frame burst's undo move. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): on a Sony folder with bursts at 0.1 s the band drew behind each burst with rounded ends, the `n/size` badges at the bottom-right, dimmed on a rejected member; one burst ended at `16/16` and the next started at `1/3`; the selected cells' outline was distinct from the band; a JPEG folder looked the same. MCP `navigate`'s burst and burst-frame moves were right in name order, under a stars filter with a rating sort, and in the JPEG folder. On a 16-frame burst with one member already rejected and one hidden by the filter, `reject_rest` rejected the 14 others (the `.xmp` and the `.dop` of each, stars and label kept), with the current file kept and nothing advancing. `undo` / `redo` reported `Undid 14 files` / `Redid 14 files`; a two-frame burst's undo moved to the other frame; a singleton and a JPEG folder did nothing; `get_view`'s `burst` listed the members with `visible: false` on the hidden one. Not run: no key was pressed (the tools and the keys share the view's `perform`); a Leica whole-second folder, a failed cell next to a band, and the keys while the tree holds the keyboard.

#### TODO

- [ ] On macOS, open a Leica whole-second burst folder in `handpic`: expect its bursts banded and badged as a Sony folder's. Expect a band to be distinguishable from a failed cell next to it, and `arrowup` / `arrowdown` and `alt+arrowleft` / `alt+arrowright` to do nothing while the folder tree has the keyboard. (The rest of this check passed on 2026-11-26 through MCP.)

### App: real-device check of the folder drop (gpui-folder-drop)

#### Background

`gpui-folder-drop` ported the Tauri window drop to the GPUI app (`crates/app`): a dropped folder opens through `open(path, None, None, cx)`, a dropped file opens its parent, and a drop of none or several paths shows `Drop a single folder or RAW file.` on the status line. While an OS file drag is over the window, a 3px `--ring` outline is drawn. CI and the unit tests of `dropped.rs` cover which single path a drop names and which folder it resolves to, and `mise run ci:local` passed. Nobody has dropped anything onto the running window: the drop, the outline drawing and clearing, and the swallowing of a drop by an occluded dialog are unverified. Step 1's checkbox was ticked on the automated criteria only. On Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`) a synthetic OLE drag started by an agent session did not reach the window (a `DoDragDrop` with the agent's own `IDropSource` failed with `E_FAIL`, and a WinForms `DoDragDrop` from an injected left press drew no outline and returned `None`; a `mouse_event` drag from Explorer reached other sessions' Handpic windows lying over this one at the target), so the boxes below need a hand drag from Explorer, or a run with the checked window topmost. Plan: `docs/plans/_archived/20261114-gpui-folder-drop/plan.md` (the pending check is also in `learnings.md` in the same folder). Files: `crates/app/src/main.rs` (`drop_paths`, `drop_outline`), `crates/app/src/dropped.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, with `mise run app:dev` running, drag a folder from Explorer over the window. Expect a 3px `#737373` outline on the main area (tree, preview, meta pane), also while the pointer is over the strip or the status row. Drop it: the folder opens and is revealed in the tree, and the outline goes.
- [ ] On Windows, drop one file (a RAW, then a non-RAW such as a `.txt`). Expect its parent folder to open and the file not to be selected.
- [ ] On Windows, drop two folders at once. Expect nothing to open and the status line to read `Drop a single folder or RAW file.`
- [ ] On Windows, open the settings modal (`ctrl+,`) and drop a folder over it. Expect nothing to open (the occluded layer swallows the drop).
- [ ] On Windows, drag a folder into the window and back out without dropping. Expect the outline to clear.
- [ ] Record the results in `learnings.md` and turn the "Not verified in the running app" bullet of "A folder dropped onto the window opens, a later drop winning" in `docs/agents/app.md` into a Measured one.

### App: real-device checks of the filter menu's icons and headings (gpui-filter-menu-icons)

`gpui-filter-menu-icons` gave the GPUI app's strip `Filter ▾` menu (`crates/app`) a shape per kind of row, drawn with Lucide icons from a new asset source, and a non-clickable heading over every section. The rating rows are now ordered 5 stars down to 0 and drawn as five star icons. The color labels are squares, the `Face focus` rows use `scan-face` and its frown derivative, and Portrait / Landscape use `rectangle-vertical` / `rectangle-horizontal`. New headings are `Flag`, `Rating`, `Color label` and `Orientation`. Unit tests cover the row model (order, headings, leads, checked items) and the asset source (every icon path loads SVG bytes, an unknown path loads `None`), and `mise run ci:local` passes. The window was never run, so how the icons and headings look and behave is unchecked: no agent can see the window, and Step 1's checkbox was ticked on the automated criteria only. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`) a heading showed over every section, the flag dots, the star rows 5 to 0 lit and dim, the orientations' rectangles and the `Face focus` icons with `Neutral` aligned were seen, each section's rows started at the same x, clicks toggled and the menu scrolled. The color labels are now gpui-component `Tag`s, not the squares described above (a later change). The headings' hover and click were not run, and no HiDPI display was at hand. Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): hovering the `Flag` heading drew no highlight, and a click on `Rating` changed nothing and left the menu open. Plan: `docs/plans/_archived/20261114-gpui-filter-menu-icons/plan.md`.

Files: `crates/app/src/main.rs` (`menu_row`), `crates/app/src/menu.rs`, `crates/app/src/icons.rs`.

#### TODO

- [ ] On a HiDPI or fractional-scale display, if available, check that the 1 px stroke icons read cleanly at 12 px. If a shape reads badly, pick a size per kind (`MENU_ICON` in `main.rs`). (Not run on 2026-11-27: the Windows host has one display at 100 %.)

### App: real-device checks of Import… (gpui-folder-dialogs Step 5)

#### Background

`gpui-folder-dialogs` Step 5 ported Import… to the GPUI app (`import.rs`: preview on a background task, `prepare` / `run_plan` on a thread with a cancel flag, the dialog stepping aside, the first date folder opened through `open_from_tree`). CI and the unit tests of `import.rs`, `context.rs` and `dialog.rs` cover the model. Nobody has seen the dialog, the progress, the open of the first date folder, the reopen or the cancel in the running app. The step's checkbox was ticked on the automated criteria only. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-5/learnings.md`; the run's material was under `D:\Photos\tests\20261122-folderops\`, not the `20261113-gpui-folder-dialogs` paths its setup box names) the dialog's count, its date folder rows and undated list and `Run` outlined, a run opening its first date folder in the tree and strip and ending in `Imported 9 files (273.1 MB)` with each `.xmp` beside its RAW, the reopened dialog's `Nothing left to copy` with `Run` dimmed and `Cancel` focused, the other-size list, the no-root refusal and one `import:` log line per run were seen. The source held no JPEGs of the two dates and no `.dop`; a 3.16 GB source copied from the file cache in a few seconds, too fast for the mid-run box; MCP `import_folder` is refused during a run (`a folder operation is under way in Handpic; try again once it ends`); the `Source:` line and the status line's `no RAW files in` text showed the Windows verbatim prefix (since stripped for display); the list clipped its last row (fixed by `folder-dialog-list-layout`). On Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-folder-dialogs/results.md`), on a source with JPEGs of the two dates and a `.dop`: the dialog's Tab from `Run` to `Cancel` and back and its Escape; the `reading …` note of a 500-JPEG preview; a 375-file, 6.31 GB run not in the file cache, with the dialog stepping aside, the first date folder opening and the pane counting `importing n / 375`; `Import…` picked mid-run bringing the dialog back with `Run` dimmed and `Cancel` focused, Escape hiding it with the run going on; and `Cancel` stopping a second run with `Import canceled, 78 files copied` and no `.handpic-tmp` file left were seen. The strip did not fill during the run: the open date folder read `This folder has no RAW or JPEG files.` until that folder's copies had all landed, because the open folder's watcher reports a burst `DEBOUNCE` (500 ms) after its last event and a copy that never pauses that long never settles; the fix, a maximum wait on the debounce, is delegated to the session `dlg-c4-import-watch`. What is left open is the box below. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/import.rs`, `crates/app/src/folderops.rs`, `crates/engine/src/import.rs`, `crates/engine/src/watch.rs`.

#### TODO

- [ ] On Windows, once the open folder's watcher reports during a burst that never pauses (a maximum wait on `DEBOUNCE` in `crates/engine/src/watch.rs`, delegated to the session `dlg-c4-import-watch`), `Run` an import on a source not in the file cache (or on a slow disk). Expect the strip of the opened date folder to fill while the pane counts `importing n / m`. (On 2026-11-27 the dialog stepped aside and the pane counted, but the strip stayed empty until that date folder's copies ended.)

### App: real-device checks of the tree's inline folder rename (gpui-folder-dialogs Step 7)

#### Background

`gpui-folder-dialogs` Step 7 ported the folder `Rename…` to the GPUI app: the inline editor over the tree row (`textinput.rs`), the held rename with its pending name during a scan, the slow second click and the cancel and refusal rules (`rename.rs`, `folderops::start_rename` / `gate_request` / `run_rename`). CI and the unit tests of `rename.rs`, `idle.rs` and `context.rs` cover the model and the gate. On Windows 11 on 2026-11-22 the refusals, Escape and an empty name, the field's keys and clipboard, Alt+Tab, the held rename's pending row, tooltip and note, its two cancels and the slow click were seen (results in `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`); the held rename failed once in two tries with `Access denied (os error 5)` (fixed since; 5 of 5 held renames succeeded on 2026-11-27 and its item was removed), two clicks about 480 ms apart opened the field (borderline with the 500 ms double-click time), and the IME and a rename during an import run were not run. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): the right-click `Rename…` put a field in the row with the name selected whole; Enter renamed the folder in place with the strip's files, judgments and position kept, no pass 1 work after it and the tree keeping the keyboard; a case-only rename worked; a single click on a root's name opened it with no field; a rename during an import run was refused with `Rename… is unavailable while an import runs`. The IME was not run. The step's checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261113-gpui-folder-dialogs/plan.md` (the full steps are in the `learnings.md` of the same folder). Files: `crates/app/src/rename.rs`, `crates/app/src/folderops.rs`, `crates/app/src/textinput.rs`, `crates/app/src/idle.rs`.

#### TODO

- [ ] On Windows, with the Japanese IME on, `Rename…`, type `しゃしん` and convert to `写真`. Expect an underlined composition, the candidate window at the field, the first Enter committing the conversion only and the second renaming the folder to `写真`.

### App: real-device checks of the app icon on Windows, macOS and Linux

#### Background

The `native-app-icon` feature gave the GPUI app its icon: `crates/app/build.rs` embeds `crates/app/icons/icon.ico` as icon resource ID 1 on a Windows target (through `winresource`), `crates/app/packaging/macos/Info.plist` and the `app:bundle:macos` mise task produce `Handpic.app`, and the window opens with `app_id: Some(IDENTIFIER)` with `crates/app/packaging/linux/dev.minodisk.handpic.desktop` and the `app:install:linux` task for Linux. CI and the tests cover the resource: the icon resource ID 1 was verified in the built Windows exe with `FindResourceW` / `LoadImageW`, the `datadir.rs` consistency test checks the `Info.plist` and `.desktop` file against `IDENTIFIER` and the binary's name on all three legs, and the macOS CI leg runs `plutil -lint` and checks that the executable and the `.icns` exist. Nobody has looked at the icon as the OS shows it: the taskbar, title bar and Explorer on Windows, the Dock and Finder on macOS, and the dock / taskbar / alt-tab of a running window on a Wayland and an X11 session. The Step 1 checkbox was ticked on the automated criteria only. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): after `mise run app:package:macos` (which runs `app:bundle:macos`) and a launch of the bundle, `NSRunningApplication.icon` (what the Dock draws) and `NSWorkspace`'s `iconForFile` (Finder) both gave the Handpic icon; the Dock was hidden and not captured. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): the window class's big and small icons, the taskbar button and Explorer's icon of `crates/app/target/debug/handpic.exe` all showed the Handpic icon, and the title bar draws it at its left end. Plan: `docs/plans/_archived/20261114-native-app-icon/plan.md`.

Files: `crates/app/build.rs`, `crates/app/src/main.rs`, `crates/app/packaging/macos/Info.plist`, `crates/app/packaging/linux/dev.minodisk.handpic.desktop`, `mise.toml`, `docs/agents/app.md`.

#### TODO

- [ ] On Linux, on a Wayland session and on an X11 session, run `mise run app:install:linux` with `~/.local/bin` on `PATH`, then launch Handpic from the application menu. Expect the running window to show the Handpic icon in the dock / taskbar / alt-tab. If it does not, check `StartupWMClass` (X11) and the `.desktop` file's base name (Wayland) against `IDENTIFIER`. Once it passes, remove the "Pending manual check" bullet under `## Packaging` in `docs/agents/app.md`.

### App: real-device checks of the strip's face focus icon and cell tooltips

#### Background

The `native-strip-face-icon-tooltips` feature gave the GPUI app's strip the Tauri strip's face focus icon (a smiling `scan-face` in bright green on a good frame, the frowning variant in gray on a low one, nothing on a neutral one, painted with `paint_svg` at the image box's bottom-left) and its hover texts (the scan's full error on a failed cell, the label's name over a labeled cell's name band). The tooltips are transparent region `div`s over the strip's canvas that build gpui-component's `Tooltip`. The tree's pending rename icon moved from the hand-drawn `Tip` to the same `Tooltip`. The unit tests of `MarkState::icon` and `strip::tip_regions`, clippy and the app tests cover the pure rules. Nobody has watched the running window: the icon's appearance during pass 2, its crispness at 100 % and 150 % scale, the dimming on a rejected cell, the tooltips' delay, wrapping and placement, and whether a region `div` leaves the strip's click, right-click and wheel input alone were never exercised. The Step 1 checkbox was ticked on the automated criteria only. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-4/learnings.md`) the frowning gray and the smiling green icons showed at the image box's bottom-left on low and good frames at 100 % scale; a truncated RAW copy showed `preview out of range` in a themed tooltip on hover, a Red-labeled cell's name band `Red`, and a plain cell nothing. The tree's pending rename tooltip `Renames when the scan finishes` was seen on hover in the batch before (`docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`). Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): on a fresh index the icons appeared on cells during pass 2 with no input, and none on neutral frames or JPEGs; no 150 % display was at hand. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): rejecting a good frame dimmed its green icon with the cell; on a failed and Blue-labeled cell the error showed as a tooltip over the cell and `Blue` over its name band, the tooltip swapped to the next cell's band (`Red`), a wheel scroll under the still pointer removed it, and a cell cut by the window's left edge still showed its own inside the window; over the tooltip regions a click, a Ctrl+click, a Shift+click, a right-click and the wheel still selected, extended, opened the menu and scrolled. The 500 ms delay was not measured (a capture takes about 0.3 s) and the only error at hand was too short to wrap. What is left open is in the boxes below. Plan: `docs/plans/_archived/20261114-native-strip-face-icon-tooltips/plan.md`.

Files: `crates/app/src/main.rs` (`strip_tips`, `paint_face_icon`), `crates/app/src/strip.rs` (`tip_regions`), `crates/app/src/mark.rs` (`MarkState::icon`), `crates/app/src/folderops.rs` (`tree_name`).

#### TODO

- [ ] On Windows, expect the strip's 12 px face focus icon to be crisp at 150 % display scale. (Not run on 2026-11-23: no HiDPI display; the icons' appearance during pass 2 was seen.)
- [ ] On Windows, hover a cell the scan failed on with an error long enough to wrap. Expect the tooltip after about 500 ms (time it with something faster than the `screenshot` tool, whose call takes about 0.3 s), wrapping at 360 px and staying inside the window. (On 2026-11-27 a short error's tooltip, a failed and labeled cell, the swap, the scroll, a cut cell and the clicks were seen.)
- [ ] On macOS and Linux, when at hand: reject a good frame and expect its icon dimmed with the cell; hover a failed cell and a labeled cell's name band and expect the error and the label's name as tooltips, swapping from cell to cell, going when the strip scrolls under a still pointer, and shown for a cell cut by the window's left edge; over a cell with a tooltip, click, ctrl / shift click, right-click and wheel still select, open the menu and scroll. (Seen on Windows on 2026-11-22 and 2026-11-27.)

### App: real-device check of the strip marks' one dark outline (strip-marks-outline)

#### Background

On Windows 11 on 2026-11-23 the strip's stars showed a dark outer outline and the face focus icons did not, and the user wanted the strip's marks drawn one way, like the stars. The `strip-marks-outline` change paints the face focus icon, the closed-eyes mark and the burst badge as the stars are painted: the mark over one black copy of itself 1 px down and right (`MARK_SHADOW`), in place of the eight copies at `overlay::HALO`'s offsets, which the `f` overlay's face labels keep. CI, the unit tests and a capture through the MCP `screenshot` tool showed the new drawing on Windows; the capture showed no burst badge, and colors and legibility are left to the user's eye. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261127-strip-marks-outline/plan.md`.

Files: `crates/app/src/main.rs` (`paint_marks`, `paint_face_icon`, `paint_burst_badge`, `MARK_SHADOW`).

#### TODO

- [ ] On Windows, on a scanned folder that includes a burst (frames within `BURST_GAP_MS`, so the badge shows) and a closed-eyes mark over a bright area, check on screen that the strip's stars, face focus icon, closed-eyes mark and burst badge each have the same dark outline (one dark copy down and right), and that each reads over both bright and dark parts of a photo.
- [ ] On macOS and Linux (any platform at hand), repeat the same check.

### App: real-device checks of the gpui-component adoption (theme, Input, controls, menus, Root, the three Dialogs)

`gpui-component-adoption` called `gpui_component::init` with a Neutral dark theme map, replaced `textinput.rs` with `Input`, moved the settings modal's controls to `Button` / `Checkbox` / `Radio` with scrollbars, gave the Sort / Filter and right-click menus the theme's look, mounted `Root`, and turned the trash / delete dialog, the folder operations' dialog and the settings modal into `Dialog`s. `mise run ci:local`, the pure models' unit tests, the keymap tests and the Step 5 A/B latency gate cover the build and the rules. The crate has no GPUI view test and `--auto` cannot click, so the IME, focus, Tab order, outside click, Escape behavior and look of every step were never exercised on screen. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): seen in passing, in a 480 px tall window `Filter ▾` and the right-click menus stop under the title bar, the `Filter ▾` thumb drags and a press on its track scrolls it and leaves it open, and the `Lightroom language` `Select` opens and chooses by keys with Tab / Shift+Tab keeping the focus in the settings; the boxes below were not otherwise run. Plan: `docs/plans/_archived/20261114-gpui-component-adoption/plan.md`. Files: `crates/app/src/main.rs`, `crates/app/src/theme.rs`, `crates/app/src/settingsview.rs`, `crates/app/src/settingsmodal.rs`, `crates/app/src/folderops.rs`, `crates/app/src/dialog.rs`, `crates/app/src/trash.rs`, `crates/app/src/modal.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

Run only on copies under `D:\Photos\tests\20261114-gpui-component-adoption\`, with a fresh `--index` and a test `--settings`. Build with `cargo build --manifest-path crates/app/Cargo.toml` (rename a locked `handpic.exe` aside, do not kill it). Launch with `crates/app/target/debug/handpic.exe --index <fresh temp>/index.sqlite --settings <test>/settings.sqlite D:\Photos\tests\20261114-gpui-component-adoption\<copy>`.

#### TODO

- [ ] On Windows (Step 1: init and theme), page with the arrows, press `tab` (the side panes toggle), `f`, `z`, `e`, `d`, open and close the Sort / Filter menus, open the settings with `ctrl+,` and type a label name with the Japanese IME, open a right-click menu on the strip and on the tree, open and cancel the trash dialog, and press `ctrl+shift+i` (the component inspector opens, debug build). Expect everything to look and behave as on `main`, with only `ctrl+shift+i` new. Then run once with `--auto 20` and compare the timing lines with a `main` build's: they should be the same in kind (not the A/B gate).
- [ ] On Windows (Step 2: `Input`), in the settings' Sidecar tab check that the name fields show the stored names with a 1 px white-15 % border, no fill, and the same height and text size as `main` within a pixel or two, and that the focused one has the `#e5e5e5` border and no extra ring. Then:
  - Type a label name with the Japanese IME: the composition is underlined, the candidate window sits at the caret, and Enter inside the composition commits the composition, not the field.
  - Commit with Enter, with a click elsewhere in the box and with Tab (Tab and Shift+Tab move between the modal's stops from inside a field), then reopen the modal and see the names stored.
  - Escape in a field closes the modal with the edited name kept. `Reset` and a language pick show their names in the fields.
  - In the Import tab type a pattern with and without the IME: the example line follows each key, an invalid pattern shows its refusal in red on commit, and an example pattern button fills the field and its example.
  - `ctrl+a` then typing replaces the text; `ctrl+c` / `ctrl+x` / `ctrl+v` work; `ctrl+z` undoes (new).
  - Right-click in a field: no panic, and the OS-native Cut / Copy / Paste / Select All menu shows and works.
  - A key typed in a field (`f`, `z`, `d`, the arrows) acts in the field only.
  - After closing the modal the arrows page again.
- [ ] On Windows (Step 2 catch-up: the tree's inline rename editor, ported to `Input` without being seen on screen), start a rename from the tree (a slow second click on the open folder's name, or the right-click `Rename…`). Expect an editor with the whole name selected, a `#e5e5e5` border, no fill, 13 px text, fitting the row's height. Then:
  - Type a new name with the Japanese IME: Enter inside the composition commits the composition, not the rename.
  - Commit with Enter: the folder is renamed and the tree has the keyboard again. Start another and commit with a click elsewhere: renamed as well.
  - Start another and press Escape: nothing is renamed, the name comes back, the tree has the keyboard.
  - Switch to another app with the editor open: no rename until the window comes back.
- [ ] On Windows (Step 4: menus), the Sort and Filter menus are the `#171717` surface with a white-10 % border and 6 px radius, 26 px rows with 6 px rounded `#262626` hover, the check icon at the left of a checked item, muted headings, separators across the padding, and a disabled item muted with no hover. Use a folder with bursts and EXIF variety so the Filter menu is long. Then:
  - Sort and Filter open above their button with the right edges aligned.
  - An outside press closes an open Sort / Filter menu (in the strip, in the preview, and on the menu's own button, which does not reopen it). An item click applies and leaves the menu open.
  - With the Filter menu open: `escape` closes it, the arrows page, `e` / `z` / `f` toggle their overlays, the burst keys act, and the menu stays open through all but `escape`.
  - Make the window short so the Filter menu exceeds its cap: it scrolls by wheel, shows a scrollbar, dragging the thumb scrolls it, a press on the scrollbar does not close it, and reopening starts at the top.
  - With a Sort / Filter menu open nothing takes the GPUI focus.
- [ ] On Windows (Step 5: `Root`), everything looks as on `main` (menus' rows, settings buttons and fields, empty state's text the same size). Then:
  - `tab` toggles the side panes. Bind `shift+tab` to an action in the Keyboard Shortcuts tab (e.g. `toggleStrip`): `shift+tab` acts.
  - In the settings modal Tab / Shift+Tab walk its stops (from a field too), and the Shortcuts tab's `+` then `tab` adds `tab` as a chip.
  - IME in a label name composes and commits as before.
  - `ctrl+c` in a settings field with text selected copies it; `ctrl+c` outside does what the keymap binds it to, if anything.
  - The menus close on an outside press. The trash dialog's Tab / Shift+Tab move between its buttons, Enter runs the focused one, Escape cancels.
  - Click in the tree, use the arrows there, press `escape` or click the preview: the keyboard comes back to the photos.
  - Quit with `q` and relaunch without a folder: the last folder and the window's place and size come back.
- [ ] On macOS (Step 5: `Root::new`'s window hit-test forwarder), click on every pane (tree, preview, strip, meta pane) and on the status row's buttons, and drag the title bar. Expect each to land where it should. Also repeat the Windows Step 5 list when at hand.
- [ ] On Linux (Step 5: client-side decorations), launch on GNOME Wayland and on an X11 session. Expect the window's edges to look acceptable (record what the frame, shadow and resize bands look like), resizing from the edge to work, and a right-click in a settings field to open Cut / Copy / Paste / Select All (a drawn `PopupMenu` on Linux).
- [ ] On Windows (Step 6: trash / delete dialog), use a fresh copy for each destructive run.
  - Right-click a strip file, `Move to Trash…`: a 640 px dark (`#171717`) box with the title, the line, a red run button and Cancel, the window dimmed, no close button, no animation, and the focus ring on the run button.
  - Enter moves the file to the Trash; again with Space; again with a click on the run button. The dialog closes when the run ends and the pane says `Moved 1 file to the Trash`.
  - Tab, Shift+Tab, Tab: the ring goes Cancel, run, Cancel. Enter on Cancel and Escape close without moving anything.
  - A press on the dimmed backdrop (also a double-click, and near the top edge) or on the box's text does nothing, the window does not move or maximize, and Enter still presses the focused button.
  - `ctrl+,` and a paging key while open do nothing, and the wheel over the strip behind does not scroll.
  - `Delete Permanently…` opens on Cancel. Enter, Space, `shift+enter`, `ctrl+space` do nothing (on Cancel, and after Tab on the run button). Hold Enter from before the menu item's click: nothing happens. Double-click the menu item so the second click lands on the run button: nothing is deleted. A single click on the run button deletes.
  - Escape during a long run does nothing, and both buttons look disabled until it closes.
  - Press `Move to Trash…` during pass 1: the dialog closes, the scan stops, and the run happens at the scan's end without the dialog.
  - `ctrl+z` after a move brings the file back.
  - With the tree holding the keyboard before the dialog opened, Escape and a run's end both leave it holding it. After every close the arrows page and `tab` toggles the side panes.
- [ ] On Windows (Step 7: folder operations' dialog), with the settings' Import root set to a test folder under the same test directory, open each of `Rewrite Sidecars from Index…`, `Delete Sidecars…`, `Sequence JPEG Timestamps…`, `Import…` and `Rename by Capture Date…` from the tree's right-click menu. Expect a 640 px dark box with the title, text, row list, notes, a run button (light for primary, red for Delete Sidecars) and Cancel, the window dimmed, no close button, no animation, and the ring on the run button. Then:
  - Tab, Shift+Tab, Tab: ring goes Cancel, run, Cancel. Enter on Cancel closes; Space on the run button runs it; Escape closes; any other key (`f`, an arrow, `ctrl+,`) does nothing.
  - A press on the backdrop or the box's text (also a double-click near the top edge) does nothing, the window does not move, and Enter still presses the focused button.
  - A disabled run (e.g. Rename by Capture Date on a colliding target): ring on Cancel, Tab keeps it there, Enter closes.
  - A long list: rows scroll by wheel inside the box with a scrollbar, thumb drag scrolls, and notes and buttons stay in view.
  - Sequence on a big folder: the run button turns disabled, the ring moves to Cancel, the running note shows, the pane's progress moves, and Enter or Escape cancels.
  - Import: the dialog steps aside once the folders exist and the window's keys work; `ctrl+,` does nothing; `Import…` brings the dialog back with the ring on Cancel; Escape steps it aside again; Cancel cancels the import.
  - Rewrite Sidecars / Rename by Capture Date during a run: Escape does nothing, both buttons look disabled, and the dialog closes at the run's end.
  - An operation pressed during pass 1 opens its dialog when the scan ends.
  - After each close the tree still has the keyboard, and an inline rename then works (Enter commits, Escape cancels).

### App: real-device checks of the strip, folder tree and meta pane scrollbars

#### Background

The `native-scrollbars` feature gave the GPUI app's thumbnail strip, folder tree and meta pane gpui-component's scrollbar in the theme's `ScrollbarMode::Scrolling` (shown while the area scrolls, fading out after about 2 s idle, taking a press only while shown). The tree's bar sits on a `relative()` wrapper of its `uniform_list`, the meta pane's body ends in `overflow_y_scrollbar()`, and the strip's horizontal bar goes through `StripBar`, a `ScrollbarHandle` over the strip's own model whose pending offset `Handpic::render` applies through `Strip::scroll_to` and `strip_moved`. `stripbar.rs`'s unit tests and `mise run ci:local` cover the pure logic. Nobody has watched the running window: the bars' look, the fade-out, dragging and track clicks, the thumbnails following a drag, and what a press on a shown bar does and does not reach were never exercised. The Step 1 checkbox was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261115-native-scrollbars/plan.md`. Since `native-scrollbars-always` the mode is `ScrollbarMode::Always`, so the bars show at rest and the fade-out and the press taken only while shown, above, no longer apply (`### App: real-device checks of the always-visible scrollbars` holds that feature's checks). Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`): on a 295-file folder the strip's thumb dragged to the far end showed the last cells' thumbnails within 3 s, and the thumb followed `→`, `←`, `Home`, `End`, a refilter and its reset, and a 500 px narrower window, the current cell in view each time; a press on the thumb and two track clicks left the current file and the one selected file, and the cell's `Red` tooltip showed before and after; a click, a right press and an inline rename (Enter and `escape`) next to the tree's bar behaved as before; the wheel scrolled the settings modal's `Lightroom language` list and Keyboard Shortcuts page, `Filter ▾` and the strip's right-click menu in a short window, their thumbs moving. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): in a 480 px tall window, dragging the thumbs of `Filter ▾`, the strip's right-click menu, the Keyboard Shortcuts page, the Sidecar page and the `Lightroom language` list scrolled them, a press on the `Filter ▾` track scrolled it and left it open, the tree's right-click menu overflowed the window and scrolled by wheel, and the tree's own thumb drag scrolled the tree. What is left open is in the boxes below.

Files: `crates/app/src/main.rs` (`tree_pane`, `meta_pane`, `render`), `crates/app/src/stripbar.rs` (`StripBar`), `crates/app/src/strip.rs` (`Strip::content_width`).

#### TODO

- [ ] On macOS and Linux, when at hand, repeat the Windows checks the Background records (the strip's, the tree's and the meta pane's bars), and drag the thumbs of the settings' `Lightroom language` list and Keyboard Shortcuts page, of `Filter ▾` and of the strip's right-click menu in a short window, and make the tree's right-click menu overflow in a short window and scroll it. Expect the bars to scroll their areas.

### App: real-device check of the meta pane's `Analysis` group and the AF rows in `Maker note`

`native-meta-analysis` made the GPUI app's (`crates/app`) right-hand meta pane show an `Analysis` group from the values the scan's pass 1 and pass 2 stored in the index (`Eye focus`, `Eyes`, `Eyes closed`, `Eye aspect ratio`, `Yaw`, `Pitch`, `Roll`, `Mesh eye offset`, `Edge gap`, `Sharpness`), and `AF point` / `AF frame` in the `Maker note` group after `AF tracking`. Unit tests in `meta.rs` and `mise run ci:local` cover the row builders, `groups` and the string formats (ported from `meta.test.ts`). Nobody has looked at the running window: what the pane shows for a scanned file, that the rows appear by themselves when the scan reaches the shown file, the failed-preview and hidden-by-refilter cases, and the first preview after a folder open (that path now sets `self.shown`'s index to `None`, so the pane shows the old file name with no index rows until the new folder's first preview arrives) were never exercised. Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): on a fresh index of 125 ARWs the shown file's `Analysis` group appeared at `analyzing 27/125` with no input, holding `Eye focus` (`Sharp · 0.281`), `Eyes` (`Open · 76`), `Eyes closed`, `Eye aspect ratio`, `Yaw` / `Pitch` / `Roll`, `Mesh eye offset`, `Edge gap` and `Sharpness`, with `AF point` (`5913, 2964 of 8640 × 5760`) and `AF frame` directly after `AF tracking` in `Maker note` and neither in `Analysis`; in a folder under 10 files the group appears only when pass 2 ends, since pass 2 commits rows in batches of 10 (`BATCH` in `crates/engine/src/index.rs`), which is expected; a JPEG showed no `Analysis` group, a filter hiding every file left only `No files match the current filter.`, and the rows were the same with `f` on and off. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): a scanned ARW whose JPEG markers were then zeroed (size and mtime kept) showed the decode error in the preview area, a `Maker note` group holding only `AF point` / `AF frame`, and its full `Analysis` group. Plan: `docs/plans/_archived/20261114-native-meta-analysis/plan.md`. Files: `crates/app/src/meta.rs`, `crates/app/src/main.rs` (`meta_pane`, the open path's `shown`).

#### TODO

- [ ] On Windows, open a second folder while a file of the first is on screen. Expected: until the new folder's first preview arrives, the pane shows no rows from the index (not another file's), then the new file's rows. (Inconclusive on 2026-11-27: the first capture landed 0.37 s after `open_folder`, when the new folder's preview was already shown, and both folders held files of the same names. Use folders with different file names and something faster than a screenshot.)

### App: real-device checks of the settings as a full-window layer (native-settings-component Step 1)

#### Background

The `native-settings-component` feature's Step 1 replaced the settings `Dialog` with a full-window, opaque, occluding layer drawn by `Handpic::render`, under a focus trap. CI and `mise run ci:local` cover the build and the pure `settingsmodal.rs` tests. The layer's opacity, typing, the Tab trap (30 Tabs and 2 Shift+Tabs stayed in the settings with the panes untouched), the `tab` capture, the blocked keys and strip click, and the close by Escape and `×` were seen on Windows 11 on 2026-11-21; hovers, wheel scrolls and the drop were not tried. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): with the settings open, a hover where a strip cell's tooltip would show showed none, and three wheel notches there left the strip unscrolled; the drop was not run. Plan: `docs/plans/_archived/20261115-native-settings-component/plan.md`.

Files: `crates/app/src/settingsview.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On Windows, run the app with a test `--settings` store and an open folder, open the settings and drop a folder on the window. Expected: it opens nothing (record what actually happens; if it differs, correct the drop rule in `docs/agents/app.md`).

### App: real-device checks of the settings body drawn with gpui-component's `Settings` (native-settings-component Step 2)

#### Background

The `native-settings-component` feature's Step 2 drew the settings body with gpui-component's `Settings` (sidebar with a search field, four `SettingPage`s), mapped the sidebar colors in `theme.rs`, and replaced the old tab-switch capture cancel with a capture-phase pointer-press cancel. The new `editing` rule (an `Input` key context on the focused path) lets the private search field take keys. CI and the remaining `settingsmodal.rs` tests cover the pure model. Typing in the search field, Escape on a non-empty query, the capture cancel, the Tab trap and the search filtering were run on Windows 11 on 2026-11-21 (the sidebar now also lists Cache, MCP, Appearance and Updates); the refusal of `home` / `end` in a capture and the user's visual check of the look are left. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): on Keyboard Shortcuts, `+` then `home` showed `"home" is taken by this app` and `+` then `end` the same for `end`, in red in the settings body, not on the window's status line (the full-window settings layer covers it), the keymap unchanged. Plan: `docs/plans/_archived/20261115-native-settings-component/plan.md`.

Files: `crates/app/src/settingsview.rs`, `crates/app/src/theme.rs`.

#### TODO

- [ ] On Windows, the user's visual check of the settings' look (group headers, label-left / control-right rows, the sidebar's colors).

### App: real-device checks of the drag-resizable tree and meta panes

`native-pane-resize` made the GPUI app's (`crates/app`) folder tree and meta pane drag-resizable by their boundary with the preview, stored under the `panelWidths` key of the settings store (default 220 px, a pane 160 to 480 px, the preview at least 240 px). Unit tests (`panewidths.rs`, `settings.rs`'s read of `panelWidths`) and `mise run ci:local` cover the arithmetic and the setting. On Windows 11 on 2026-11-21 a drag clamped at 480 and 160 px, a narrow window kept the preview at 240 px or more and widening restored the widths, a `Tab` hide and show came back at the width, a restart restored 160 / 480, and `--auto` started at 220 with a drag writing nothing. Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`): the cursor turns col-resize on the preview's outer 5 px beside each pane and the 3 x 20 px pill shows on hover only, the 1 px line reading as the preview's edge (judged in Neutral dark only, 3 levels from the tree pane's fill); every release, over the preview, a pane, the strip or outside the window, stored the clamped width at once, kept it over a quit and came back at it; the 1:1 crop was re-cut about 3 s after a drag that grew the canvas; the tree's rows, its inline rename input and its thumb work at 160 px; the boundary ignored the first 3 to 5 px of a drag and then sat under the pointer, since fixed by `pane-drag-offset-meta-wrap` to keep the press's offset (re-checked on Windows 11 on 2026-11-22 after the fix: the tree's and the meta pane's boundaries and the strip's top edge kept the press's offset in both directions, the first 1 to 4 px of a drag still moving nothing); the meta pane broke words mid-word at 160 px, since fixed to put a value that does not fit under its label and to break the name after its separators (re-checked on Windows the same day: no word broke at 160 px, and at 220 px only `AF point` and `Metering` went under their labels); the preview has no context menu. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-empty-text-wrap/results.md`): in a 1200 px window with both panes at 480 the no-folder message, since `empty-text-wrap` wrapped within the preview, sat inside it and a click on it opened the picker, while clicks on the bands and the tree opened nothing. What is left is in the boxes below. The limits (160 / 480 / 240) were not judged by eye. Plan: `docs/plans/_archived/20261115-native-pane-resize/plan.md`. Files: `crates/app/src/main.rs` (`pane_handle`, `drag_pane`, `end_pane_drag`, the root `div` in `render`), `crates/app/src/panewidths.rs`.

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store (except where a check says to restart).

#### TODO

- [ ] On macOS and Linux, repeat the hover, drag, narrow-window, restart and hide / show checks above, and in a 1200 px window with both panes at 480 (the preview at 240 px) check that the no-folder message wraps onto centered lines inside the preview, a click on the text opens the picker and a click on a band opens nothing (`empty-text-wrap`, seen on Windows 11 on 2026-11-27, results in `docs/plans/_archived/20261127-empty-text-wrap/results.md`). CI only builds these platforms. Not runnable here: another OS than the host's (Windows).

### App: real-device check of the folder tree's file counts on collapsed rows (native-tree-index-counts, native-folder-count-cache, native-folder-count-prune)

#### Background

The native-tree-index-counts work made every drawn row of the GPUI app's folder tree show its file count, expanded or collapsed, when it is above zero. The native-folder-count-cache work moved the count of a row not listed in this session to the index's `folder_counts` table: one row per folder, keyed by the tree node's own path, written off the UI thread whenever the tree learns a count from disk (an expand, `Refresh`, a tree watcher change, `Expand All`, a reveal's chain, an open's listing), carried by a folder rename and written for a date split's new folders, kept by the eviction, and read whole once at startup and after a rename or a date split. A listed folder shows its `read_dir` count. The table starts empty at the upgrade, so no collapsed row shows a count until its folder has been listed once. `mise run ci:local` and the `tree.rs` / `index.rs` / `rename.rs` / `datefolder.rs` unit tests cover the merge, the case folding, the write coalescing and the table's writes, carry and read. Nobody has seen the pills drawn, the cached count after a restart, or the re-read after a rename or a split in the running app. A write still queued at quit is lost (not drained), which the folder's next listing repairs. The native-folder-count-prune work made every successful tree listing also delete the `folder_counts` rows of the child folders the listing no longer holds, and the rows under them, keeping the child leading to any tree root under the listed folder; `index.rs` and `tree.rs` unit tests cover the prune and its order within `CountWrites`, but nobody has seen it in the running app. Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-3/learnings.md`): expanded and listed folders show the `read_dir` count; collapsed rows listed earlier in the index show their cached count after a quit and restart; a folder renamed in the app keeps its count after a restart; a counted folder renamed outside the app, and a new folder of an old name, show no count after a restart until listed; a folder never listed shows none; a long name next to a count pill truncates with the pill whole. The prune was seen only as far as that outside rename shows. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): expanded then collapsed never-opened folders, one through a symlink, showed their counts and kept them after a restart; the opened folder too; a never-listed folder showed none. An expanded row's pill looked the same as a collapsed one's. A RAW added outside the app followed a `Refresh`, and the new count showed after a restart. A folder renamed in the app kept its count after a restart. A folder deleted outside and re-created under the old name showed no count until listed. A long name truncated with the pill whole. A `Rename by Capture Date…` split showed each new date folder's count while collapsed, the emptied original removed. Not run: an index upgraded from before the cache table, a network volume, a folder renamed outside the app, and a split whose undated files keep the original. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): an index whose `folder_counts` table was dropped showed no count on collapsed rows; a RAW added outside the app followed a `Refresh` and the new count showed after a restart; a `Rename by Capture Date…` split showed each date folder's count while collapsed, the emptied original removed; a junction and a folder on a mapped network drive kept their counts after a restart; a counted folder deleted outside lost its row on its parent's `Refresh`, and a new folder of that name showed no count after a restart. Not run: a split whose undated files keep the original folder. Plans: `docs/plans/_archived/20261115-native-tree-index-counts/plan.md`, `docs/plans/_archived/20261116-native-folder-count-cache/plan.md`, `docs/plans/_archived/20261117-native-folder-count-prune/plan.md`.

Files: `crates/app/src/tree.rs` (`set_cached_counts`, `set_listed_count`, `CountWrites`, `prune_keep`), `crates/app/src/main.rs` (the tree row's count pill, `tree_listed`, `cache_count`, `prune_counts`, the read at startup, `opened`), `crates/app/src/folderops.rs`, `crates/engine/src/index.rs` (`folder_counts`, `set_folder_counts`, `rename_folder_counts`, `prune_folder_counts`), `crates/engine/src/rename.rs`, `crates/engine/src/datefolder.rs`, `docs/agents/app.md` (the folder tree section).

#### TODO

- [ ] On Windows (`mise run app:dev`), run `Rename by Capture Date…` with a split that leaves undated files in the original folder: it shows no count until listed. (The rest of this check passed on 2026-11-27.)
- [ ] On macOS (`mise run app:dev`), launch the app on an index upgraded from before the cache table: expect no collapsed row to show a count yet. Expand a folder reached through a network volume, restart: expect its count. Rename a counted folder outside the app, expand or `Refresh` its parent, quit and restart: expect no count on the new name until it is listed. Run `Rename by Capture Date…` with a split that leaves undated files in the original folder: it shows no count until listed. (The rest of this check passed on 2026-11-26.)
- [ ] On Linux (`mise run app:dev`), launch the app on an index upgraded from before the cache table: expect no collapsed row to show a count yet. Expand a folder that was never opened and holds RAW files, collapse it, then quit and restart: expect its collapsed row to show the count before you expand it. Open a folder and let it list, restart: the same. A folder never listed shows no count. Expand a folder: its number is the `read_dir` count and looks the same expanded or collapsed. Right-click a folder > `Refresh` and add or remove a RAW outside the app: the count follows, and after a restart the new count shows. Rename a listed folder, restart: the renamed row keeps its count. Run `Rename by Capture Date…` with a split: each new date folder shows its count while collapsed, and the original folder (if undated files keep it) shows none until listed. Expand a folder reached through a symlink or a mounted network folder, restart: expect its count. Check a long name next to a count pill still truncates with the pill whole. Delete or rename a counted folder outside the app, expand or `Refresh` its parent, quit and restart, then create a folder of the old name: expect no count on it until it is listed.

### App: real-device checks of the title bar (native-title-bar)

#### Background

The `native-title-bar` feature replaced the OS title bar of the GPUI app with gpui-component's `TitleBar` (34 px, the app icon from `icons/app.png` served through `Icons`, and the text "Handpic"). The window options are `..TitleBar::window_options()` with an explicit `titlebar` title "Handpic", and Linux requests `WindowDecorations::Client`. CI and the tests cover the build on all three platforms and a test that `Icons` loads the icon's PNG bytes. The author saw one debug-build launch on Windows 11 with no folder open. In it the bar showed the icon, "Handpic" and the three controls, the process's main window title read "Handpic", and the tree, the empty-preview message and the status row laid out below the bar. On Windows 11 on 2026-11-21 minimize, maximize, restore, close and the bar's drag worked, the main window title read "Handpic", a double-click on the bar maximized and restored, and the bottom-right corner resized; a folder's preview and overlays under the bar, the menus' height cap, the other resize borders, Snap Layouts, the bar under a dialog, saved window geometry, macOS and Linux are not exercised. The Step 1 checkbox was ticked on the automated criteria and that launch screenshot only. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): with a folder open the preview, `f`, `d`, `z`, the tree, the meta pane, the strip and the status row lay out below the bar, and in a 480 px tall window `Filter ▾` and the right-click menus stop under it; drags from all four edges (the top one is the client's top 1 px, over the bar) and all four corners resized the window, the maximize button's hover showed Snap Layouts, and the maximized bar spans the display; a stored `windowBounds` placed the window at the same rectangle on three launches; five themes each gave the bar and its border their colors at once. Failed: under the trash dialog or a folder operation's dialog, minimize, maximize and the bar's drag are dead. Not run: the drop outline. Plan: `docs/plans/_archived/20261116-native-title-bar/plan.md`.

Files: `crates/app/src/main.rs`, `crates/app/src/icons.rs`, `crates/app/src/modal.rs`, `crates/app/src/session.rs`, `crates/app/src/theme.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, open a folder and drag a folder over the window. Expect the drop outline to lay out below the bar with nothing clipped. (The rest of this layout check passed on 2026-11-27.)
- [ ] On Windows, with the Cache page's `Clear the index cache?` confirmation open over the settings, click minimize / maximize and drag the bar, and double-click the bar under any dialog. Expect them to work, the bar undimmed, and Enter / Escape still acting on the dialog afterwards. The settings modal alone was measured working on 2026-11-27, and under the trash and `Delete Sidecars…` dialogs minimize, maximize, the drag and close were measured working on 2026-11-28 (`docs/plans/_archived/20261128-title-bar-over-dialogs/results.md`). Not runnable here: the cache confirmation over the settings and the double-click were not part of that run.
- [ ] On macOS and on Linux (client-side decorations), with the trash dialog and a folder operation's dialog open, click the window controls and drag the bar. Expect them to work, the bar not dimmed, and a press on the bar or on the dimmed area below it to leave the focus on the dialog's focused button (Enter / Escape still act on the dialog). The bar is a `deferred` draw above the dialog layer since `title-bar-over-dialogs`; Windows was measured working on 2026-11-28 (`docs/plans/_archived/20261128-title-bar-over-dialogs/results.md`). Not runnable here: needs macOS and Linux.
- [ ] On macOS, expect the traffic lights to sit inside the bar without overlapping the icon and title (80 px left padding), the content to start below the bar, and double-click on the bar to follow the system setting.
- [ ] On Linux, under client-side decorations expect the bar to draw its controls and `Root`'s frame and shadow. Under server-side decorations (X11 without a compositor, or a Wayland compositor that insists) expect no second set of controls. WSLg can serve as a first pass once a WSL distribution is installed, but it does not replace a check on a regular GNOME / KDE desktop.

### App: real-device checks of the title bar's settings button (native-title-bar-settings)

#### Background

The `native-title-bar-settings` feature added a gpui-component `Button` with the Lucide `settings` icon as the second child of the app's `TitleBar`, at the right end of the bar. A click opens the settings modal through `Handpic::open_settings`, the same method `ctrl+,` (`meta+,` on macOS) uses, with the same refusals. The press does not move the window: a wrapping `div` stops the left mouse-down from propagating. The tooltip is "Settings" with the `OPEN_KEY` as a `Kbd`. CI and the unit tests cover the tooltip's text and key per platform and that `icons/settings.svg` loads through `Icons`. By hand on Windows 11 (debug build, driven by a script) the author saw the following:
- A press and drag on the gear left the window where it was, and a drag on the empty bar moved it.
- A click opened the settings.
- `Escape` closed the settings and `ctrl+,` reopened them right away.
- The tooltip showed on hover.
- The gear and its hover fill were visible in Neutral Light and Dark.

Seen on Windows 11 on 2026-11-21: the window controls and the bar's drag after the change, a judgment key and paging right after closing the settings the gear opened, and the gear pressed with the tree holding the keyboard, with `Sort ▾` open and with a right-click menu open; never exercised: a shipped non-Neutral theme, the gear during an inline rename, macOS, Linux. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c5/results.md`): the gear, its hover fill and its tooltip `Settings Ctrl+,` showed in Neutral dark, Gruvbox Dark and Solarized Light, and a click on the gear during an inline rename opened the settings and closed the field with the name unchanged. The Step 1 checkbox was ticked on the automated criteria and the checks above. Plan: `docs/plans/_archived/20261117-native-title-bar-settings/plan.md`. The `title-bar-tweak` feature then retuned the bar: 13 px text, an 18 px app icon and a gear with a thinner stroke (`icons::SETTINGS_THIN`, 14 px) in a `TITLE_BAR_HEIGHT`-wide, full-height, square-cornered ghost `Button` with no `mr_2` margin, after the window controls' own box. That look was seen and approved only on Windows 11 in a dev build; macOS and Linux are unseen. Plan: `docs/plans/_archived/20261127-title-bar-tweak/plan.md`.

Files: `crates/app/src/main.rs` (`settings_button`, `open_settings`), `crates/app/src/settingsmodal.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On macOS (`mise run app:dev`), open a folder and look at the title bar. Expect the gear flush at the bar's right edge (no `mr_2` margin), its hover fill filling the bar's height with square corners, the 14 px icon not clipped, and the 18 px app icon and 13 px text looking right beside the traffic lights. A press and drag on the gear should not move the window (the bar's `start_window_move`), while the rest of the bar still drags. A click should open the settings and the tooltip should show `Cmd+,`.
- [ ] On Linux, under client-side decorations, open a folder and look at the title bar. Expect the gear flush beside the window controls, its hover fill filling the bar's height with square corners, the icon not clipped, and the 18 px app icon and 13 px text looking right. A press and drag on the gear should not move the window, the rest of the bar should still drag, and a click should open the settings.


### App: real-device check of the OS notification when an import ends while the window is inactive (native-import-notification)

#### Background

The native-import-notification work makes the GPUI app (`crates/app`) post one system-only OS notification when an `Import…` run ends while the main window is not active. The title is `Import finished`, or `Import canceled` after a cancel, and the body is the status line's summary (`import::done_status`). The app also now sets its process identity at startup (`cx.set_app_identity`), which on Windows sets the explicit AppUserModelID `dev.minodisk.handpic`. `mise run ci:local` and the unit tests of `import::done_notification` (the decision and the text) cover the pure part. The toast's actual delivery was never observed, nor was the click activation or the taskbar icon and grouping after the AppUserModelID was set. The step's checkbox was ticked on the automated criteria only. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-5/learnings.md`) the toast was delivered at the desktop's bottom-right with the header `Handpic`, the title `Import finished` and the status line's summary as its body (`Imported 60 files (3.16 GB)`); its app icon is a generic one, not Handpic's, as `docs/agents/app.md` expects (no icon is registered). Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): a cancel with Notepad in front posted `Import canceled` / `Import canceled, 13 files copied`; a finished run posted `Import finished` / `Imported 18 files (676.5 MB)` about 0.9 s after `Run`, and a click on the toast brought the Handpic window to the front; a run with the window kept in front posted nothing for 10 s; no log had a `failed to register AppUserModelID` or `cannot show a system notification` line. `docs/agents/app.md`'s section is now (Measured). The taskbar button's icon and grouping were not seen. Plan: `docs/plans/_archived/20261116-native-import-notification/plan.md`.

Files: `crates/app/src/folderops.rs` (`import_ran`, the deferred post), `crates/app/src/main.rs` (`set_app_identity`), `crates/app/src/import.rs` (`done_notification`), `docs/agents/app.md` (the section "An import's end posts a system-only notification while inactive").

#### TODO

- [ ] On Windows, with the taskbar shown (it is auto-hidden on the 2026-11-27 host, so no instance's button was seen), run `mise run app:dev` and check that the taskbar button's icon and grouping of the running app look unchanged (an existing pin of the exe may no longer group with the running window). If the grouping misbehaves, file a fix issue.

### App: real-device checks of the gpui-kit umbrella crate (native-gpui-kit)

#### Background

`native-gpui-kit` Step 1 made `crates/app` depend on `gpui-kit` `=0.9.1` alone (no direct `gpui-pre`, `gpui-pre-platform`, `gpui-component` or `gpui-kit-assets`), import GPUI through `gpui_kit::*`, start through `gpui_kit::init` / `gpui_kit::open_window`, and optimize the framework crates in the dev profile. The umbrella crate also turns `font-kit`, `x11`, `wayland` and `runtime_shaders` on for `gpui-pre-platform` (read from manifests and sources, not run): before, macOS had no `font-kit` and Linux had neither backend. `mise run ci:local` (879 native tests, clippy) and the `native` CI job cover the build. On Windows a debug launch ran 20 s without a panic. No one has looked at the window on any platform, and macOS and Linux were never run. The Step 1 checkbox was ticked on the automated criteria only. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): the debug window drew its text (the title bar, the tree, the no-folder message, the status row, the meta pane, the `Sort ▾` / `Filter ▾` buttons) with no shader error on stderr or in the log; the popover menus were not opened. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): with a copy of the real settings store the window restored its saved place (`window: restored on display 0` at the stored bounds, a 1409 x 856 client) and the copy held the same bounds after `q`; the Lucide icons (tree, strip and menu stars, orientation, face focus) and the settings' `Radio`s, `Input`s, `Select`s, `Switch` and `NumberInput` drew in Neutral dark and Gruvbox Dark (the app no longer has a `Checkbox`); `tab` toggled both side panes in the main view and walked the focus between `Move to Trash` and `Cancel` in its dialog. Plan: `docs/plans/_archived/20261116-native-gpui-kit/plan.md`. Files: `crates/app/Cargo.toml`, `crates/app/Cargo.lock`, `crates/app/src/main.rs`, `crates/app/src/theme.rs`, `crates/app/src/icons.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Linux, in an X11 session and in a Wayland session, run `mise run app:dev` (the build now compiles both backends). Expected: the window opens, draws, and takes the keyboard.

### App: real-device checks of the Appearance tab (mode, shipped themes) (native-themes)

The `native-themes` feature made the GPUI app's theme a setting: Step 2 stored `theme` (mode System / Light / Dark), added a Neutral light palette and an `Appearance` tab; Step 3 vendored gpui-component's 21 theme files (36 themes) and added a light-theme and a dark-theme `Select`. `mise run ci:local` and the unit tests cover the setting rule, the mode resolution, the palette mappings, the `Roles` derivations and the theme-file parsing. Nobody has looked at the result on a real machine: both steps' checkboxes were ticked on the automated criteria only, so Light, System following the OS, the shipped themes' readability, the fixed judgment colors on light surfaces and the pickers' keys were never exercised on screen. Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): Neutral Light, Catppuccin Latte, Tokyo Night and Neutral dark, set through MCP `set_settings`, applied at once to the panes, the strip, the status row, the title bar and the menus; in the light themes a rejected cell's dim was too faint and the filter menu's `Face focus` `Good` green too bright (since fixed by `light-theme-dim-and-menu-green`); System, a restart and the pickers' keys were not run. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): set through MCP `set_settings`, Light (Neutral) turned the panes, the strip, the title bar and the meta pane light and readable, the stars and the burst badges too. In System mode with Catppuccin Latte and Tokyo Night chosen, switching the OS appearance swapped the app between them live, both ways. Dark stayed Tokyo Night with the OS in light, and a restart restored the theme on the first frame. Not run: the dialogs and the settings box under each theme (no MCP tool opens the settings), and the pickers' keys. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): under Light (Neutral), Catppuccin Latte and Tokyo Night, set through MCP, the settings box (Appearance page) and the `Move to Trash` dialog took each theme and stayed readable; the strip's stars and pick dot read on the light strip, and the label Tags stayed readable on a hovered row in Neutral light and dark. The pickers' keys passed: a click opened the dark theme `Select`, Escape closed the menu only, Space opened it, Down moved, Enter picked (`Tokyo Moon`), Enter on the focused box opened it again, and a second Escape closed the settings. A quit and relaunch with Catppuccin Latte in Light mode came up in Latte (the first frame was not captured). Plan: `docs/plans/_archived/20261115-native-themes/plan.md`.

Files: `crates/app/src/theme.rs`, `crates/app/src/main.rs`, `crates/app/src/settingsview.rs`.

#### TODO

- [ ] On Windows, on a host whose desktop no other session is using, in the Appearance page (`ctrl+,`): choose System and switch the OS appearance while the app runs (also with two non-Neutral themes chosen): the app follows it without a restart. Choose Dark: it stays dark whatever the OS does. (Not run on 2026-11-27: switching the OS appearance changes the shared desktop under other sessions' checks.) In Light, check the tree cursor's ring and the progress bar. With a shipped dark theme (e.g. Tokyo Night) and a shipped light theme (e.g. Catppuccin Latte), check the burst band, the preview's surround taking the darkest of the theme's surfaces (`Roles::preview`; black in Neutral dark, per `preview-theme-background`, plan `docs/plans/_archived/20261118-preview-theme-background/plan.md`) and the strip's file names and failures, plus the reject dim on both themes and the stars and pick dot on the dark one (only the light strip's stars and pick dot were seen on 2026-11-27). Pick Neutral again: the app looks exactly as before (Neutral dark unchanged). Restart: the choice is restored before the window is drawn (restored on 2026-11-27, the first frame not captured). Record what is hard to read in the archived plan's `learnings.md`.
- [ ] On macOS, open the settings (`meta+,`), Appearance: expect the dialogs and the settings box to turn light under Light and to take a shipped dark and a shipped light theme, staying readable. Pickers' keys: Space / Enter open the menu, the arrows move, Enter picks, Escape with the menu open closes the menu only, with it closed closes the settings. (On 2026-11-26 Light, System following the OS with two shipped themes, Dark and the restart were seen, set through MCP.)
- [ ] On Linux, open the settings (`ctrl+,`), Appearance: expect the dialogs and the settings box to turn light under Light and to take a shipped dark and a shipped light theme, staying readable; Dark stays dark whatever the desktop does; a restart restores the choice. Pickers' keys: Space / Enter open the menu, the arrows move, Enter picks, Escape with the menu open closes the menu only, with it closed closes the settings. Check System with `window.appearance()` following the desktop's appearance switch.

### App: real-device checks of the right-click menus drawn as gpui-kit's PopupMenu (native-context-menu)

`native-context-menu` replaced the hand-built `div` menus behind the folder tree's and the thumbnail strip's right-click menus (`context_popover`, `context_row`, `context::Menu` / `Open`) with gpui-component's `PopupMenu` through gpui-kit, placed by the view itself with `deferred` / `anchored` so the menu still flips near the window's edges. `mise run ci:local` and the remaining `context.rs` tests (`folder_groups`, `strip_groups`, `shown_path`) cover the items and groups. The crate has no GPUI view test, so the menu itself was never exercised: the step was ticked on the automated criteria only. Four things were read from the source and never run: the key order (the menu takes `escape` / `up` / `down` / `enter` before the view's `key_down`, while `left` / `right` and every other key still reach it), the canvas-based hand-back of the tree's keyboard on a press on the menu, the focus returning to the view after every close path, and `Rename…` ending with the inline `Input` focused. Windows only. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): the tree's menu on the home root and a non-root folder, with and without the tree holding the keyboard, opened as a `PopupMenu` at the pointer with its groups separated, the root without `Rename…` and `Expand All` / `Collapse All`, the open folder unchanged and, with the keyboard held, Down afterwards moving the tree cursor; the strip's menu kept a three-file selection when opened inside it and replaced it outside, a gap and the empty area opened nothing, the JPEG-only folder's menu had no judgment groups, and the judgment groups showed `Kbd` shortcuts with the current flag, stars and label checked; a cell at the right edge opened the menu to the pointer's left, near the bottom above it, and in a 597 px window it ran from y 74 (the 34 px title bar plus 40) to the bottom and scrolled; `Rename…` ended with the inline `Input` focused and the name selected, `Copy File Name` filled the clipboard, a star row and `p` judged the target, and `Move to Trash…` opened its dialog. Plan: `docs/plans/_archived/20261116-native-context-menu/plan.md` (the steps and expected results are also in `learnings.md` in the same folder). Files: `crates/app/src/main.rs`, `crates/app/src/context.rs`, `crates/app/src/folderops.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On Windows, with a menu open. Escape closes it only. A keymap key (for example `p`) closes it and acts. Left / Right close it and page the strip. A press outside closes it. A second right-click elsewhere replaces it. After each close the keymap works again (the focus is back on the view), and a press on the menu while the tree had the keyboard hands it back. (Up / Down moving the highlight and Enter running the item were fixed and checked on both menus on 2026-11-28: `docs/plans/_archived/20261128-context-menu-keys/results.md`; the `menu_takes_*` UI tests pin them with Escape, `p` and Right.)

### App: real-device check of the catalog-served icons (native-catalog-icons)

#### Background

`native-catalog-icons` made the GPUI app (`crates/app`) serve `star`, `square`, `rectangle-vertical`, `rectangle-horizontal` and `scan-face` from `gpui-kit-assets` instead of its own copies of the Lucide SVGs. `star` and `square` come from the default `Assets` set, and the other three are selected with `icon_assets!`. The app keeps only the three derivatives the catalog lacks: the filled star, the filled square and the frowning `scan-face`. Unit tests cover the asset source: every one of the eight icon paths loads an SVG, each catalog const equals its `IconName` variant's `path()`, a derivative loads its own bytes, and an unknown or empty path loads nothing. `mise run ci:local` and clippy pass. The catalog's SVGs differ from the old copies only in whitespace and in carrying `width="24" height="24"`, so no visible change is expected. Nobody has looked at the rendered icons in the running window, and Step 1's checkbox was ticked on the automated criteria only. Seen on Windows 11 on 2026-11-23 (results in `docs/plans/_archived/20261123-windows-device-checks-mcp-visual/results.md`): the `Filter ▾` menu's star rows (filled and outlined), the orientation rectangles and the `Face focus` icon, and the strip's good and low face icons, were judged OK by the user in Neutral dark; the label squares are gpui-component `Tag`s now and no longer catalog icons. Plan: `docs/plans/_archived/20261116-native-catalog-icons/plan.md`.

Files: `crates/app/src/icons.rs`.

#### TODO

- [ ] On macOS and Linux (any platform at hand), open the `Filter ▾` menu and a scanned folder and confirm the star rows, the orientation rectangles, the `Face focus` faces and the strip's face focus icon on a good and a low frame look right.

### App: real-device check of the strip menu's judgment order and star rows (native-strip-menu-order)

#### Background

`native-strip-menu-order` made the strip's right-click menu in the GPUI app (`crates/app`) follow the Filter menu for its judgment groups. The flag group now runs Pick, Unflag, Reject. The rating group runs from 5 stars down to the unrated item, and each row draws the Filter menu's star icons in place of the `N stars` / `No stars` text. The label group is unchanged. Each item keeps its shortcut, its checked state and its `Action::Judge` action. `context::Item.label` now reuses `menu::Label`, and `main.rs` draws both menus' labels through one `menu_label` helper. Unit tests in `context.rs` assert the new order and the star counts, and a pin test compares the flag, rating and label order with `filter_rows`. `menu.rs`'s tests pass unmodified, and `mise run ci:local` and the PR's GitHub checks pass. Nobody has right-clicked a strip cell in the running app, and Step 1's checkbox was ticked on the automated criteria only. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): on a picked, 3-star, Yellow file the menu read Pick, Unflag, Reject, five rows of 5 to 1 lit stars and a row of five dim stars, Red to Purple then No label, with the checks on Pick, the 3-star row and Yellow and the shortcuts (`P`, `U`, `X`, `5` … `0`, `Ctrl+Alt+1` … `Ctrl+Alt+0`) at the right end; a click on the 4-star row set 4 stars. Plan: `docs/plans/_archived/20261116-native-strip-menu-order/plan.md`.

Files: `crates/app/src/context.rs`, `crates/app/src/main.rs`.

#### TODO

- [ ] On macOS and Linux (any platform at hand), run `mise run app:dev` with `--settings` and `--log` pointing into a test folder (as `docs/agents/app.md` requires, so the real store is not touched). Open a folder and right-click a strip cell of a picked, rated (for example 3 stars), labeled file. Expected: the flag group reads Pick, Unflag, Reject. The rating group shows five rows of 5 down to 1 lit yellow stars, then a row of five dim stars. The label group is Red to Purple, then No label. The checks sit on Pick, the 3-star row and the file's label. Each row shows its shortcut at the right end (`P`, `U`, `X`, `5` ... `1`, `0`, the label keys). Clicking a star row sets that rating.

### App: real-device checks of the drag-resizable strip height (native-strip-resize)

`native-strip-resize` made the thumbnail strip's height in the GPUI app (`crates/app`) draggable by its top edge, stored under the `stripHeight` key of the settings store (default 128 px, 96 to 302 px, the preview at least 240 px), with the cells' image box scaling with the height. Unit tests (`stripheight.rs`, `strip.rs`'s geometry, hit test, range, content width and carried-offset tests, `settings.rs`'s read of `stripHeight`) and `mise run ci:local` cover the arithmetic and the setting. On Windows 11 on 2026-11-21 a drag moved the strip between 96 and 302 px with the current cell staying in view, and the marks were legible at 96 px. Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-resize-scrollbars/results.md`): the cursor turns row-resize on y 802 to 807 under the 1 px line at y 801, which reads on black in Neutral dark, with a cell click and the scrollbar's thumb drag and track clicks still working; at 302 px and 100 % the 404 px thumbnails are drawn at about 1:1 and were legible; every release, over the preview, a pane, the status row or outside the window, stored the height at once (302 `MAX`, 179, 96 `MIN`, 302) and came back at it after a restart; `toggleStrip` hid the strip, its header and its line and brought them back at the dragged 302 px; with 199 stored, `--auto 40` started at 128 and a drag wrote nothing. The 1:1 re-cut after a strip edge drag was seen in batch 7 on the same day (`docs/plans/_archived/20261122-windows-device-checks-7/results.md`, the always-visible scrollbars' box 7), and the same re-cut after a pane drag in this run. The sharpness on a HiDPI display is left. The limits (96 / 302 / 240) were not judged by eye. Plan: `docs/plans/_archived/20261117-native-strip-resize/plan.md`. Files: `crates/app/src/main.rs` (`strip_handle`, `StripDrag`, `drag_strip`, `end_pane_drag`, the wrapper `div` around the middle row and the strip in `render`), `crates/app/src/stripheight.rs`, `crates/app/src/strip.rs` (`Geometry`, `Strip::set_geometry`).

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store (except where a check says to restart).

#### TODO

- [ ] On Windows, at 302 px on a HiDPI display, check the thumbnails' sharpness. At 100 % (96 DPI) the 404 px thumbnails are drawn at about 1:1 in the 405 x 270 box and were legible on 2026-11-22; no HiDPI display was at hand.
  - Not run on 2026-11-27 either: the host has one 5120 x 1440 display at 100 %, and changing its scale would disturb the desktop other sessions share.
- [ ] On macOS and Linux, repeat the hover, drag, scaling, restart, 1:1 and hide / show checks above. CI only builds these platforms.

### App: real-device checks of the always-visible scrollbars

#### Background

The `native-scrollbars-always` feature made `theme::apply` set the GPUI app's theme `scrollbar_mode` to `ScrollbarMode::Always` (in `theme.rs`'s `finish`). The folder tree's bar, the strip's `StripBar`, the meta pane's bar and the dialog bodies' bars now show at rest and take a press without scrolling first. The mode is global, so the settings modal's lists, the `Sort ▾` / `Filter ▾` popovers, `PopupMenu` and `Select` lists show their bar when they overflow too. A unit test on a plain `Theme` and `mise run ci:local` cover that the mode is `Always` after a vendored config. Nobody has watched the running window: the bars' look and drag were never exercised. gpui-base 0.9.1 still skips a bar whose content fits, so a short folder should show no bar. Plan: `docs/plans/_archived/20261117-native-scrollbars-always/plan.md`. The `native-scrollbar-gutter` feature then reserved the bars' 16 px track instead of leaving them over content: the tree row's right padding and the meta pane's body padding (`theme::SCROLLBAR_TRACK`) and `stripheight::GUTTER`, a 9 px band under the strip's cells. `mise run ci:local` and the unit tests (`SCROLLBAR_TRACK` against gpui-base's `Scrollbar::width()`, `stripheight::dragged` with the gutter) cover the arithmetic; the on-screen result was never exercised, and both features' checkboxes were ticked on the automated criteria only. Plan: `docs/plans/_archived/20261117-native-scrollbar-gutter/plan.md`. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-7/results.md`) the tree's, the strip's, the meta pane's and an overflowing `Import…` dialog body's bars showed at rest, and the settings' list, a `PopupMenu` and a `Select` list showed theirs; dragging the tree's, the strip's and the dialog body's thumbs and clicking their tracks scrolled them (the strip's thumbnails loading); with three cells and the tree collapsed no bar was drawn and presses reached the cells and the rows; nothing lay under the tree's thumb at rest, hovered or dragged, the badge's pill ending 16 px from the pane's edge and 7 px from the resting thumb (x 218-223), and a press on the pill's right end selected its row; the meta pane's values ended 7 px left of its thumb, and no `Import…` text lay under that dialog's bar; the strip's bar sat under the names with nothing under its thumb, a burst's band included, and a press and a right press on a name band selected the cell and opened its menu; a short folder left an empty 9 px band, the header's line stayed at y 801 from folder to folder, the edge drag stopped at `MIN`, `MAX` and `PREVIEW_MIN`, a 1:1 crop was re-cut after a drag, and a 603 px window's filter menu fitted above the header; the bars looked acceptable in Neutral dark, Neutral light and Catppuccin Mocha. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c6/results.md`): in a 637 px window a drag of the meta pane's thumb scrolled it and a press on its track above the thumb scrolled it back, with no wheel first; an error line in the pane's notes wrapped with its ink 13 px from the pane's right edge (the glyph's side bearing plus the 12 px of `pr_3`). What is left open is in the boxes below.

Files: `crates/app/src/theme.rs` (`finish`, `apply`, `SCROLLBAR_TRACK`), `crates/app/src/main.rs` (`tree_pane`, `meta_pane`, `render`), `crates/app/src/stripbar.rs` (`StripBar`), `crates/app/src/stripheight.rs` (`GUTTER`, `dragged`), `crates/app/src/modal.rs` (`shell`), `docs/agents/app.md` ("The strip, the tree and the meta pane draw the theme's scrollbar").

#### TODO

- [ ] On Windows, compare a tree that does not overflow with a build from before `native-scrollbar-gutter`. Expect it to look unchanged apart from 4 px more room right of the badges.
  - Not run on 2026-11-27: it needs a build of a commit from before #1147, when the crate was `crates/native`.
- [ ] On Windows, in System mode on the Appearance page, change the OS appearance (light / dark) and look at the bars. Expect the bar's look to stay acceptable. (Neutral dark, Neutral light and Catppuccin Mocha chosen on the page were judged acceptable on 2026-11-22.)
  - Not run on 2026-11-27: switching the OS appearance changes the desktop other sessions share.
- [ ] On macOS and Linux, check that the tree's, the strip's, the meta pane's and an overflowing dialog body's bars show at rest and that a thumb drag and a track click scroll each without scrolling first, that nothing lies under the bars (the 16 px track and the strip's 9 px gutter), that the meta pane's notes and errors keep 12 px from its right edge, and that the bars look acceptable in a light and a dark theme and under System following the OS appearance.

### App: real-device checks of the strip's header row with Sort and Filter (native-strip-header)

#### Background

`native-strip-header` moved `Sort ▾` and `Filter ▾` of the GPUI app (`crates/app`) from the status row to a new 32 px header row on the strip's top edge. The resize handle now sits on the header's top edge. The status row is pinned to 30 px (`STATUS_ROW_HEIGHT`), and the popover's height cap is `menu_max_height` less the strip pane's height. `stripheight::dragged` takes the pane's wanted height and takes `HEADER` off itself. `stripHeight` still means the cells' area height. CI, the `stripheight.rs` unit tests, the unchanged `companion.rs` / `pager.rs` tests and `mise run ci:local` cover the arithmetic and the build. The layout, the popover's placement and the drag were never seen in the running window. The step was ticked on the automated criteria only. macOS and Linux are covered by the CI build only. Plan: `docs/plans/_archived/20261117-native-strip-header/plan.md`. On Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-7/results.md`) the header showed `Sort ▾` and `Filter ▾` at its right end; each menu opened upward from its button and closed on a second press, a press outside and `escape`; the filter button was lit under a filter, its border going to `input` while hovered, and the sort button was dimmed and inert in a JPEG-only folder; in a 603 px window with the strip as tall as allowed the long filter menu stayed inside the window, scrolled and toggled from its button, unclipped; the resize cursor (read with `GetCursorInfo`) showed on y 802-806, the 5 px under the 1 px line at y 801, and the button's hand cursor began at y 808; the drag stopped at `MIN`, `MAX` and `PREVIEW_MIN`; at 128 the image box was 144 x 96; a press, a right press and a wheel on the header selected, opened and scrolled nothing; `toggleStrip` hid the header and closed an open menu, the status row stayed 30 px with the circle shown and gone, and a filter showed `n / shown (all)`; the default window's preview canvas measured 1158 x 767 (1160 x 768 with the pane borders and the header's line), not the 1160 x 810 inferred, the status row 30 px and the header 32 px. What is left is the macOS and Linux box below.

Files: `crates/app/src/main.rs` (`strip_header`, `status_row`, `menu_button`, `change_panels`), `crates/app/src/stripheight.rs` (`HEADER`, `dragged`), `docs/agents/app.md`.

Run each check with `--settings <fresh test store>`, `--index <test index>` and `--log <test file>`, never the real store.

#### TODO

- [ ] On macOS and Linux, repeat the header, popover, handle, drag and `toggleStrip` checks above. CI only builds these platforms.

### App: real-device checks of the packages the Package workflow builds (gpui-packaging)

The `gpui-packaging` feature added the manual `Package` workflow and `tools/package/` scripts. CI covers building, `hdiutil verify` and `codesign --verify` of the DMGs, the silent install, upgrade and uninstall of the setup `.exe` on an elevated runner, `install.sh` under a throwaway `HOME` (including the wrong-digest refusal), and `apt` installing and removing the `.deb`. Never exercised: the local mise package tasks on a real machine, a launch from any package, Gatekeeper's first-launch prompt, the absence of a UAC prompt for a standard Windows user (the hosted runner is elevated), the Start menu entry, the desktop menu entry and window icon matching on Wayland and X11, and the Vulkan-less GL fallback. Plan: `docs/plans/_archived/20261117-gpui-packaging/plan.md`. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): `mise run app:package:macos` built `Handpic-2.1.0-macos-arm64.dmg` (the name carries the crate's version, not `0.0.0`); it opened and the copied app started. Gatekeeper's first-launch prompt cannot be reproduced with a local build, which carries no `com.apple.quarantine` attribute (only `com.apple.provenance`). `spctl --assess` rejects the ad hoc signed app. The native-release Step 5 install docs (`README.md`, `docs/humans/usage.md` and the `.ja.md` pairs) were written without a macOS or Linux machine. Plan: `docs/plans/_archived/20261121-native-release/plan.md`.

Files: `mise.toml`, `tools/package/macos.sh`, `tools/package/windows.sh`, `tools/package/linux.sh`, `crates/app/packaging/windows/handpic.iss`, `crates/app/packaging/linux/install.sh`, `crates/app/packaging/linux/control`, `README.md`, `README.ja.md`, `docs/humans/usage.md`, `docs/humans/usage.ja.md`.

#### TODO

- [ ] On macOS, with a `Handpic-<version>-macos-<arch>.dmg` that carries the quarantine attribute (a downloaded one; a local `mise run app:package:macos` build has none), drag `Handpic.app` to `Applications` and start it. Gatekeeper asks for the ad hoc signed app's first launch to be allowed under System Settings, Privacy & Security. Expect the app to start. (The local build, the DMG and the launch passed on 2026-11-26.)
- [ ] On Windows, install Inno Setup 6 (`winget install JRSoftware.InnoSetup`), then run `mise run app:package:windows`. Expect `crates/app/target/release/Handpic-0.0.0-windows-x64-setup.exe` and `Handpic-0.0.0-windows-x64.zip`. As a standard (non-admin) user, run the setup. Expect no UAC prompt, the app under `%LOCALAPPDATA%\Programs\Handpic`, and a Start menu entry `Handpic` that starts it. Run the setup again silently (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`) and expect no prompt. Uninstall from Settings, Apps.
  - Not run on the 2026-11-27 host: it installs Inno Setup and a per-user Handpic under `%LOCALAPPDATA%\Programs\Handpic`, where the host's own Handpic install lives.
- [ ] On Linux x86_64 with `dpkg-dev` and the GPUI apt list installed, run `mise run app:package:linux`. Expect `crates/app/target/release/Handpic-0.0.0-linux-x64.tar.gz`, `install.sh` and `handpic_0.0.0_amd64.deb`. Run `sh install.sh <tarball> <its sha256sum>`, then start Handpic from the desktop menu on a Wayland and an X11 session. Expect the window to show the Handpic icon (matched by `dev.minodisk.handpic`). Install the deb with `sudo apt install ./handpic_0.0.0_amd64.deb`, start it from the menu, and remove it with `sudo apt remove handpic`. If a machine without a Vulkan driver is at hand, check that the app still starts (wgpu's GL fallback through `libEGL`).
- [ ] On Linux, with a Riffle deb already installed (`dpkg -l riffle`), run `sudo apt install ./handpic_<version>_amd64.deb`. Expect apt to keep Riffle installed (no removal, no `Replaces` / `Conflicts` prompt, no file overwrite), both launchable, and `dpkg -s handpic` to show `libegl1` in `Depends`. (No `Conflicts` / `Replaces: riffle` was decided from the packaging alone in `docs/plans/_archived/20261127-deb-libegl1-no-replaces/plan.md`.)
- [ ] On a Mac with macOS 15 or later, download the latest release's `Handpic-<version>-macos-arm64.dmg` with a browser, drag `Handpic.app` to `/Applications` and open it. Expect Gatekeeper to block it, and System Settings, Privacy & Security, Open Anyway to let it start, as `README.md` / `docs/humans/usage.md` "Installing" (and their Japanese pairs) say.
- [ ] On Linux, run the `install.sh` command of the usage guide against a release's `SHA256SUMS`. Expect `~/.local/bin/handpic` to start from the desktop menu.


### App: real-device check of the screenshot tool's failed-capture path on Linux (native-qa)

#### Background

The `native-qa` feature added a `screenshot` MCP tool, listed only in a `--features screenshot` build of `crates/app`. Its Done-when says a failed capture (Linux's "not implemented for this platform", a lost device) is a tool error with a message, not a panic. The capture was exercised on Windows only. There was no Linux host, so the error path was reasoned about from the code, and no CI job builds or runs the feature on Linux. Plan: `docs/plans/_archived/20261118-native-qa/plan.md`. Files: `crates/app/src/main.rs` (`screenshot`), `crates/app/src/screenshot.rs` (`capture`).

#### TODO

- [ ] On Linux, build the app with `--features screenshot` (`cargo run --features screenshot -- --mcp-port <port>` with `mcpEnabled` on), call the `screenshot` tool through the MCP endpoint, and expect a tool error carrying `render_to_image`'s message ("not implemented for this platform"), the app still running and the window still drawing, not a panic or a bridge timeout.

---

### App: watch the macOS app CI job's cold-cache duration against the 30-minute timeout after the test-support dev-dependency (native-qa)

#### Background

`native-qa` Step 4 added `gpui-pre-platform` with `test-support` as a dev-dependency of `crates/app`, so the GPUI crates compiled in a second variant (plain for the bundle step, `test-support` for clippy and tests), and cold-cache `app (macos-latest)` runs took 24 to 29.5 minutes against `timeout-minutes: 30` and were cancelled at the limit on #1267, #1272 and #1286. `macos-ci-bundle-reuse` (`docs/plans/20261127-macos-ci-bundle-reuse/plan.md`) made the bundle step build with the `screenshot` feature, so it reuses the test build's dependencies and compiles only `handpic-app`, and kept the limit at 30 (see "The macOS bundle step reuses the test build" in `docs/agents/app.md`). The saving is expected to bring a cold run to about 17 to 21 minutes but was not seen on a cold run yet. Files: `.github/workflows/ci.yml` (the `app` job), `mise.toml` (`app:bundle:macos`).

#### TODO

- [ ] A cold-cache `app (macos-latest)` job (one whose `actions/cache` step restores nothing) finishes inside the 30-minute limit on three consecutive PRs. If one does not, raise `timeout-minutes` in `.github/workflows/ci.yml` and record it in `docs/agents/app.md`.

---

### App: real-device check of the default window's centering on the visible area (todo-ten-closeouts)

#### Background

`todo-ten-closeouts` Step 5 replaced `Bounds::centered` for the default window with `session::default_bounds(primary, workspace)`, which centers 1600x1000 in the primary display's visible area. On Windows (`cfg!(windows)` in `window_placement`) it subtracts the visible area's offset from the display's corner, because GPUI's Windows backend hands the bounds to `SetWindowPlacement` as workspace coordinates. The earlier measurement (one 5120x1440 display, a 62 px left taskbar) put the window 93 px right of the display's center: 31 px (the visible area's center offset) plus the taskbar's 62 px. The unit tests in `crates/app/src/session.rs` and the PR's GitHub checks cover the formula; the window's real rectangle was not read at the time, because no local build was made on that host. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): with the taskbar docked left and a fresh `--settings` store, the window measured 1616x1008 outer (client 1600x1000), centered to the pixel horizontally at x 2591 and within the frame's 3.5 px vertically, 31 px right of the display's center, no longer 93 px; the result is recorded under "A launch restores the last folder and the window" in `docs/agents/app.md`. Plan: `docs/plans/_archived/20261123-todo-ten-closeouts/plan.md`.

Files: `crates/app/src/session.rs` (`default_bounds`), `crates/app/src/main.rs` (`window_placement`), `docs/agents/app.md` ("A launch restores the last folder and the window").

#### TODO

- [ ] On Windows 11, with no `windowBounds` used (`--auto 3 <folder>`, a fresh `--settings` store), open the app with the taskbar at the bottom and read the window's outer rectangle (`GetWindowRect` or a screenshot). Expect the 1600x1000 client centered in the visible area (the display less the taskbar), within the frame's few px. (Not run on 2026-11-27: the host's taskbar setting is the user's; the left-docked case passed.)

---

### App: real-device checks of the update flow on macOS, Linux and a standard Windows account (native-updater)

The `native-updater` feature gave the app a launch check, an Updates page, and an update path per install kind (`self_update` for the portable ZIP and the tarball, the staged setup for Inno Setup, a staged swap of the whole bundle for a `.app`, and a notice with the release page for a deb). CI and the engine's tests cover the updater against a fake GitHub API (the policy table, a ZIP and a `.tar.gz` update, a bundle swap in a temp folder, every digest and signature failure, the setup's staging and both verifications) and, for Inno Setup, a `/WAITPID` and `/RELAUNCH` upgrade on the Windows runner. On the developer's Windows PC the portable ZIP and Inno Setup rows were measured from rc.1 to rc.2 on the test feed `minodisk/handpic-update-test`. Never exercised: the `.app`, tarball and deb paths on a real machine (no macOS or Linux machine was at hand), the real `codesign` swap outside CI, a standard Windows account (the PC's token was non-elevated but the hosted runner is elevated), a real network cut (offline was simulated with `HTTPS_PROXY=http://127.0.0.1:9`), and the gpui-kit matrix rows "a release published mid-install" and "interrupted install, failed relaunch, or downgrade". The dry run with the production key on the real feed `minodisk/handpic` (2026-11-21, the native-release plan's Step 4, `v2.0.0-rc.1` and `v2.0.0-rc.2` kept as prereleases) measured the portable ZIP and Inno Setup rows again on the same Windows PC and answered none of the boxes below, which stay open (no macOS or Linux machine; the standard account, the real network cut and the skipped matrix rows were not run). See "The rc dry run on the real feed" in `docs/plans/20261109-gpui-port/packaging-updater.md`. The rcs on the real feed can serve these boxes in place of the test feed. Test builds come from a `Package` run with `update_test_repo` set and `tools/package/test-release.sh` (see `docs/agents/app.md`). Plan: `docs/plans/_archived/20261118-native-updater/plan.md`.

Files: `crates/engine/src/update.rs`, `crates/app/src/update.rs`, `crates/app/src/main.rs`, `crates/app/src/settingsview.rs`, `tools/package/test-release.sh`, `docs/plans/20261109-gpui-port/packaging-updater.md`.

#### TODO

- [ ] On macOS, with an rc.1 `Handpic.app` in `/Applications` and rc.2 on the test feed, launch it. Expect the Updates page to go `Checking`, `Downloading`, `Verifying`, `ReadyToRestart` without a click, only `Handpic.app` left in `/Applications`, and `Restart Now` to log the three `quit:` drain lines and reopen the bundle as rc.2. Record whether Gatekeeper prompts after the swap and whether the folder-access grants survive it.
- [ ] On macOS, open the same rc.1 build from the mounted DMG or from `~/Downloads` (translocated). Expect `Failed` with the "move Handpic.app to /Applications" message and nothing changed.
- [ ] On Linux, with an rc.1 tarball install from `install.sh` and rc.2 on the test feed, launch it. Expect the same phases without a click, the pane's note, and `Restart Now` to log the three `quit:` drain lines and come back as rc.2 (through the path read at launch, not the ` (deleted)` one). A second launch within the hour must not check (`updateCheckedAt`).
- [ ] On Linux, with an rc.1 deb install (`/usr/lib/handpic`), launch it. Expect the notice with `Open Release Page`, which opens the release in the browser, and nothing installed.
- [ ] On Windows, as a standard (non-admin) user, with an rc.1 Inno Setup install and rc.2 staged, quit plainly and then use `Restart Now`. Expect no UAC and no SmartScreen prompt, rc.2 installed, and a relaunch only after `Restart Now`.
- [ ] On any platform, cut the network for real (disable the adapter) at launch and on `Check for Updates`. Expect the app to stay usable and `Failed` to be retryable. On the Updates page, check that `Check for Updates` is disabled while a check runs and that `Open Release Page` opens the browser (the page's click paths were never driven).
- [ ] On a real machine, try the rows the rehearsal skipped: publish a release while an install is downloading, interrupt an install, and a failed relaunch, and note what is left launchable (the engine claims no rollback beyond the `.app` bundle mode's restore of the old bundle).

### App: real-device checks of the Windows GUI-subsystem release build (windows-gui-subsystem)

#### Background

The windows-gui-subsystem feature builds the Windows release `handpic.exe` for the GUI subsystem (`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` in `main.rs`), so a launch from Explorer, the Start menu or an updater relaunch opens no terminal. The automated checks passed. The PE `Subsystem` field reads 2 on the release exe and 3 on the debug exe. A `Start-Process` launch had no `conhost.exe` child and `AttachConsole` failed. The `--log` file held the launch's lines, and a provoked panic left its `panic:` line and exited 101. Two checks were never exercised. One is a real double-click in Explorer. The other is PowerShell 7 (`pwsh`), which isn't installed on the measuring machine, so only Windows PowerShell 5.1 was measured. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): the installed release 2.1.0 launched through `explorer.exe` on a `.lnk` (ShellExecute, as a double-click does) had parent `explorer.exe`, no child process, one window and no console window. Plan: `docs/plans/_archived/20261121-windows-gui-subsystem/plan.md`.

Files: `crates/app/src/main.rs`, `crates/app/src/logger.rs`, `docs/agents/app.md`, `docs/agents/app-log.md`, `docs/humans/performance.md`, `docs/humans/performance.ja.md`

#### TODO

- [ ] On Windows with PowerShell 7 (`pwsh`), run `handpic.exe ... > file` on the release build. Expected: record whether the file captures the release build's stdout (the `page` lines and quit summary). Windows PowerShell 5.1 does not: the prompt returns at once and the file stays empty. If `pwsh` captures it, add it to the table in `docs/agents/app.md` and to the Windows paragraph of `docs/humans/performance.md` / `.ja.md`. (PowerShell 7 is still not installed on the measuring host as of 2026-11-27.)

---

### App: real-device check of Handpic installed beside Riffle 1.10.0 on Windows (native-release)

#### Background

The native-release rc dry run (`v2.0.0-rc.1` / `v2.0.0-rc.2` on `minodisk/handpic`, 2026-11-21) measured the portable ZIP and Inno Setup rows on one Windows PC. Only the coexistence of the two installs was seen. Never exercised: what the two uninstall entries and Start menu shortcuts look like with Riffle 1.10.0 installed and used, and the first launch's settings seeding from Riffle's `settings.json`. The user left the row Unverified. CI and the `settings.rs` tests cover the import. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): Riffle 1.10.0 and Handpic 2.1.0 are installed side by side with two uninstall entries (`Riffle`, `dev.minodisk.handpic_is1`), only Handpic has a Start menu shortcut, and there is no Riffle `settings.json` on that host, so the seeding could not be seen; nothing was uninstalled. Plan: `docs/plans/_archived/20261121-native-release/plan.md`. Record: "The rc dry run on the real feed" in `docs/plans/20261109-gpui-port/packaging-updater.md`.

Files: `crates/app/src/settings.rs`, `crates/app/packaging/windows/handpic.iss`, `docs/plans/20261109-gpui-port/packaging-updater.md`.

#### TODO

- [ ] On Windows with Riffle 1.10.0 installed and used (a `settings.json` holding non-default settings), install Handpic from `Handpic-2.0.0-rc.1-windows-x64-setup.exe` (or a later release). Expect two uninstall entries and two Start menu shortcuts, each removable on its own, and Handpic's first launch to seed its settings from Riffle's `settings.json` (the sidecar format, the keymap and the other keys `settings.rs` imports).

---

### App: the Inno Setup uninstaller leaves the update staging folder (native-release)

#### Background

Observed in the native-release rc dry run (2026-11-21): after an uninstall from Settings, Apps, the staging folder `<cache dir>\dev.minodisk.handpic\update` still held the staged rc.2 setup and its ZIP. `handpic.iss` now deletes it through an `[UninstallDelete]` entry and keeps the index, settings and logs; the uninstall itself is not yet checked. Inno Setup records the entry in the uninstall log at install time, so only an install that a setup built with the entry has installed or upgraded deletes the folder. Plan: `docs/plans/_archived/20261121-native-release/plan.md`.

Files: `crates/app/packaging/windows/handpic.iss`, `docs/agents/app.md`, `docs/plans/20261109-gpui-port/packaging-updater.md`.

#### TODO

- [ ] Pending manual check, on Windows: with an Inno Setup install made by a setup built with the `[UninstallDelete]` entry and an update staged by it, uninstall from Settings → Apps and confirm `%LOCALAPPDATA%\dev.minodisk.handpic\update` is gone while `index.sqlite`, `settings.sqlite` and the logs remain. Once checked, record the result in the rc dry run section of `docs/plans/20261109-gpui-port/packaging-updater.md` and replace the **Unverified** note in the Inno Setup rule of `docs/agents/app.md`.

---

### App: real-device checks of the capital executable name (`Handpic.exe`, `Contents/MacOS/Handpic`)

#### Background

The `capital-exe-name` feature ships the Cargo bin `handpic` as `Handpic.exe` in the Windows portable ZIP and the Inno Setup install, and as `Contents/MacOS/Handpic` in `Handpic.app`; Linux keeps `handpic`. The engine's updater now looks the executable up in an archive by a fixed name per install kind (`Handpic/Handpic.exe`, `Handpic/bin/handpic`) instead of the running exe's own file name, and writes it onto the running exe's path, so a portable `handpic.exe` keeps its name. The unit tests cover the lookup and the kept name, `mise run app:package:windows` was run locally and its ZIP listed, and a throwaway Inno Setup script showed an upgrade writes the new casing (`x.exe` then `DestName` `X.exe` left `X.exe`). Nobody ran a real upgrade over a 2.0.0 install, a real portable update from a test feed or the macOS bundle. A 2.0.0 portable copy is expected to fail its update (it looks for `Handpic/handpic.exe`) and to be reinstalled by hand, as accepted. Seen on macOS on 2026-11-26 (results in `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`): the DMG of `mise run app:package:macos` passed `hdiutil verify`. Its `Handpic.app`, copied to `~/Applications`, held `Contents/MacOS/Handpic`, passed `codesign --verify --strict` (ad hoc, identifier `dev.minodisk.handpic`) and launched. The update from a 2.0.0 bundle was not run (it needs a test feed). Seen in the v2.1.0 `Release` run 45712125456 on 2026-11-25 (after #1241's merge `3bafc9b`), which ran `package.yml` through `workflow_call`: its Windows job's log listed `Handpic.exe` in the installed folder and `Handpic\Handpic.exe` in the extracted ZIP, and the published v2.1.0 assets (`SHA256SUMS` checked) hold `Handpic/Handpic.exe` in the portable ZIP and `Handpic.app/Contents/MacOS/Handpic` in both macOS tarballs. Plan: `docs/plans/20261123-capital-exe-name/plan.md`.

Files: `tools/package/windows.sh`, `crates/app/packaging/windows/handpic.iss`, `tools/package/macos.sh`, `crates/app/packaging/macos/Info.plist`, `mise.toml` (`[tasks."app:bundle:macos"]`), `crates/engine/src/update.rs` (`portable_asset`, `install`).

Test copies get their own `--settings`, `--index`, `--log` and `--mcp-port`, and are quit by their own PID, never by image name.

#### TODO

- [ ] On Windows 11, over a real 2.0.0 Inno Setup install, run the new setup by hand, then (on another 2.0.0 install) through the updater with `Restart Now`. Expect `Handpic.exe` (and no `handpic.exe`) in `%LOCALAPPDATA%\Programs\Handpic`, the Start menu shortcut starting it, Settings → Apps showing Handpic with its icon, the relaunch starting the new version, and the uninstall removing the folder's files and the entry.
- [ ] On Windows 11, a portable copy whose exe is `handpic.exe`, running a build with this change, updates from a test feed (`tools/package/test-release.sh`) to a newer build. Expect the new version after `Restart Now`; record the exe's file name afterwards (expected `handpic.exe`). Repeat with a copy whose exe is `Handpic.exe` (expected to stay `Handpic.exe`).
- [ ] On Windows 11, a 2.0.0 portable copy offered the new release fails its install and stays on 2.0.0, as accepted; record the message shown.
- [ ] On macOS, a 2.0.0 `Handpic.app` in `/Applications` updates to the new bundle from a test feed (`tools/package/test-release.sh`) and restarts into it. (The DMG's launch, `Contents/MacOS/Handpic` and `codesign --verify --strict` passed on 2026-11-26.)

### App: real-device check of the Windows one-line install script (install-scripts)

#### Background

The `install-scripts` feature added `install.sh` (macOS / Linux) and `install.ps1` (Windows) at the repository root, as the primary install method in `README.md` and `docs/humans/usage.md` (and their `.ja.md` pairs). `install.ps1` downloads the latest release's `Handpic-<version>-windows-x64-setup.exe` and `SHA256SUMS`, checks the digest with `Get-FileHash`, and runs the Inno Setup silently. The macOS and Linux scripts were run for real (macOS arm64 against release v2.0.1; Linux in a `debian:stable-slim` container) and `mise run ci:local` passed. At the time `install.ps1` was neither run nor parsed: `pwsh` is not on the development Mac, and CI has no PowerShell check. Never exercised: the install itself, the absence of a SmartScreen warning on the setup and on the first launch, and the `irm | iex` behavior in Windows PowerShell 5.1 and PowerShell 7. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): in Windows PowerShell 5.1, a local copy of `install.ps1` with a wrong digest piped to `iex` downloaded `SHA256SUMS` and the setup, then threw `Checksum mismatch for Handpic-2.1.0-windows-x64-setup.exe; nothing installed.`; the session went on, `$ErrorActionPreference` and `$ProgressPreference` were unchanged, `Handpic.exe` was untouched and no `handpic-install-*` folder was left. Plan: `docs/plans/_archived/20261124-install-scripts/plan.md`.

Files: `install.ps1`.

#### TODO

- [ ] On Windows (x64, in Windows PowerShell 5.1 and again in PowerShell 7), run `irm https://raw.githubusercontent.com/minodisk/handpic/main/install.ps1 | iex`. Expect the download and install lines, then `%LOCALAPPDATA%\Programs\Handpic\Handpic.exe` with `unins000.exe` beside it, the Start menu shortcut and the `HKCU` uninstall entry. Expect no SmartScreen warning on the setup or on the first launch of `Handpic.exe`. (Not run on 2026-11-27: it would replace the user's own install.)
- [ ] On Windows with PowerShell 7, repeat the forced failure (a wrong digest in a local copy of `install.ps1` piped to `iex`). Expect the error to print and the shell to stay open, and `$ErrorActionPreference` and `$ProgressPreference` to be unchanged in the session afterwards. (Passed in Windows PowerShell 5.1 on 2026-11-27; PowerShell 7 is not installed on that host.)
- [ ] On Windows 11 on Arm (if one is at hand), run the same one-liner. Expect the x64 setup to install and start (the setup allows `x64compatible`).

### Engine: real-device check of the chunked Delete Sidecars… move on Windows (chunked-sidecar-delete)

#### Background

`foldersidecars::delete_run` now moves the sidecars through `trash::move_groups`, so `trash::run_chunked` on Windows: up to `CHUNK` (100) sidecars per shell operation, with the per-file fallback after a failed chunk. Before this it moved one sidecar per shell operation. The unit tests cover the outcome with fake movers: the moved and failed files, the recorded run, each `Trashed`'s `owner` and `cleared`. They never reach the real Recycle Bin, and the Windows timing and the progress circle were never exercised. The earlier per-operation cost was about 140 ms a file, against about 6.5-7 ms a file once chunked (the `todo-trash-quit-fixes` measurement for the Undo of Move to Trash). Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): `delete_sidecars` moved 300 `.xmp` in about 1.6 s (5-7 ms a file, against about 140 ms), and the MCP `undo` / `redo` tools restored all 300 in 2759 ms and moved them again in 2075 ms; the progress circle showed empty, then a partial arc, then went, not seen ending full; the timing is recorded in `docs/agents/shared-crates.md`. Plan: `docs/plans/_archived/20261126-chunked-sidecar-delete/plan.md`. Files: `crates/engine/src/foldersidecars.rs`, `crates/engine/src/trash.rs`.

#### TODO

- [ ] On Windows 11, on a folder of a few hundred sidecars, run `Delete Sidecars…` and watch the status row's progress circle: expect it to step per chunk and end full. Then undo and redo with the keys (`Ctrl+Z` and its redo key) rather than the MCP tools: expect the sidecars to be restored and then moved to the Recycle Bin again. (The timing and the undo / redo through the MCP tools passed on 2026-11-27.)

### App: real-device check of the group threshold setting (all-faces-check Step 4)

`all-faces-check` Step 4 added the `groupMinFaces` setting (2 to 20, default 5) to the GPUI app's (`crates/app`) settings modal, Culling page, as `Faces in a group photo` under the auto-advance switch. Pass 3 takes its threshold from it, and lowering it re-lists the open folder so pass 3 runs on the rows that now qualify. Unit and scan-level tests plus CI cover the parse, the range, the visibility rule and the lowering trigger. Nobody has driven the number field in the running app. The step was ticked on the automated criteria only. gpui-component's own number field was not used, because it commits on every keystroke. This is a custom `NumberInput` committed on Enter, blur or a step. Plan: `docs/plans/_archived/20261123-all-faces-check/plan.md`. Files: `crates/app/src/settingsview.rs`, `crates/app/src/main.rs` (`set_group_min_faces`). Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): through the MCP tools, `group_min_faces` was 5 on a fresh store, `set_settings` with 3 started `checking faces 0/4` at once and pass 3 filled exactly the four rows whose `faces_group` was 3 or 4, 25, 1 and a string were refused with the value in force kept, and 3 survived a restart; the field itself was not driven.

#### TODO

- [ ] On Windows, open the settings (`ctrl+,`), Culling page, and drive `Faces in a group photo` by hand. Expected: typing `3` and Enter (or Tab away) applies it; `-` / `+` step by one within 2 to 20 and apply at once; typing `25` or `1` and Enter shows the refusal in the status line and the field goes back to the value in force; Escape in the field commits it and closes; typed digits are not taken as shortcuts.
- [ ] On macOS and Linux, open the settings (`meta+,` on macOS, `ctrl+,` on Linux), Culling page. Expected: `Faces in a group photo` under the auto-advance switch shows 5 on a fresh store; typing `3` and Enter (or Tab away) applies it and, with a folder open, the status line shows `checking faces` only when some frame's `faces_group` is 3 or 4; `-` / `+` step by one within 2 to 20 and apply at once; typing `25` or `1` and Enter shows the refusal in the status line and the field goes back to the value in force; Escape in the field commits it and closes; typed digits are not taken as shortcuts; the value survives a restart.

### App: real-device check of the all-faces meta rows and the strip's closed-eyes mark (all-faces-check Step 5)

`all-faces-check` Step 5 added to the meta pane's `Analysis` group the rows `Similar faces`, `Faces judged`, `Faces with closed eyes` and `Faces with soft eyes`. The three counts are read through the group threshold's visibility rule (`group::shown`) and hidden on a frame under the gate. It also widened the strip cell's closed-eyes mark (`mark::closed_eyes_mark`) to turn on for a closed count of at least 1. Unit tests cover the row builders and the pure rule. No test paints the pane or the cell, and the counts were never compared with a real group photo folder. The step was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261123-all-faces-check/plan.md`. Files: `crates/app/src/meta.rs`, `crates/app/src/mark.rs`, `crates/app/src/main.rs`. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): on `_DSC2914` (`Similar faces` 3) at threshold 3 the `Analysis` group ended with `Faces judged 1`, `Faces with closed eyes 0` and `Faces with soft eyes 1`, and at 4 only `Similar faces 3` showed; that folder held no closed count, and the counts were not judged against the photos.

#### TODO

- [ ] On Windows, open a real group photo folder (for example a team-sport ARW folder) and let the scan finish (`checking faces` gone from the status line). Expected: on a frame whose `Similar faces` is at or above `Faces in a group photo`, `Faces judged`, `Faces with closed eyes` and `Faces with soft eyes` agree with what the photo shows (blinks counted, a soft back row counted), and on a frame whose closed count is at least 1 the amber `eye-off` mark sits on the strip cell and goes when the number is raised past its `Similar faces`.
- [ ] On macOS and Linux, open a real group photo folder and let the scan finish (`checking faces` gone from the status line). Expected: on a frame whose meta pane `Similar faces` is at or above `Faces in a group photo`, the `Analysis` group ends with `Faces judged`, `Faces with closed eyes` and `Faces with soft eyes` that agree with what the photo shows, and the amber `eye-off` mark sits on the strip cell exactly when that closed count is at least 1 or the judged face is closed; on a frame under the number only `Similar faces` shows and no mark comes from the counts; raising the number past a marked frame's `Similar faces` hides its count rows and its counts' mark at once, lowering it back brings them back.

### App: real-device check of the Anyone closed / Anyone soft filter items (all-faces-check Step 6)

`all-faces-check` Step 6 added `Anyone closed` and `Anyone soft` to the filter menu's `Eyes` section (stored and MCP names `anyone_closed` / `anyone_soft`). They pass a frame whose closed (soft) count, read through the group threshold's visibility rule, is at least 1, and are OR-ed with the section's other items. Unit tests cover the predicate, the stored round trip, the menu rows and the `companion.rs` cases. No test drives the items through a real scan, and the judged readings were never compared with a real group photo folder. The step was ticked on the automated criteria only. Plan: `docs/plans/_archived/20261123-all-faces-check/plan.md`. Files: `crates/app/src/filter.rs`, `crates/app/src/menu.rs`, `crates/app/src/main.rs`. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): through the MCP tools, `Anyone soft` kept the 2 frames with a soft count of 1 at threshold 3 and none at 4, `Anyone soft` with `Open` kept 19 at 3 (17 at 4), `Anyone closed` kept none (no closed count in that folder), and the threshold and the `anyone_soft` check survived a restart; the readings were not judged against the photos.

#### TODO

- [ ] On Windows, open a real group photo folder whose frames hold a closed count, let the scan finish (`checking faces` gone from the status line), then check `Filter ▾` > `Eyes` > `Anyone closed`. Expected: the strip narrows to the frames whose meta pane `Faces with closed eyes` is at least 1; judge each `Anyone closed` and `Anyone soft` reading against the photo (a real blink or soft face, or a false reading worth noting).
- [ ] On macOS and Linux, open a real group photo folder, let the scan finish (`checking faces` gone from the status line), then check `Filter ▾` > `Eyes` > `Anyone closed`. Expected: the strip narrows to the frames whose meta pane `Faces with closed eyes` is at least 1 (each a real blink on some face, or a false closed reading worth noting), no frame whose `Similar faces` is under `Faces in a group photo` stays; `Anyone soft` likewise against `Faces with soft eyes`; adding `Open` widens the strip to those frames plus the open-eyed ones; raising the number in the settings past a shown frame's `Similar faces` hides it at once; the checks survive a restart.

### App: real-device checks of the settings layer under the title bar (settings-under-title-bar)

#### Background

The `settings-under-title-bar` feature moved the settings layer below the title bar: the layer's wrapper in `Handpic::render` is `absolute` with `top(TITLE_BAR_HEIGHT)`, `left_0`, `right_0` and `bottom_0`, so the title bar (app icon, "Handpic", the folder's name, the gear, the window drag, the traffic lights / window controls) stays drawn and usable while the settings are open. CI and `mise run ci:local` cover the build and a UI test (`the_settings_layer_lies_under_the_title_bar_and_fills_the_rest`) of the layer's bounds, which also clicks the title bar's folder-name overlay with the settings open and finds them still open. The OS-level window drag, the traffic lights, the window controls, the gear while open (inferred from `open_settings` returning while open, not tested), a shortcut capture under way surviving a title bar press, and a file drop on the title bar (inferred from the hit test, no guard, recorded as Inferred in `docs/agents/app.md`) were never exercised on a device. Plan: `docs/plans/_archived/20261125-settings-under-title-bar/plan.md`.

Files: `crates/app/src/main.rs`, `crates/app/src/settingsview.rs`, `docs/agents/app.md`.

#### TODO

- [ ] On macOS, run `mise run app:dev`, open the settings with `cmd+,`, then check each of these. Expected: all as described.
  1. Drag the window by the title bar: it moves.
  2. The traffic lights no longer overlap the "Settings" heading (the heading starts below the 34 px bar).
  3. Press the gear while the settings are open: nothing happens.
  4. Start a shortcut capture on the Keyboard Shortcuts page, then drag the window by the title bar: the capture state still reads sensibly.
- [ ] On Windows and on Linux, on a device, open the settings with `ctrl+,`. Expect the window controls (minimize / maximize / close) at the title bar's right and the window drag to work while the settings are open.
- [ ] On any platform, drop a file or folder onto the title bar while the settings are open and record whether it opens that folder under the settings (expected from the hit test: it does). Then update `docs/agents/app.md`'s pointer rule from Inferred to Measured.

### App: real-device check of the strip's closed-eyes mark (eyes-mark-tier-mcp)

#### Background

The `eyes-mark-tier-mcp` feature (Step 2) added a closed-eyes mark to the strip cell: Lucide's `eye-off` in amber (`CLOSED_EYES_COLOR`, `0xffb347`), painted by `paint_face_icon` in a fixed slot right of the face focus icon at the image box's bottom-left, and shown when `mark::closed_eyes_mark` finds the judged face's stored eye state closed. The unit tests of `closed_eyes_mark`, the icon path tests (`every_icon_path_loads_an_svg`) and `mise run ci:local` cover the rule and the icon. Nobody has watched the running window: whether the marks agree with the meta pane and the filter on a real folder, whether the mark appears live during pass 2, and its placement at the minimum strip height were never exercised. The Step 2 checkbox was ticked on the automated criteria only. The mark judges one face (nearest the AF point, else the largest confident one); a face under 60 px is not judged. Seen on Windows 11 on 2026-11-27 (results in `docs/plans/_archived/20261127-windows-device-checks-c7/results.md`): on a 24-ARW people folder the one marked cell, `_DSC1926.ARW`, was exactly the file `Filter ▾` > `Eyes` > `Closed` kept and the one `get_view` gave a closed eye state; on a fresh `--index` the marks appeared live while the status row showed `analyzing 14/24`, with no re-listing; at `MIN` the mark stayed clear of the burst badges, three-digit `117/120` to `120/120` included, and dimmed with a rejected cell. Plan: `docs/plans/_archived/20261123-eyes-mark-tier-mcp/plan.md`.

Files: `crates/app/src/main.rs` (`strip_cells`, `paint_face_icon`), `crates/app/src/mark.rs` (`closed_eyes_mark`).

#### TODO

- [ ] On macOS and Linux, when at hand, open a folder of RAW files with people and let pass 2 finish. Expect the strip cells showing the amber `eye-off` mark at the bottom-left to be exactly the files whose meta pane `Eyes` row says closed, and exactly the files the filter menu's `Eyes` > `Closed` keeps. On a fresh `--index`, expect the mark to appear on cells live as pass 2 reaches them (`analyzing N / M`), with no re-listing. At the minimum strip height (`stripheight::MIN`), expect the mark to sit right of the face focus icon without overlapping it or the burst badge (including a three-digit badge), and to be dimmed with a rejected cell. (All three passed on Windows on 2026-11-27.)

### App: no CI job builds the app without `test-support` before `Package`

#### Background

`macos-ci-bundle-reuse` (`docs/plans/_archived/20261127-macos-ci-bundle-reuse/plan.md`) switched the macOS `app` job's bundle step to `--features screenshot` so it reuses the test build. With that, no CI job before `Package` builds the app without GPUI's `test-support`. A test-only GPUI API used in non-test code (the rule under "The main view opens on GPUI's test platform" in `docs/agents/app.md`) therefore surfaces only in the `Package` workflow's release build. Clippy and the tests still cover the default feature set's code. A cheap follow-up is a feature-less `cargo check` on one leg, if the cost of a second GPUI check variant is acceptable. Files: `.github/workflows/ci.yml`, `docs/agents/app.md`.

#### TODO

- [ ] Decide whether a feature-less `cargo check` on one `app` job leg is worth its cost (a second GPUI check variant). If so, add it, and update the rule in `docs/agents/app.md` under "The main view opens on GPUI's test platform" to say which job now catches a test-only API in non-test code.

### Agents: `planner.md` should have a plan name the input kind and material of each hands-on Done-when line

#### Background

The agent-run-device-checks work (plan: `docs/plans/_archived/20261127-agent-run-device-checks/plan.md`) made the develop wrap-up run the manual checks this host can run, and file only the rest. `planner.md` was left unchanged, so a plan may still write "Manually verified on Windows" as a Done-when line, with no word on whether an agent can run it or what material it needs. `implementer.md` step 6 now asks a pending check to be recorded with its platform, input kind and material. Files: `.claude/agents/planner.md`, `.claude/agents/implementer.md`, `.claude/skills/develop/SKILL.md` (3.3).

#### TODO

- [ ] Make `planner.md` ask a plan to name, for each hands-on Done-when line, the input kind (agent-runnable or user-only) and the material, consistent with the wrap-up's reason list in `SKILL.md` 3.3 and `implementer.md` step 6.

### App: the eyes supersede rule is stale in `CLAUDE.md` and in the `eyes.rs` comments

#### Background

`CLAUDE.md`'s `src/eyes.rs` description says "so a newer request or a clear supersedes the running judgment", and the doc comments of `EyesTicket` and `EyesCache::current` in `crates/app/src/eyes.rs` say the same. `EyesCache::request` issues no ticket while a judgment is in flight, so only a `clear` supersedes a running one. Seen on Windows 11 on 2026-11-27: 60 `navigate next` 8 ms apart logged three `eyes` lines (results in `docs/plans/_archived/20261127-windows-device-checks-mcp/results.md`). The section "An on-demand judgment is superseded through a shared atomic" of `docs/agents/app.md` was already corrected; `CLAUDE.md` and the code comments were not, because the plan changed docs and todo only. Files: `CLAUDE.md`, `crates/app/src/eyes.rs`.

#### TODO

- [ ] Reword the `src/eyes.rs` description in `CLAUDE.md` and the doc comments of `EyesTicket` and `EyesCache::current` so that while a judgment runs only a `clear` supersedes it, and a newer request's id only follows a settle or a clear.

### Agents: the develop README still describes `NEEDS_APPROVAL` as a stopping point

#### Background

`.claude/skills/develop/README.md` still describes `merger`'s `NEEDS_APPROVAL` as a point where develop stops for the user. It does so in the step sequence diagram, in "Folding into one PR" risk 3 and in the "Operational note: using it with `/goal`" section. `.claude/skills/develop/SKILL.md` § Merge merges with a standing approval, except for the release PR and the release-shaping paths. The develop-decide-by-recommendation work (plan: `docs/plans/_archived/20261128-develop-decide-by-recommendation/plan.md`) reworded only the "single point of dialogue" sentence and left the rest. Files: `.claude/skills/develop/README.md`, `.claude/skills/develop/SKILL.md`.

#### TODO

- [ ] Bring the three README places in line with SKILL.md § Merge (merge with a standing approval, stop only for the release PR and the release-shaping paths).

### Agents: confirm the develop session end through herdr `/exit` on a real run (develop-decide-by-recommendation)

#### Background

The develop-decide-by-recommendation work (plan: `docs/plans/_archived/20261128-develop-decide-by-recommendation/plan.md`) made the develop main agent end its Claude Code session after the final report when nothing else is pending. Inside herdr (`HERDR_ENV=1`) it makes `sleep 5; herdr pane send-text "$HERDR_PANE_ID" "/exit"; herdr pane send-keys "$HERDR_PANE_ID" enter` its last tool call, as a background, unsandboxed Bash call, so `/exit` lands once the turn is idle. The `herdr pane send-text` and `herdr pane send-keys` commands were confirmed to exist with `herdr pane --help`. Nothing exercised the call on a real run. The open risk is the timing: the background command's exit re-invokes the main agent close to the moment `/exit` is submitted. Files: `.claude/skills/develop/SKILL.md` (§ Finishing the run).

#### TODO

- [ ] On Windows, in herdr, let one real develop run reach the final `MERGED` with nothing else pending. Check that the "Decisions taken without asking" list is written, that the background `/exit` call is the last tool call, and that `/exit` lands once the turn is idle and the session closes with the worktree left for `wrangle`'s reap. Note whether the command's completion re-invokes the agent before `/exit` is submitted. Done when one real develop run in herdr ends with /exit submitted and the session closed, and SKILL.md § Finishing the run is corrected if it was not. Not runnable here: it needs a real develop run inside herdr to its final `MERGED`, and whether the session closed is the user's to observe.

### Core: non-faces among pass 3's candidates on AF frames (candidates-floor)

#### Background

The candidates-floor plan lowered `allfaces::candidates`' floor to `GATE_LOW_RES_CONFIDENCE` (0.6). At that floor the 640x448 whole-image search of an AF frame counts a jersey number, a ball, a hand or an ear as a face: 6 of the 13 added faces on the first 300 ARW of `2026-11-07`, plus one face behind both hands. They read open, so no closed count moved, but they raise `Faces judged` and, for three of them, the soft count (`Anyone soft`). All were on frames under the default gate of 5, so they show only under a lowered `Faces in a group photo`. Measurement: `docs/plans/_archived/20261128-candidates-floor/measurement.md`; plan: `docs/plans/_archived/20261128-candidates-floor/plan.md`.

Files: `crates/core/src/allfaces.rs` (`candidates`), `crates/core/src/scan.rs` (`other_faces`).

#### TODO

- [ ] Decide between a size band relative to the judged face (as the gate has) and a higher floor on the AF path, to drop these non-faces. Either changes the counts, so it needs an `ALLFACES_VERSION` bump. Measure it on the same `2026-11-07` set first.

### Core / CLI: `allfaces` answers unknown on a frame with no judged face but a 0.6 to 0.8 face (candidates-floor)

#### Background

After the candidates-floor change, `whole_image_cue_rgb` gives no RGB without a judged face, and `other_faces` answers unknown when a candidate needs pixels. 14 frames of `2026-02-01` went from `0 0 0` to `- - -` in `handpic-cli allfaces`. The gate count of such a frame is 0, so the app never runs pass 3 on it; only the CLI shows it. Plan: `docs/plans/_archived/20261128-candidates-floor/plan.md`.

Files: `crates/core/src/scan.rs` (`other_faces`, `whole_image_cue_rgb`).

#### TODO

- [ ] Decide whether the CLI's `allfaces` should answer `0 0 0` again on a frame with no judged face (for example by not treating a candidate as needing pixels there), or whether `- - -` is the intended answer, and document it.

### Core: the time cost of pass 3's 0.6 floor is unmeasured on an idle host (candidates-floor)

#### Background

The candidates-floor measurement ran each build once on a shared host. The "before" means were 3 to 4 times the documented ones and single files moved by hundreds of ms either way, so no time delta could be quoted. The cost is estimated from the added mesh count (3.0 per gate frame on the Leica groups, 0.46 on the team sport) at about 44 ms per mesh from the stage profile. Plan: `docs/plans/_archived/20261128-candidates-floor/plan.md`.

Files: `docs/humans/performance.md`, `docs/humans/performance.ja.md`.

#### TODO

- [ ] On an idle host, with the user's go-ahead for repeated runs, alternate the before and after builds as `docs/humans/performance.md` does (`handpic-cli allfaces <files> 1`, fastest of three) on `2026-02-01` and the first 300 ARW of `2026-11-07`, and add the measured per-file times to the "All-faces check (Windows 11)" section of both languages.
