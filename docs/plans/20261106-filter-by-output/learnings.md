# Learnings

## Step 1

- `outputs_of` cannot reuse `read_listing` or `folders::is_file`: both swallow
  errors (`entries.flatten()`, `kind` returning `None`), so an entry that fails
  to read would silently drop an export and turn its RAW into `No output`.
  `output.rs` propagates every per-entry error instead.
- The match inverts the rule to stay linear: each export stem adds the RAW
  stems it can stand for to one set (its own, the stem minus a `_<digits>` or
  `-<digits>` suffix, and the text before every `-dxo`), then each RAW is one
  lookup. Every `-dxo` occurrence counts, not just the first, so a RAW whose own
  stem holds `-DxO` still matches its DeepPRIME export.
- Decisions the plan left open, all on the side of giving no wrong answer:
  - A `dir` that no longer exists is `Ok(None)`, since neither folder exists;
    any other failure to read `dir` is `Err`, because an export folder may be
    in it.
  - A dangling symlink named `Output`, or one inside an export folder, is
    `Err` rather than skipped. It cannot be told from an unreachable target.
  - On a case-sensitive file system every spelling is read, so `Output/` and
    `output/` side by side both count.
  - An empty export folder is `Some([])`: the folder was read whole and holds
    no export.
- Known over-matches of the rule as specified, both safe because they keep a
  file out of `No output`: `BF_00634_1.jpg` also counts for `BF_00634.DNG` when
  it is really `BF_00634_1.DNG`'s own export, and "any extension" lets a stray
  `X.xmp` in `Output/` count for `X`.
- The symlink tests are `#[cfg(unix)]`, so they run on the Linux and macOS CI
  legs, not on the Windows machine this step was checked on. The `Err` path of
  an unreadable folder is covered on every platform by handing `export_names`
  a path that is a file.
