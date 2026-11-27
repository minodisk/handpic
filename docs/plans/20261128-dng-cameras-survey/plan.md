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

# DNG cameras and Sony portrait orientation from the raw.pixls.us survey

## Purpose

A survey of the public raw.pixls.us samples on 2026-11-27 confirmed that
Handpic reads the preview and the EXIF of 22 more DNG-writing bodies
(Pentax, Ricoh, Leica, Apple, Google, Samsung) and that a portrait Sony ARW
is shown upright with its AF point on the subject. This change lists those
bodies in `docs/humans/cameras.md` and its Japanese pair,
records the orientation result, and moves what the survey found unsupported
into `todo.md`. Only confirmed facts are listed; nothing in the code changes.

## Steps

- [x] Step 1: List the surveyed DNG bodies, record the Sony portrait check and update `todo.md`
  - Done when:
    - `docs/humans/cameras.md` and `docs/humans/cameras.ja.md` hold the same
      22 new rows in the same order, the DNG paragraph and the Sony
      orientation paragraph below (the `.ja.md` body in Japanese).
    - `todo.md` has the Sony orientation line removed and its heading and
      background corrected, the Tier 1 line narrowed, and the four new items.
    - No file outside those three and this plan folder changes.
    - The PR's GitHub checks pass.
  - Implementation approach:
    - **Table rows.** In both files, insert these rows directly after
      `| Leica M11-P | – | – | – | – |` and before `| Canon EOS R | ...`. The
      row text is identical in both files (camera names stay in Latin
      script). If another session's Nikon rows conflict on rebase, keep
      theirs; these rows are in a different block.

      ```markdown
      | Leica M10-R | – | – | – | – |
      | Leica M10 | – | – | – | – |
      | Leica SL2 | – | – | – | – |
      | Leica Q2 | – | – | – | – |
      | Leica CL | – | – | – | – |
      | Pentax K-1 Mark II | – | – | – | – |
      | Pentax K-1 | – | – | – | – |
      | Pentax K-3 Mark III | – | – | – | ✓ |
      | Pentax K-3 Mark III Monochrome | – | – | – | ✓ |
      | Pentax K-70 | – | – | – | – |
      | Pentax KP | – | – | – | – |
      | Ricoh GR III | – | – | – | – |
      | Ricoh GR IIIx | – | – | – | – |
      | Apple iPhone 12 Pro | – | – | – | ✓ |
      | Google Pixel 8 Pro | – | – | – | ✓ |
      | Google Pixel 7 Pro | – | – | – | ✓ |
      | Samsung Galaxy S23 Ultra | – | – | – | ✓ |
      | Samsung Galaxy S23+ | – | – | – | ✓ |
      | Samsung Galaxy S23 | – | – | – | ✓ |
      | Samsung Galaxy S22 Ultra | – | – | – | ✓ |
      | Samsung Galaxy S22 | – | – | – | ✓ |
      | Samsung Galaxy S21 Ultra | – | – | – | ✓ |
      ```

      (The S22 row stands for the SM-S901U and SM-S901E samples, the S22
      Ultra row for SM-S908U1, the S21 Ultra row for SM-G998B.)
    - The Pixel 7 Pro (1280x964) is listed because both files exclude a
      body only when its largest embedded JPEG is "under 1280 px" /
      "1280 px 未満". Its `✓` for sub-second follows the survey's dump
      (`subsec Some("682399")`), not the report's summary line, which omits
      it.
    - Bodies under the floor (Pixel 3a, 4 XL, 4a, 6 Pro, iPhone 7 Plus, SE,
      XS) and bodies with no embedded preview are not mentioned: the policy
      paragraph already says such bodies are not listed.
    - **DNG paragraph.** Add one paragraph directly before the paragraph
      that starts "The Leica M (Typ 240) and M Monochrom (Typ 246) are not
      listed" (Japanese: "Leica M（Typ 240）と M Monochrom（Typ 246）は表に
      載せていません"). English:

      > The Leica, Pentax, Ricoh, Apple, Google and Samsung bodies above
      > write DNG. None of their samples recorded an AF point, an AF frame
      > size or face tracking. The Pentax and Ricoh samples show no lens
      > name, and the Leica M10 sample shows no aperture: these bodies
      > record them only in their MakerNote, which Handpic does not read
      > for them yet. The Pixel 7 Pro sample's embedded JPEG is 1280x964,
      > exactly at the 1280 px floor.

      Japanese:

      > 上の表の Leica、Pentax、Ricoh、Apple、Google、Samsung のカメラは DNG
      > を書き出します。これらのサンプルはどれも AF 点、AF 枠のサイズ、顔追尾
      > を記録していません。Pentax と Ricoh のサンプルではレンズ名が表示され
      > ず、Leica M10 のサンプルでは絞り値が表示されません。どちらもメーカー
      > ノートにしか記録されておらず、Handpic はまだそれを読まないためです。
      > Pixel 7 Pro のサンプルの埋め込み JPEG は 1280x964 で、長辺がちょうど
      > 下限の 1280 px です。

      Wrap the English paragraph at the file's existing line width; the
      Japanese file keeps one paragraph per line, as its other paragraphs do.
    - **Sony orientation paragraph.** The table has no orientation column,
      so the result goes in body text only. Add one paragraph directly
      after the paragraph that starts "The Sony α9 II, α7R IV, α7R IVA, α7C,
      α6400, α6600 and ZV-E10 embed no full-size JPEG". English:

      > Portrait orientation is confirmed on an α6400A (ILCE-6400A) sample
      > shot with the camera turned (Exif `Orientation` 8): Handpic shows it
      > upright and the focus mark lands on the subject. Handpic turns the
      > image and the AF point the same way on every Sony body, so the other
      > bodies are not checked one by one.

      Japanese:

      > 縦位置は α6400A（ILCE-6400A）のサンプル（Exif の `Orientation` が 8）
      > で確認しました。Handpic はこのファイルを正しい向きで表示し、フォーカス
      > マークは被写体の上に来ます。画像と AF 点の回転はどの Sony のカメラでも
      > 同じ処理なので、ほかのカメラは個別には確認していません。

    - **`todo.md`, Sony item.** Rename the heading
      `### Docs: Sony AF point, face tracking and portrait orientation are unconfirmed for lack of samples`
      to
      `### Docs: Sony AF point and face tracking are unconfirmed for lack of samples`.
      Delete the Background bullet that starts "No sample is a portrait
      frame (Orientation 1 on all 66 files)". Delete the TODO line
      `- [ ] Confirm orientation on a portrait ARW sample and record the result in docs/humans/cameras.md.`
      Leave the first TODO line as it is.
    - **`todo.md`, Tier 1 line** under
      `### Core: widen camera support from public sample RAW files`. Replace
      the `- [ ] Tier 1: ...` item with:

      ```markdown
      - [ ] Tier 1: verify preview/EXIF extraction on DNG samples of the
            bodies raw.pixls.us did not hold in the 2026-11-27 survey: the
            Leica M11 family beyond the M11-P, the Q3 and the SL3, the Pentax
            KF (its samples there are PEF only), the Ricoh GR IV, and ProRAW
            from the iPhone 13 or later; add each working body to the
            `docs/humans/cameras.md` table. The Pentax, Ricoh GR, Leica and
            phone bodies raw.pixls.us holds are done and listed there.
      ```

      Leave the tier description and the Tier 2 and Sigma BF lines untouched.
    - **`todo.md`, four new items.** Insert them directly after the
      `### Core: widen camera support from public sample RAW files` section
      (before `### Core: M-RAW RAF files are unverified`), not at the end of
      the file. Each uses the `### <Area>: <title>` / `#### Background` /
      `Files:` / `#### TODO` shape of the neighbouring items:

      ```markdown
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
      ```

    - Evidence for every value above is the survey report and
      `index\tier1_dump.txt` in the survey's scratchpad
      (`C:\Users\daisu\AppData\Local\Temp\claude\4d0f8a7a7e14c812\109664e4-4c19-433a-9e5f-07dfe5e7c936\scratchpad\`);
      the samples are on raw.pixls.us (CC0). Do not commit sample files.
    - No `README.md` change: its Compatibility section lists formats, not
      bodies.

## Trade-offs and risks

- **`cameras.ja.md` drift.** Decided with the user: add only this change's
  rows and paragraphs to both files and file the full resync as a todo item.
- **Pixel 7 Pro sub-second.** The report's summary omits it, but the dump
  shows a sub-second value, so the row is `✓`.
- **Size of the Pentax / Ricoh JPEG.** The request described it as tens of
  MB; the dump shows 2.9 to 6.4 MB. The todo text uses the dump's range.
- **Which phone DNGs lack EXIF.** The request named three; the dump shows
  the Pixel 2 XL failing the same way, and all four still give Make and
  Model. The todo text lists four and says "none of the shooting fields".
- **α6400A row.** Not in the table; mentioned only in the orientation
  paragraph, since a row would need its AF frame size, which the report does
  not state.
- **Concurrent edits.** Another session is adding Nikon Z6III / Z50II / Z5II
  rows and NEF crop notes to both cameras files, and `todo.md` changes often.
  On a conflict, keep the other session's lines and re-apply these.

## Progress
