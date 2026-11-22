<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../humans/usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Record the Windows real-device checks of ORF folders and big-endian DNGs (2026-11-22)

## Purpose

A real-device run on Windows 11 on 2026-11-22 exercised two pending manual
checks of `todo.md`: an ORF folder in the strip, the sidecars and Move to
Trash, and the big-endian DNGs after the `EXTRACTOR_VERSION` bump (on an index
a real pre-bump build wrote at `EXTRACTOR_VERSION` 11). Every sub-point of
both passed, and the run found two things outside the boxes. This plan records
the run, removes the two items, and adds the two findings as new items. Docs
and todo only, no code change: one step, one PR (single-PR mode). It follows
`docs/plans/_archived/20261122-windows-device-checks-6/plan.md`.

Unlike that plan, this one is written after the run. The results already
exist; Step 1 copies them into this folder as `results.md`. They are the
source of truth, and lists A to D below name the exact edits.

## Steps

- [x] Step 1: Copy the run's results into `results.md`, apply them to `todo.md`, and write `learnings.md`
  - Done when:
    - `results.md` in this folder is the run's results file, content unchanged, with LF line endings.
    - Every edit of lists A and B is applied; list C (no `docs/agents/app.md` change) holds; `learnings.md` holds list D.
    - `git diff origin/main --stat` shows only `todo.md` and this plan's files; no `todo.md` item outside lists A and B changed.
    - `grep -c '^### ' todo.md` is still 136 (two items out, two in; the 138 first written here was a miscount), every `###` heading has a blank line before it (but the first, directly under `## Cross-cutting / other` as on `origin/main`), and the file has no doubled blank line that `origin/main` does not have.
    - The PR description records what was seen for each sub-point of the two removed items, since `todo.md` no longer holds them, and says that the run opened copies under `D:\Photos\tests\20261122-orf-dng\`, not `D:\photos\samples\ORF\` or `D:\Photos\samples\DNG\` themselves.
    - `mise run ci:local` passes locally and the PR's GitHub checks pass.
  - Implementation approach:
    - **Results.** The run's results are in the session scratchpad's `results.md` (the caller gives the path). Copy it to this folder as `results.md` without rewording, CRLF converted to LF. Its backticked paths are not links, so lychee does not follow them.
    - **Editing.**
      - `todo.md` is at the repository root. Locate each item by its `###` heading, never by line number. An item runs from its heading to the line before the next `###` heading.
      - Keep every other heading, the neighbouring items and every sentence this plan does not name, byte-for-byte.
      - Write a Python script to the scratchpad with the Write tool and run it with `python -I`, asserting one match per `###` anchor.
      - Each new item takes the place of a deleted one, so the blank line before and after stays as it is.
      - Each new item has the shape of the two items batch 6 added (`### App: the 1:1 view of a file whose crop read fails is a plain black canvas`): `#### Background`, a blank line, the paragraph, the `Files:` line directly under it, a blank line, `#### TODO`, a blank line, the box.
    - Suggested PR title: `docs: record the 2026-11-22 Windows real-device checks of ORF folders and big-endian DNGs`.
    - **A. Two items deleted whole.**
      - `### App: real-device check of an ORF folder in the strip, sidecars and Move to Trash` (one box, five sub-points, all PASS in `results.md`, "An ORF folder in the strip, sidecars and Move to Trash"):
        - (1) all 167 ORFs listed, 165 with a thumbnail, the tree row 167. The two without, `Unknown_image.orf` and `Unknown_image_2.orf`, read `no embedded preview`: files without a preview JPEG, the failure `docs/agents/raw-metadata-parsing.md` documents for ORFs that hold only the 160x120 thumbnail, and the same two failed the same way in the 2026-10-02 `check` run (`docs/plans/_archived/20261002-big-endian-dng/check-after.txt`). They do not keep the sub-point open and get no new item.
        - (2) the 3200x2400 preview and the 1:1 view upright on the orientation 8 file the box names.
        - (3) the meta pane's EXIF rows, no make or model ending in a space.
        - (4) the XMP and the `.dop` written with the sidecar format set to both.
        - (5) Move to Trash moving the ORF and both sidecars, Undo restoring all three.
      - `### App: real-device check that big-endian DNGs open after the EXTRACTOR_VERSION bump` (one box, PASS in `results.md`, "The pre-bump build" and "Big-endian DNGs after the EXTRACTOR_VERSION bump"):
        - The index was written by a build of commit `425667a2` (`EXTRACTOR_VERSION` 11): 8 rows caching `not a little-endian TIFF/ARW`.
        - The current build re-extracted all 8 to `extractor` 13; 7 gained a thumbnail and metadata.
        - The thumbnails, the fit view and the 1:1 view showed for the K-5 II, the iPhone 12 Pro / Pro Max and the Leica M (Typ 240).
        - The eighth file, `iPhone_12_Pro_Max_thing.dng`, is a JPEG named `.dng` and now reads `invalid TIFF byte order`, which `docs/plans/_archived/20261002-big-endian-dng/learnings.md` already records. No item for it.
    - **B. Two new items** (no existing `todo.md` item covers either).
      - In the place of the ORF item:

        ```markdown
        ### Core: the lens name of some ORFs keeps a trailing space

        #### Background

        Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-orf-dng/results.md`): after a scan of 167 ORF samples, the index's `lens` of `E-M5MarkII__1230049.ORF` and `E-PM2_P1010001.ORF` is `LEICA DG SUMMILUX 25/F1.4 `, with a trailing space. No `make` or `model` of the folder ends in a space: `read_ifd0` in `crates/core/src/exif.rs` trims both (`trim_end`), while `read_exif_ifd` in the same file, which the ORF, NEF, CR3 and JPEG / RAF readers share, takes the lens as stored. `crates/core/src/arw.rs`'s own reader (ARW, DNG) trims only NULs from all three, and `crates/engine/src/exif.rs` drops a lens only when it is all blank. The meta pane's `Lens` row and the filter menu's `Lens` section take the string from the index; whether the padded name shows up as its own filter entry next to the same lens without the space was not checked.
        Files: `crates/core/src/exif.rs` (`read_exif_ifd`, `read_ifd0`), `crates/core/src/arw.rs` (`exif`), `crates/engine/src/exif.rs` (the `lens` of `Exif`), `crates/engine/src/index.rs` (`EXTRACTOR_VERSION`).

        #### TODO

        - [ ] Trim the lens name's trailing spaces as the make and the model are, with a test: either where it is read (then decide whether `arw.rs`'s reader gets the same, and bump `EXTRACTOR_VERSION` so indexed rows are re-extracted) or where `crates/engine/src/exif.rs` builds the displayed lens (no re-extraction).
        ```

      - In the place of the big-endian DNG item:

        ```markdown
        ### Core: the Leica M (Typ 240) DNGs show only their 160x120 JPEG in the fit view and the 1:1 view

        #### Background

        Seen on Windows 11 on 2026-11-22 (results in `docs/plans/_archived/20261122-windows-device-checks-orf-dng/results.md`): the three Leica M (Typ 240) DNG samples open, but the only JPEG `arw::parse` finds in them is 160x120 (`page` lines `jpegKB=9..13 px=160x120`). The fit view is that image upscaled, visibly blocky, and the 1:1 view shows it as a 160x120 picture in the middle of the canvas; the orientation is right in both. The app says nowhere that the preview is small. `arw::parse` looks only at IFDs with a JPEG offset and length (0x0201 / 0x0202) and at single-strip YCbCr `Compression=7` IFDs, so a preview stored another way (uncompressed, or in the MakerNote) would be missed; whether these files hold one was not checked. An ORF that holds only a 160x120 thumbnail is refused with `no embedded preview` instead (`docs/agents/raw-metadata-parsing.md`). What pass 2 reads from a 160x120 preview (sharpness, faces) was not looked at.
        Files: `crates/core/src/arw.rs` (`parse`, `strip_jpeg`, `PREVIEW_MIN_WIDTH`), `crates/app/src/zoom.rs`.

        #### TODO

        - [ ] Check with exiftool whether the Leica M (Typ 240) DNGs carry a preview larger than 160x120 that `arw::parse` misses, and read it if so. If they do not, decide what the app does with a thumbnail-only DNG: show it as now, say that the preview is small, or refuse it as the ORF reader does.
        ```

    - **C. `docs/agents/app.md`**: nothing added. The run's reusable lessons are already rules there (the MCP test-run rule; Git Bash mangling `\\` in a JSON argument, the client expanding placeholders). Making the old index needed the removed Tauri app at one commit, which takes no `--index`; a later `EXTRACTOR_VERSION` check builds a GPUI parent commit and passes `--index`, so the identifier change and the first-run dialog do not recur. The note goes to `learnings.md` (list D).
    - **D. `learnings.md`**: this step's notes.
      - The pre-bump tree (`425667a2`) is the Tauri app. It needed only `tauri.conf.json`'s `identifier` changed to keep its settings, window state and index apart from real data, and its first-run sidecar dialog answered.
      - Seeding `lastFolder` made the old build open the test folder without a click.
      - Git Bash mangled `\\` in the MCP JSON argument even inside single quotes; the client expanded placeholders instead (already a rule in `docs/agents/app.md`).
      - `set_view` `zoom` over MCP entered the 1:1 view, so those captures needed no synthetic input.
      - Why `docs/agents/app.md` got nothing (list C).
      - A "Deferred issues (todo candidates)" section naming the two items of list B, each with its basis in `results.md` and its files, as batch 6's `learnings.md` does.

## Trade-offs and risks

- **ORF sub-point (1), "every `.ORF` appears in the strip with a thumbnail".** Two of the 167 show `no embedded preview`. The plan deletes the box: the two are listed, carry a plain reason, and are files without a preview JPEG, a documented failure whose todo was dropped on purpose (`docs/plans/_archived/20260930-drop-olymp-compacts-todo/plan.md`). Read literally, the sub-point did not hold for them; the alternative is to keep a box narrowed to those two files, which would track a case the project already decided not to support. The PR description names the two files either way.
- **The boxes name `D:\photos\samples\ORF\` and `D:\Photos\samples\DNG\`; the run opened copies** under `D:\Photos\tests\20261122-orf-dng\`, since the app only opens test copies. The content is the same, so the boxes close; the PR description records it.
- **`docs/agents/app.md` gets no bullet.** The alternative is one bullet in the real-device rules ("to make an index of an older `EXTRACTOR_VERSION`, build the parent commit in a temporary worktree with its data kept apart"). Not taken: the awkward parts were specific to the removed Tauri app, and with the GPUI app it is `--index` on a parent-commit build. The caller can add it in the wrap-up if it wants the rule on record.
- **The lens item leaves the fix's place open.** A trim in `crates/core` changes stored values and needs an `EXTRACTOR_VERSION` bump, which re-extracts every indexed folder once; a trim where the displayed lens is built needs none but leaves the index value padded. The item's implementer decides.
- **The Leica finding is one item, not two.** The parser check comes first, and the "say the preview is small" question only exists if no larger preview is found. Splitting it into a parser item and an app item is the alternative. Its heading is `Core:` for the same reason.
- **Placement.** Each new item takes the place of the item it came from, as batch 6 did, which keeps the neighbours and the blank lines untouched. The alternative is next to `### Core: widen camera support from public sample RAW files` (the Leica item) and the EXIF filter items (the lens item).
- **`results.md` is copied unchanged**, including its title's "batch 8" and the scratchpad wording; `todo.md` names it by its archived path, which exists only after the wrap-up's archive move (the same PR in single-PR mode).

## Progress
