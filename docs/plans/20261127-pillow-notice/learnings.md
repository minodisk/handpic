# Learnings

## Step 1

- The upstream `LICENSE` (raw, `main`, fetched 2026-11-27) is 30 lines,
  UTF-8 with LF endings and a trailing newline, its three copyright lines
  using `©`. `crates/core/LICENSE-pillow` is a four-line header plus a blank
  line, then that file byte for byte (`tail -n +6` of the notice compares
  equal to the download).
- The header names no Pillow source file, version or commit: the comment
  of `resample.rs` names none, and none was checked against upstream.
- `bash tools/package/notices.sh <temp dir>` copied `LICENSE-pillow`
  identical to the source, and `shellcheck` passed on the script.
