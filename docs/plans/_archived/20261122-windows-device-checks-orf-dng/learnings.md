# Learnings

## Step 1

- The pre-bump tree (`425667a2`, `EXTRACTOR_VERSION` 11) is the removed Tauri app. To keep its settings, window state and index apart from real data, only `tauri.conf.json`'s `identifier` needed changing (to a test identifier); its first-run sidecar dialog had to be answered (Lightroom, then Continue) before it scanned.
- Seeding `lastFolder` in the old build's settings made it open the test folder without a click.
- Git Bash mangled `\\` in the MCP JSON argument even inside single quotes; the client expanded placeholders for the test folder and the backslash instead. This is already a rule in `docs/agents/app.md`.
- MCP `set_view` `zoom` entered the 1:1 view, so the 1:1 captures needed no synthetic input.
- `docs/agents/app.md` got nothing. The reusable lessons (the MCP test-run rule, the `\\` mangling) are already rules there. Making the old index needed the Tauri app at one commit, which takes no `--index`; a later `EXTRACTOR_VERSION` check builds a GPUI parent commit and passes `--index`, so the identifier change and the first-run dialog do not recur.
- The plan's Done-when first said `todo.md` holds 138 `###` headings; `origin/main` (`ba9efeaf`) holds 136, and the edit keeps 136. The plan was corrected. The first `###` heading sits directly under `## Cross-cutting / other` with no blank line, as on `origin/main`; left as is.

## Deferred issues (todo candidates)

- The lens of `E-M5MarkII__1230049.ORF` and `E-PM2_P1010001.ORF` is stored as `LEICA DG SUMMILUX 25/F1.4 ` with a trailing space, while make and model are trimmed. Added to `todo.md` as `### Core: the lens name of some ORFs keeps a trailing space`. Basis: `results.md`, "An ORF folder in the strip, sidecars and Move to Trash", the FINDING after (3). Files: `crates/core/src/exif.rs`, `crates/core/src/arw.rs`, `crates/engine/src/exif.rs`, `crates/engine/src/index.rs`.
- The three Leica M (Typ 240) DNGs show only their 160x120 JPEG, upscaled in the fit view and small in the 1:1 view. Added to `todo.md` as `### Core: the Leica M (Typ 240) DNGs show only their 160x120 JPEG in the fit view and the 1:1 view`. Basis: `results.md`, "Big-endian DNGs after the EXTRACTOR_VERSION bump", the FINDING. Files: `crates/core/src/arw.rs`, `crates/app/src/zoom.rs`.
