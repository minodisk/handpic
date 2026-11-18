<plan-guide>
When you start executing this plan, record the in-flight plan path in auto memory (`MEMORY.md`).
After every step's PR is merged, the `develop` skill's wrap-up phase (normally a chore PR, or the same PR as the implementation in single-PR mode) does the archive move and removes the auto memory entry.

Update this file as you go if problems or design changes come up.
Record additional important information (investigation results, design details) in separate files in this folder.
Record what you learn during implementation, and notable events (CI failures, review feedback, plan changes, user intervention), in `learnings.md` as they happen.
After finishing a step, continue to the next without asking the user.
lychee (`mise run lint`) resolves a relative link in `docs/plans/**` from the linking file's own directory, so write links relative to the plan folder (e.g. `../../usage.md`) and put an example path that is not a real link target in backticks.

<pr-rules>
- Include the plan.md update (marking the step done + the Progress entry) in the implementation PR
- Include `Plan: [path]` and `Step: [number]` in the PR description
</pr-rules>
</plan-guide>

# Split the PhotoLab guide out of `tauri-app.md` and record the Uuid-less `.dop` measurement

## Purpose

`docs/agents/tauri-app.md` has grown four sections that are about DxO
PhotoLab and its `.dop` sidecar rather than about Tauri or the app: the
`.dop` indentation / key-stability note, the `Settings` block and
`Orientation` requirements, the Uuid matching and virtual-copy behaviour, and
the `photolab::lookup` rename caveat. Moving them into their own guide,
`docs/agents/photolab.md`, makes them findable for anyone touching
`crates/core/src/dop.rs`, `crates/app/src/photolab.rs` or the `.dop` side of
`crates/app/src/sidecar.rs`, and keeps `tauri-app.md` to the app's own
pitfalls.

The same PR records a hand-run measurement with PhotoLab 10.0.1 on Windows
(2026-10-01) that settled whether a fresh `.dop` without Uuids could sidestep
`photolab.rs`'s database lookup: it cannot. Omitting the `Items[0].Uuid` and
`Source.Uuid` lines still imports a virtual copy (the new `Items` row's Uuid
is the nil UUID `00000000-0000-0000-0000-000000000000`, the master keeps the
database state); setting both to `""` makes PhotoLab ignore the sidecar (no
virtual copy, nothing applied, database unchanged). The random-Uuid control
reproduced the documented virtual copy, and PhotoLab rewrote none of the
`.dop` files. Recording this in the new guide and the two related `todo.md`
items stops the question from being re-investigated: the database's Uuids
remain the only fix, so `photolab.rs` stays.

Lightroom: `docs/agents/**` holds no Lightroom Classic- or
Lightroom-specific knowledge (the only mentions are archived-plan source
paths and one incidental clause in the `LabelNames` fallback section, which
is about Riffle's own reader). The Lightroom / Lightroom Classic behaviour is
user documentation in `README.md`. So no `docs/agents/lightroom.md` or
`docs/agents/lightroom-classic.md` is created; if such knowledge lands later,
give each app its own guide, with the shared parts in one and a pointer from
the other.

## Steps

- [x] Step 1: Create `docs/agents/photolab.md` from the four PhotoLab sections of `tauri-app.md`, add the Uuid-less `.dop` results, and update the two `todo.md` items
  - Done when:
    - `docs/agents/photolab.md` exists, with a `# ` title and a short intro in
      the style of `docs/agents/github-actions-workflows.md` ("Read this
      before touching `crates/core/src/dop.rs`, `crates/app/src/photolab.rs`
      or the `.dop` side of `crates/app/src/sidecar.rs` ..."; "The tags
      follow `tauri-app.md` (removed since): **Hit** ... **Measured** ...
      **Inferred** ..."), followed by these four sections moved verbatim from
      `tauri-app.md` (heading text, body, bullets and `Source:` lines
      unchanged; only the heading level may change to fit the new file):
      - "`.dop` indentation follows the table, and keys are not stable across
        versions (Hit)" (was line 706)
      - "`.dop`'s `Items[0]` needs a minimal `Settings` block, or PhotoLab
        reports no images (Hit)" (was line 719; keeps its `Orientation`
        paragraph)
      - "PhotoLab matches `.dop` items by Uuid; a fresh sidecar on a
        registered image becomes a virtual copy (Hit)" (was line 757)
      - "`photolab::lookup` keys on folder + file `Name`; a rename does not
        follow through to PhotoLab or a shared sidecar (Hit)" (was line 804)
    - In the moved "PhotoLab matches `.dop` items by Uuid" section, the list
      of hand-run results (starting "unregistered image + fresh `.dop` with
      random Uuids") has two new bullets after the "carrying the database's
      Source Uuid and master Item Uuid" bullet:
      - registered image + fresh `.dop` with the `Items[0].Uuid` and
        `Source.Uuid` lines omitted (dates newer than the database item's
        `ModificationDate`): imported as a virtual copy carrying the rating /
        pick; the new `Items` row's Uuid is the nil UUID
        `00000000-0000-0000-0000-000000000000`; the master keeps the database
        state.
      - registered image + fresh `.dop` with both Uuids `""`: ignored; no
        virtual copy, nothing applied to the master, database unchanged.
      - plus one sentence (in the same section, e.g. appended to the "So
        `sidecar::write_kind` ..." paragraph) that PhotoLab rewrote none of
        the sidecars and that a Uuid-less `.dop` therefore does not get
        around the lookup (PhotoLab 10.0.1, Windows, 2026-10-01).
    - `docs/agents/tauri-app.md` no longer contains those four sections. At
      the place they were (between the "Path comparisons in the UI take the
      platform `ignoreCase` flag" section and "Removing an XMP element needs
      its end tag") there is one short paragraph pointing to
      [`photolab.md`](../../../agents/photolab.md) for the `.dop` / PhotoLab pitfalls.
      Nothing else in `tauri-app.md` changes (the APFS `exists()` section's
      `.dop` example, the sidecar-generic sections at the former lines 945,
      961 and 1792, and the passing `.dop` mentions in the watcher and CSS
      sections stay).
    - `CLAUDE.md`'s Layout paragraph points at the new guide next to its
      `src/dop.rs` or `src/photolab.rs` mention, the way it already points
      at `docs/agents/ui-styling.md` for `style.css` (one parenthetical "see
      `docs/agents/photolab.md`"). No other index exists: `grep -rn
      'tauri-app.md#'` finds no anchor link into the moved sections, and no
      `.claude/**`, code comment, `docs/**` or archived plan names them by
      title, so no other reference needs fixing; the implementer re-runs the
      grep (`tauri-app.md#`, the four heading texts, `photolab`, `\.dop`)
      before opening the PR and records the result in `learnings.md`.
    - `todo.md` "### App: PhotoLab's virtual-copy fix only works on Windows"
      and "### App: a busy PhotoLab database silently falls back to random
      Uuids" each gain one sentence in the description paragraph (not in
      the TODO checklist) saying that omitting or emptying the Uuids was
      checked (PhotoLab 10.0.1, Windows, 2026-10-01) and does not avoid the
      virtual copy — omitted gives a virtual copy with a nil Uuid, empty
      makes PhotoLab ignore the sidecar — so the database's Uuids remain the
      only fix.
    - No todo item to remove `photolab.rs` is added; `todo.md`'s "Open in
      Terminal" and "jq reserved words" items are untouched (another session
      is deleting them).
    - No `docs/agents/lightroom.md` or `docs/agents/lightroom-classic.md`
      is created (nothing to move; see Purpose).
    - `mise run ci` passes (Prettier formatting of the Markdown and lychee's
      offline link check with fragments, which covers `./photolab.md` from
      `tauri-app.md`, `./tauri-app.md` from `photolab.md` and any fragment
      link).
  - Implementation approach:
    - Move the text with a cut-and-paste, not a rewrite: the `Source:`
      bullets and the "(see the next entry)" cross-reference inside the
      `Settings` block section must still point at a section that follows it
      in the new file, so keep the four sections in their current order.
    - Headings: `tauri-app.md` nests these as `###` under `## Rust side`; the
      new guide has no "Rust side" / "Frontend" split, so `##` (or keep `###`
      under a single `##`) is fine. Pick one and keep it consistent; lychee
      `--include-fragments` only matters for links that use anchors, and
      none exist today.
    - `tauri-app.md` lines 706–818 (the four sections) are the removal
      range; verify with `git diff --stat` that the file shrinks by roughly
      that and nothing else moved.
    - `todo.md` sections at lines 1027–1034 and 1042–1049: append the
      sentence to the end of each description paragraph, reflowing to the
      existing wrap width.
    - The measurement procedure (folder opened once in PhotoLab with no
      `.dop` written, PhotoLab closed, a fresh `.dop` from `dop.rs`'s
      template placed with `Date` / `CreationDate` / `ModificationDate`
      stamped newer than the DB item's `ModificationDate`, PhotoLab reopened
      on the folder; three variants: Uuid lines omitted, both `""`, random
      control) goes into this plan folder's `learnings.md`; the guide keeps
      the brief bullets.
    - Docs-only; no code and no `README.md` / `README.ja.md` change.
    - Rebase onto `main` before opening the PR: another session is editing
      `todo.md` (deleting the "Open in Terminal" and "jq reserved words"
      items); the touched sections differ, so any conflict is trivial.

## Trade-offs and risks

- One step, not two: the move is a cut-and-paste of about 110 lines plus a
  pointer, and the new bullets and `todo.md` sentences are a dozen lines; a
  reviewer can check "verbatim" with a side-by-side diff in one sitting.
  Splitting would make the Uuid-less bullets land in a file that then moves
  in the next PR, which is worse to review.
- Where the Uuid-less bullets go: into the moved section in `photolab.md`,
  not into `tauri-app.md` first, so the PR shows the section once.
- Guide index: the repo has no `docs/agents/README.md`; the only
  guide-to-code pointer outside `docs/agents` is `CLAUDE.md`'s Layout
  paragraph, so that is where the new guide is listed. If the caller would
  rather keep `CLAUDE.md` untouched, the `tauri-app.md` pointer paragraph
  alone still makes the guide reachable from the file agents are told to
  read first; note the choice in `learnings.md`.
- Lightroom guides are skipped because there is nothing in `docs/agents/**`
  to move. If the caller wants `README.md`'s Lightroom / Lightroom Classic
  sections mirrored into agents guides, that is new writing, not a move, and
  should be its own plan.

## Progress

- (2026-10-01) Step 1 complete
