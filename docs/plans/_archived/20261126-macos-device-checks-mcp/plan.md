<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.
A plan folder moves under `_archived/` when it completes, so link another plan only at its archived path (`../_archived/<folder>/...`) or link a stable non-plan path, and name a plan still in flight as a backticked path. After rebasing onto a main where plan folders moved, re-run the link check (`mise run ci:local` runs lychee when the branch changed Markdown; `mise run lint` always does).

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Record the macOS real-device checks run through MCP and the shell (2026-11-26)

## Purpose

A real-device run on macOS (Darwin 27.2.0, arm64) on 2026-11-26 exercised
eleven pending macOS checks of `todo.md` (the file is at the repository root)
through the app's MCP tools and the shell. Five passed whole, one passed in
its first half, five passed in part, and nothing failed. This plan records the
run in the repository: `results.md` in this folder, the passed boxes deleted
from `todo.md`, the partial ones narrowed to what was not run, and two
sentences of `docs/agents/` brought up to date. Docs only, no code change: one
step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261123-windows-device-checks-mcp-visual/plan.md`
(#1262).

The run is already done; nothing is run again. Its notes are the session
scratchpad's `results-notes.md`, the source of truth for every edit. The
`todo.md:NNN` numbers in the notes are located by item heading here, never by
line number.

## Steps

- [x] Step 1: Write `results.md` from the run's notes and apply it to `todo.md`, `docs/agents/app.md` and `docs/agents/shared-crates.md`
  - Done when:
    - `results.md` in this folder holds every measurement and "not done" of the notes, in English, with each section named by its `todo.md` item heading (not `todo.md:NNN`).
    - Every edit of lists A to D is applied.
    - `git diff origin/main --stat` shows only `todo.md`, `docs/agents/app.md`, `docs/agents/shared-crates.md` and this plan's files; in `todo.md` only the items of list B changed; in the two `docs/agents/` files only the sentences of list C changed.
    - No two `###` items in `todo.md` are left without a blank line between them, and the count of doubled blank lines in each changed file equals `origin/main`'s. Every changed file has LF line endings.
    - The PR description records what was seen for each removed or narrowed box.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **No run.** The checks ran on 2026-11-26 at `020e89c7` (main). Source: `/private/tmp/claude-501/-Users-mino--herdr-worktrees-handpic-worktree-silver-river-9d87/f831d8fc-15af-4a3d-9b70-2674624d3e7b/scratchpad/results-notes.md`.
    - **Editing.**
      - `todo.md` is edited concurrently by other sessions. Rebase on `origin/main` right before editing and again before the PR; touch only the items named below.
      - Locate every item by its exact `###` line, never by line number. "Box N" counts the item's top-level `- [ ] ` lines from 1.
      - Keep every other heading, item, box and sentence byte-for-byte; no item heading is renamed.
      - `todo.md` paragraphs and boxes are single long lines; wrap text added to `docs/agents/*.md` at the file's width, as its neighbours.
      - `RESULTS` below stands for `docs/plans/_archived/20261126-macos-device-checks-mcp/results.md`, always written in backticks, not as a link (the folder is archived by this PR's wrap-up; a link would not resolve before that).
      - Each background sentence of list B starts `Seen on macOS on 2026-11-26 (results in \`RESULTS\`):` and goes where the item says.
      - Suggested PR title: `docs: record the 2026-11-26 macOS real-device checks run through MCP`.
    - **A. `results.md`**, shaped like `../_archived/20261123-windows-device-checks-mcp-visual/results.md`: a short intro, `## Setup`, the checks, `## Side findings`.
      - Intro: the checks were driven only by MCP tools and the shell: no key press, no right-click, no drag, no dialog button. List the tools the notes name.
      - Setup: Darwin 27.2.0, arm64, a 5120x1440 display, Dock autohide on. The debug app through `mise run app:dev:screenshot`, the release through `mise run app:package:macos` (version 2.1.0, so `Handpic-2.1.0-macos-arm64.dmg` / `.tar.gz`). Every launch with its own `--settings`, `--index`, `--log` and `--mcp-port 41999`, the store seeded with `mcpEnabled`, `mcpAllowDestructive` and `sidecarFormat` both. The test folder: APFS clones of a 117-ARW Sony ILCE-7M5 folder (19 `.dop`, 4 `.xmp`, bursts at 0.1 s).
      - One section per check, headed by the `todo.md` item's name and PASS / PARTIAL, carrying every figure of the notes and each "Not done" list. For the QoS check these are: 1755 ARW on a release binary with a cold index; pass 2 in 104026 ms with `threads=1`; pass 3 of 15 files in 2209 ms; the `keypressToPixels` p50 / p95 / max during pass 1, during pass 2 and after the scan; the `source=read` / `source=prefetch` split; no `ran at normal priority` line; the debug build's about 5 s per file.
      - The Windows reference figure (`18.4 / 25.0` during pass 2) is copied as the notes give it. Its source was not found in `docs/agents/app.md` while planning: name the source if it can be found, else mark it "as recorded in the run's notes".
    - **B. `todo.md`: one item removed whole, seven narrowed.**
      - `### Core: real-device check of the scan priority's macOS QoS branch`: its one box passed, so remove the whole item (heading, background, `#### TODO`, the box), leaving one blank line between its neighbours.
      - `### App: the macOS / Linux manual checks for Move to Trash are still open`:
        - Background sentence, inserted immediately before ` Files: \`crates/engine/src/trash.rs\``: through MCP `trash_photos` with `mcpAllowDestructive` on, the preview read `Move 1 file to the Trash?` / `Move to Trash`. A confirmed run moved the RAW, its `.xmp` and its `.ARW.dop` to `~/.Trash` and the strip showed the neighbouring file. `undo` restored all three with the reject flag and the 2 stars kept in the index and both sidecars. With a file of the same name placed at the original location, the undo reported `already exists at the original location`, left the two sidecars in the Trash and did not touch the placed file. The right-click path, the dialog's `Cancel` and Finder's "Put Back" were not run.
        - Box 1 becomes: `- [ ] On macOS, right-click a strip cell and pick \`Move to Trash…\`: the dialog opens with \`Move to Trash\` / \`Cancel\`; Cancel leaves the folder untouched; after a confirmed move, a "Put Back" from the Trash restores the file with its judgment, and if no "Put Back" entry exists, dragging it out does. (On 2026-11-26 the dialog's text, the move of the RAW with its \`.xmp\` and \`.ARW.dop\` and the neighbour shown were seen through MCP \`trash_photos\`, with no dialog and no right-click.)`
        - Box 2 becomes: `- [ ] Verify the whole flow on Linux: the dialog asks \`Move 1 file to the Trash?\`, Cancel leaves the folder untouched, confirming moves the RAW plus its \`.xmp\` and \`.ARW.dop\` to the Trash and the strip shows the neighbouring file, and a restore from the Trash brings the file back with its judgment (the \`trash\` crate's freedesktop backend has never been run here).` Its "the same flow" would otherwise point at the narrowed box 1.
        - Delete box 3 (the undo with the reject flag).
      - `### App: real-device checks of the burst bands, badges, burst keys and reject the rest`:
        - Background sentence, appended to the end of the second background paragraph (the one starting `On Windows 11 on 2026-11-22`):
          - Band and badges: on a Sony folder with bursts at 0.1 s the band drew behind each burst with rounded ends, the `n/size` badges at the bottom-right, dimmed on a rejected member; one burst ended at `16/16` and the next started at `1/3`; the selected cells' outline was distinct from the band; a JPEG folder looked the same.
          - Moves: MCP `navigate`'s burst and burst-frame moves were right in name order, under a stars filter with a rating sort, and in the JPEG folder.
          - `reject_rest`: on a 16-frame burst with one member already rejected and one hidden by the filter, it rejected the 14 others (the `.xmp` and the `.dop` of each, stars and label kept), with the current file kept and nothing advancing.
          - Undo and the rest: `undo` / `redo` reported `Undid 14 files` / `Redid 14 files`; a two-frame burst's undo moved to the other frame; a singleton and a JPEG folder did nothing; `get_view`'s `burst` listed the members with `visible: false` on the hidden one.
          - Not run: no key was pressed (the tools and the keys share the view's `perform`); a Leica whole-second folder, a failed cell next to a band, and the keys while the tree holds the keyboard.
        - Box 1 becomes: `- [ ] On macOS, open a Leica whole-second burst folder in \`handpic\`: expect its bursts banded and badged as a Sony folder's. Expect a band to be distinguishable from a failed cell next to it, and \`arrowup\` / \`arrowdown\` and \`alt+arrowleft\` / \`alt+arrowright\` to do nothing while the folder tree has the keyboard. (The rest of this check passed on 2026-11-26 through MCP.)`
        - Delete box 2 (rejectRest).
      - `### App: real-device checks of the app icon on Windows, macOS and Linux`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261114-native-app-icon/plan.md\``: after `mise run app:package:macos` (which runs `app:bundle:macos`) and a launch of the bundle, `NSRunningApplication.icon` (what the Dock draws) and `NSWorkspace`'s `iconForFile` (Finder) both gave the Handpic icon; the Dock was hidden and not captured.
        - Delete box 2 (macOS).
        - Box 4 becomes: `- [ ] Once the Windows and Linux checks above are done (or a failure is filed as its own issue), update the "Pending manual checks" bullet under \`## Packaging\` in \`docs/agents/app.md\`, which names them since the macOS check passed on 2026-11-26. Done when the bullet matches what is still pending, or is removed when nothing is.`
      - `### App: real-device check of the folder tree's file counts on collapsed rows (native-tree-index-counts, native-folder-count-cache, native-folder-count-prune)`:
        - Background sentence, inserted immediately before ` Plans: \``:
          - Expanded then collapsed never-opened folders, one through a symlink, showed their counts and kept them after a restart; the opened folder too; a never-listed folder showed none.
          - An expanded row's pill looked the same as a collapsed one's.
          - A RAW added outside the app followed a `Refresh`, and the new count showed after a restart.
          - A folder renamed in the app kept its count after a restart.
          - A folder deleted outside and re-created under the old name showed no count until listed.
          - A long name truncated with the pill whole.
          - A `Rename by Capture Date…` split showed each new date folder's count while collapsed, the emptied original removed.
          - Not run: an index upgraded from before the cache table, a network volume, a folder renamed outside the app, and a split whose undated files keep the original.
        - Box 2 (macOS) becomes: `- [ ] On macOS (\`mise run app:dev\`), launch the app on an index upgraded from before the cache table: expect no collapsed row to show a count yet. Expand a folder reached through a network volume, restart: expect its count. Rename a counted folder outside the app, expand or \`Refresh\` its parent, quit and restart: expect no count on the new name until it is listed. Run \`Rename by Capture Date…\` with a split that leaves undated files in the original folder: it shows no count until listed. (The rest of this check passed on 2026-11-26.)`
        - Box 3 (Linux) becomes self-contained: copy the macOS box's text as it is on `origin/main` (before this edit), with `On Linux` for `On macOS` and `a symlink or a mounted network folder` for `a symlink or a network volume`, dropping its "the same steps as on macOS".
      - `### App: real-device checks of the gpui-kit umbrella crate (native-gpui-kit)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261116-native-gpui-kit/plan.md\``: the debug window drew its text (the title bar, the tree, the no-folder message, the status row, the meta pane, the `Sort ▾` / `Filter ▾` buttons) with no shader error on stderr or in the log; the popover menus were not opened.
        - Delete box 2 (macOS). Boxes 1 and 3 stay.
      - `### App: real-device checks of the Appearance tab (mode, shipped themes) (native-themes)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/_archived/20261115-native-themes/plan.md\``:
          - Set through MCP `set_settings`, Light (Neutral) turned the panes, the strip, the title bar and the meta pane light and readable, the stars and the burst badges too.
          - In System mode with Catppuccin Latte and Tokyo Night chosen, switching the OS appearance swapped the app between them live, both ways.
          - Dark stayed Tokyo Night with the OS in light, and a restart restored the theme on the first frame.
          - Not run: the dialogs and the settings box under each theme (no MCP tool opens the settings), and the pickers' keys.
        - Box 3 (macOS) becomes: `- [ ] On macOS, open the settings (\`meta+,\`), Appearance: expect the dialogs and the settings box to turn light under Light and to take a shipped dark and a shipped light theme, staying readable. Pickers' keys: Space / Enter open the menu, the arrows move, Enter picks, Escape with the menu open closes the menu only, with it closed closes the settings. (On 2026-11-26 Light, System following the OS with two shipped themes, Dark and the restart were seen, set through MCP.)`
        - Boxes 1, 2 and 4 stay (box 4, Linux, refers to the Windows boxes, which are unchanged).
      - `### App: real-device checks of the packages the Package workflow builds (gpui-packaging)`:
        - Background sentence, inserted in the first background paragraph immediately before ` The native-release Step 5 install docs`:
          - `mise run app:package:macos` built `Handpic-2.1.0-macos-arm64.dmg` (the name carries the crate's version, not `0.0.0`); it opened and the copied app started.
          - Gatekeeper's first-launch prompt cannot be reproduced with a local build, which carries no `com.apple.quarantine` attribute (only `com.apple.provenance`).
          - `spctl --assess` rejects the ad hoc signed app.
        - Box 1 becomes: `- [ ] On macOS, with a \`Handpic-<version>-macos-<arch>.dmg\` that carries the quarantine attribute (a downloaded one; a local \`mise run app:package:macos\` build has none), drag \`Handpic.app\` to \`Applications\` and start it. Gatekeeper asks for the ad hoc signed app's first launch to be allowed under System Settings, Privacy & Security. Expect the app to start. (The local build, the DMG and the launch passed on 2026-11-26.)`
        - Boxes 2 to 5 stay, their `0.0.0` names too.
      - `### App: real-device checks of the capital executable name (\`Handpic.exe\`, \`Contents/MacOS/Handpic\`)`:
        - Background sentence, inserted immediately before ` Plan: \`docs/plans/20261123-capital-exe-name/plan.md\`` (that existing path is left as it is):
          - The DMG of `mise run app:package:macos` passed `hdiutil verify`.
          - Its `Handpic.app`, copied to `~/Applications`, held `Contents/MacOS/Handpic`, passed `codesign --verify --strict` (ad hoc, identifier `dev.minodisk.handpic`) and launched.
          - The update from a 2.0.0 bundle was not run (it needs a test feed).
        - Box 4 (macOS) becomes: `- [ ] On macOS, a 2.0.0 \`Handpic.app\` in \`/Applications\` updates to the new bundle from a test feed (\`tools/package/test-release.sh\`) and restarts into it. (The DMG's launch, \`Contents/MacOS/Handpic\` and \`codesign --verify --strict\` passed on 2026-11-26.)`
    - **C. `docs/agents/`: two sentences, nothing else; no section heading's tag changes.**
      - `docs/agents/app.md`, under `## Packaging`, the bullet starting `- Pending manual checks, not done here: the Dock / Finder icon`: replace it with `- Pending manual checks, not done here: the taskbar, title bar and Explorer icon on Windows, and the running window's icon after the install task on a Wayland and an X11 session. The Dock / Finder icon of the built \`.app\` was seen on macOS on 2026-11-26 (\`RESULTS\`).`
      - `docs/agents/shared-crates.md`, section `### The scan pool's priority levels, and the hot list a stale scan drops (Inferred)`, the bullet starting `- Why not \`QOS_CLASS_BACKGROUND\` for the analysis pass on macOS either`: replace its last sentence `The paging latency this gives on a real Mac is not measured yet.` with: `Measured on a Mac (arm64) on 2026-11-26, release build, cold index, 1755 ARW: paging every 0.5 s during pass 2 gave \`keypressToPixels\` p50 16.3 / p95 35.5 ms against 12.2 / 19.5 ms after the scan, pass 2 took 104 s, and no \`ran at normal priority\` line was logged (\`RESULTS\`).` The same bullet's `docs/plans/20261119-uitest-macos-timeout/` path is left as it is.
    - **D. `learnings.md`**: this step's notes.
      - What MCP plus the shell closed on macOS, and what it cannot: a key press, a right-click, a dialog button, the settings box, Finder's "Put Back", Gatekeeper on an unquarantined local build.
      - The setup worth keeping: per-launch `--settings` / `--index` / `--log` / `--mcp-port`; the bundle's executable run directly to get stdout; a debug build is unusable for the QoS check, at about 5 s per file in pass 2.
      - The side finding: the `screenshot` tool returns the last drawn frame, and a window fully covered by other windows does not draw, so bring the window to the front before a capture. No `todo.md` item.
      - Every page during the passes was `source=read`, every page after `source=prefetch`.
      - The package's name carries the crate's version (2.1.0), not the `0.0.0` the `todo.md` boxes name.

## Trade-offs and risks

- **Tools, not keys.** The run pressed no key: `reject_rest`, `undo`, `redo` and `navigate` went through MCP. The keymap action and its tool both run through the view's `perform`, and the keys themselves were pressed on Windows on 2026-11-22, so this plan deletes the rejectRest box and drops the burst keys from the narrowed burst box (approved by the user on 2026-11-26).
- **Sub-points the notes do not name** (burst band "joined across the gaps" and distinct from the current cell's outline; the themes' tree cursor ring, menus' hover, progress bar, pick green, preview surround, derived `Roles`, Neutral again; the gpui-kit box's popover menus' text; the trash dialog's text seen as the MCP preview's) are closed on the requester's list of what remains.
- **The Gatekeeper box duplicates box 4 of the same item.** Kept narrowed, as asked.
- **The icon item's follow-up box stays**, reworded; the `docs/agents/app.md` bullet is updated now.
- **The Dock icon was read through `NSRunningApplication.icon`, not seen** (the Dock was hidden).
- **The QoS item is removed whole**; `docs/humans/performance.md` is left alone. The `shared-crates.md` section keeps `(Inferred)`.
- **The Windows reference `18.4 / 25.0`** stays in `results.md` only.
- **The Linux boxes of the trash and tree-count items are reworded though not run**, because they pointed at macOS boxes this plan narrows.
- **The screenshot side finding gets no `todo.md` item.**
- `todo.md` is edited by other sessions; an item reworded on `main` between planning (`020e89c7`) and the edit must be re-read, not overwritten.

## Progress

- 2026-11-26: Step 1 done (results.md, learnings.md, todo.md, two docs/agents sentences)
- (2026-11-26) Step 1 complete
