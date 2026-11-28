# Windowed sharpness luma: bit-identity check on real files

Step 1 computes the sharpness score's luma over the score's window only
(`scan::score_rgb` through `sharpness::region`). This file records the
real-file check that the stored score did not change.

## Run

2026-11-28, Windows 11, `cargo build --release -p handpic-cli`, one run on
one thread, with an uncommitted scratch subcommand `lumawindow` in
`crates/cli/src/main.rs` (removed after the run). For each file it printed
`scan::extract_analysis(..).sharpness` (pass 2's stored score, windowed luma)
and the reference `sharpness::score_gray(&candidate::luma(&decode_rgb(preview)), ..)`
over the full-frame luma, with the trusted AF point, the `eye_af_frame`
frame and, without an AF point, the cue's whole-image faces; both compared
as `f64::to_bits`. No timing was taken.

## Result

| File | Path | Stored bits | Reference bits | Same |
|------|------|-------------|----------------|------|
| `2026-09-19/_DSC1796.ARW` | eye-AF frame | `0x406922dc95df7d4f` | `0x406922dc95df7d4f` | yes |
| `2026-09-19/_DSC1797.ARW` | eye-AF frame | `0x40886bd1c4f9284a` | `0x40886bd1c4f9284a` | yes |
| `2026-09-19/_DSC1802.ARW` | eye-AF frame | `0x407c6f76d46618d5` | `0x407c6f76d46618d5` | yes |
| `2026-09-19/_DSC1806.ARW` | eye-AF frame | `0x407551c7b82f2e9d` | `0x407551c7b82f2e9d` | yes |
| `2026-09-19/_DSC1807.ARW` | eye-AF frame | `0x406ae1e419893550` | `0x406ae1e419893550` | yes |
| `2026-09-19/_DSC1813.ARW` | eye-AF frame | `0x407df7f25b58b2e9` | `0x407df7f25b58b2e9` | yes |
| `samples/RAF/X-H2S_DSCF0001.RAF` | `WINDOW` around the AF point | `0x408dbc478c065729` | `0x408dbc478c065729` | yes |
| `samples/RAF/X-H2S_fujifilm_x_h2s_02.raf` | `WINDOW` around the AF point | `0x40910a929dcd5600` | `0x40910a929dcd5600` | yes |
| `samples/RAF/X-E4_cmp_4448.RAF` | `WINDOW` around the AF point | `0x40592d372ced461d` | `0x40592d372ced461d` | yes |
| `2026-02-01/L1005161.DNG` | eye window of the judged face | `0x408382f30e6f23f2` | `0x408382f30e6f23f2` | yes |
| `2026-02-01/L1005172.DNG` | eye window of the judged face | `0x405085941126760a` | `0x405085941126760a` | yes |

Verdict: bit-identical on all 11 files and all three window paths.
`FACES_VERSION` stays 17.

The six ARW the luma-reuse spot check used all took the eye-AF frame path
(`eye_af=1`); the plain `WINDOW` path around the AF point is covered by the
three RAF files (larger previews) and by the unit test
`scan::tests::the_windowed_luma_scores_as_the_full_frame_luma`. On the Leica
DNG previews the RGB luma equals the Y channel, so those two rows would match
on the grayscale path too; the ARW and RAF rows are the evidence.
