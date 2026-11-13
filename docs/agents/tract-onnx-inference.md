# Running ONNX models with `tract-onnx` in `crates/core`

Read this before touching `crates/core/src/faces.rs` (the YuNet face / eye
detector), `crates/core/src/eyes.rs` (the MediaPipe Face Landmarker v2 face
mesh that judges closed eyes), `crates/core/src/embed.rs` (the SigLIP 2
image embedding) or adding another ONNX model to `crates/core`. It lists what loading
and running a model through `tract-onnx` 0.23 took here, each with the reason.

The tags follow [`tauri-app.md`](./tauri-app.md): **Hit** broke something
here, **Measured** steered a design decision, **Inferred** comes from sources
or docs only.

The measurements behind these items (binary size and build time with tract,
per-call latency, the model build time) are in the
[face-aware-sharpness learnings](../plans/_archived/20260922-face-aware-sharpness/learnings.md),
Step 1 and "App binary size"; they are not repeated here.

## Loading

### Ignore the model's fixed-size shape annotations and pin the input (Hit)

The YuNet ONNX file carries `value_info` and output shapes for a 640 px input.
Handpic feeds it 320x320 (`INPUT`) and 640x448 / 448x640 (`WHOLE_INPUT`), and
with a different input fact tract fails to unify the annotated shapes against
the inferred ones. `build` therefore loads with
`tract_onnx::onnx().with_ignore_value_info(true)
.with_ignore_output_shapes(true)` and then pins the input with
`with_input_fact(0, f32::fact([1, 3, height, width]).into())`.

- Rule: when a model's annotations are for another input size than the one
  you feed, drop the annotations rather than resizing to match them, and
  always pin the input fact so the plan is fully typed before optimizing.

### Find outputs by outlet label, not by node name (Hit)

After loading, tract names the output nodes after their ops (`Sigmoid_85`,
`Reshape_132`, ...). The ONNX output names (`cls_8`, `obj_8`, `bbox_8`,
`kps_8`, then `_16` and `_32`) survive only as outlet labels. `build` maps
each `{kind}_{stride}` name to its output index through
`model.outlet_label(outlet)` over `model.output_outlets()` and fails with the
list of labels when one is missing.

- Rule: never index outputs by position or node name; resolve them by outlet
  label once at build time and keep the indices (`Detector::outputs`).

### The tract 0.23 API (Hit)

- `model.into_optimized()?.into_runnable()?` returns
  `Arc<TypedRunnableModel>` (the `Plan` alias in `faces.rs`); `run` takes
  `&Arc<Self>`.
- Run with `plan.run(tvec!(input.into()))?`.
- Read an output `TValue` with `try_as_plain_ram()?.as_slice::<f32>()`.

### `default-features = false` on `tract-onnx` (Measured)

`crates/core/Cargo.toml` depends on `tract-onnx` with
`default-features = false`, which drops `tract-transformers`: YuNet loads
and runs without it, and the binary and the build are smaller. Check a
new model's ops load with this setting before turning the defaults back on.

- The four embedding candidates of the image-embeddings plan (SigLIP 2,
  OpenCLIP, TinyCLIP, MobileCLIP) load with it too: their exports use plain
  `MatMul` / `Softmax` / `Erf` and either `LayerNormalization` or its
  decomposed form. Only ORT's fused `Attention` / `MultiHeadAttention` ops
  need `tract-transformers`, so pick plain exports (Measured).
- Quantized exports are another matter (Hit). onnx-community's SigLIP 2
  fp16 export fails to translate (the head `Gemm`: `expected 1,768,F16 got
  1,768,F32`), and its int8 export fails analysis (`ConvInteger`: `Impossible
  to unify U8 with I8`). Its uint8 export loads but runs at 8.4 s per image,
  and both dynamic quantizations are wrong in onnxruntime as well (cosine
  0.64 against FP32). Weight-only re-exports (float16 or per-channel int8
  initializers widened by `Cast` / `DequantizeLinear`) load and are
  accurate, but tract re-widens those weights on every run, at 2x to 5x
  FP32's cost. So the app runs the FP32 file. Source:
  [image-embeddings models.md](../plans/_archived/20261012-image-embeddings/models.md).
- Load a large model with `model_for_path`, not `model_for_read` on the bytes
  (Measured). `model_for_read` copies the bytes into a `Vec` of its own
  before decoding, so the caller's bytes, that copy, the parsed protobuf and
  the optimized plan's weights are alive together. `model_for_path` maps the
  file instead. On the 378 MB SigLIP 2 file the process peak went from
  1145-1161 MB to 773 MB, with the same plan build time (538 ms) and the same
  395 MB after the load. `embed::Model::load_path` is that path.
  `include_bytes!` is no help either: the static's pages plus `load`'s copy
  left 774 MB after the load. Source:
  [image-embeddings learnings](../plans/_archived/20261012-image-embeddings/learnings.md),
  Step 3.
- Object detectors load too, but the transformer heads cost too much (Measured).
  The onnx-community exports of D-FINE N / S and RT-DETRv2 R18 load (including
  `GridSample`) and run with `default-features = false`, contrary to the
  expectation that they would fail. tract runs them 9-12x slower than
  onnxruntime (1.0 / 2.4 / 2.8 s per image against 85 / 254 / 464 ms on one
  thread), while conv detectors cost 2-4x (NanoDet-Plus 73.5 against 34.9 ms,
  YOLOX Nano 53.1 against 14.5). A transformer-head detector is therefore ruled
  out on cost, not on ops: judge a candidate by its tract ms, not by whether
  it loads. Source:
  [ball-game-ranking-features learnings](../plans/_archived/20261014-ball-game-ranking-features/learnings.md),
  Step 3.

## Input and output

### Input layout: BGR NCHW, raw 0..255 floats (Inferred)

`input_tensor` box-averages the image by the scale that fits it inside the
model input (`fit`) and writes it into the top-left of a zeroed tensor of
shape `[1, 3, height, width]`, channels in BGR order, values as raw 0..255 floats
(not normalized), as OpenCV's `FaceDetectorYN` feeds YuNet. `detect` divides
the decoded boxes and eye points by the same scale.

- Why fixed sizes: the input fact is pinned, so each optimized plan serves
  every file whatever its aspect ratio. The `CATCH_CROP` crop is square, so
  its 320x320 input wastes nothing. The whole-image search uses a 640x448
  input and its transpose for portrait frames instead of a 640 square: a
  3:2 preview would leave a third of the square as padding, and the
  inference cost follows the input area (Measured: 59.6ms against 85-87ms
  per run, same recall; see "Whole-image recall (Windows 11)" in
  [`performance.md`](../humans/performance.md)).
- The grid of each stride is `width / stride` cells per row
  (`decode_stride`'s `cols`), so a non-square input decodes with the input's
  width, not its height.

### Feed an upright image, map detections back to stored coordinates (Hit)

YuNet is trained on upright faces: an unrotated portrait preview (orientation
6 / 8) loses most of its recall and swaps left / right eyes for top / bottom.
The preview is decoded as stored, so `detect_around_rgb` rotates it with
`upright_rgb` before `detect`, maps the AF point into it with `to_upright`,
and maps every face back with `to_stored`, because the sharpness score and the
AF point (`partial::focus_point`) live in stored coordinates.

- `decode::apply_orientation` only rotates 6 and 8, so `upright_rgb` reverses
  the pixel order itself for orientation 3, and `to_stored` / `to_upright`
  treat 3 as a half turn. A new orientation-aware caller must do the same or
  reuse these helpers.
- The whole-image search decodes at a DCT scale instead (`decode_whole`,
  `decode::decode_upright_near`, already upright) and `scaled_to_stored`
  scales each face by the decoded size against the stored one before
  `to_stored`. mozjpeg rounds a scaled side up (`ceil(dim * n / 8)`), so map
  with the actual sizes, not `n / 8`, and keep `Detection::width` / `height`
  at the stored size: `faces_of` and the UI scale the boxes by it.
- The AF point these coordinates meet comes from the MakerNote parsing
  described in [`raw-metadata-parsing.md`](./raw-metadata-parsing.md).

### Do not swap the cue path's full RGB decode for a cheaper grayscale + scaled decode (Measured)

The cue path (`detect_around_rgb` plus `candidate::luma`) decodes the preview
once at full size, which looks wasteful next to the whole-image search's DCT
scaled decode. A variant that took a grayscale decode for the luma and a 6/8
RGB decode for the face search was faster (11.6 ms against 13.6 ms per file)
but changed the results: 16 of the training set's per-file candidate states
changed and 6 of the held-out set's, with AUC 0.816 -> 0.819 and 0.635 ->
0.646. mozjpeg's grayscale is the Y channel, not the BT.601 integer luma of
the RGB, and the scaled crop's pixels differ.

- Rule: the cue path keeps its single full RGB decode. Only a change that
  passes an identity check of `candidates` on the labeled folders (identical
  per-file lines) may replace it, otherwise it needs a `FACES_VERSION` bump
  and a re-evaluation of the cue's threshold.
- Source: [face-detection-recall-cost learnings, Step 3 "Cue-path variant"](../plans/_archived/20261005-face-detection-recall-cost/learnings.md).

### The face mesh: RGB NHWC 0..1, points by `Identity` (Measured)

`eyes.rs` runs the second model, MediaPipe Face Landmarker v2
(`face_landmarks_detector.onnx`), and it is the first NHWC model here: its
input fact is `[1, 256, 256, 3]` (channels last), RGB, values 0..1, against
YuNet's `[1, 3, height, width]` BGR raw 0..255. `input_tensor` resizes the
face crop bilinearly to 256x256 and builds the `Array4` with the shape
`(1, INPUT, INPUT, 3)`. The layout and range come from MediaPipe's pipeline
config (`face_landmark_cpu.pbtxt`: `output_tensor_float_range 0..1`), not
from the ONNX file; a wrong channel order or range is not an error, it only
shows in the accuracy, so check a new model's input against its pipeline
and a labeled set, not only whether it loads.

- Outputs by outlet label: `Identity` is the 478 points, (x, y, z) each, x
  and y in input pixels (map them back by the crop window's position and
  per-axis scale, since a clamped crop near the image edge is not square);
  `Identity_1` is the face presence logit and `Identity_2` is unused. Only
  `Identity` is resolved (`Model::points`).
- The z of each point is a relative depth, larger away from the camera, in
  input pixels at the scale of x (Inferred: MediaPipe's
  `LandmarkProjectionCalculator` scales it by the ROI width like x and
  `TensorsToLandmarks` divides it by the input size like x). `to_full`
  maps it with the horizontal factor `kx` only; `pose::head_pose` reads it
  for the head pose (MediaPipe's face geometry pipeline). Checked by hand:
  the yaw / pitch signs it yields agree with the face crops of a few
  turned and tilted faces, and keeping z left the `riffle-cli eyes` EAR
  columns of 40 labeled faces byte-identical (Measured).
- The head pose (`pose::head_pose`, a port of MediaPipe's face geometry
  pipeline at commit `212f110c`) needs no linear algebra crate. The 3x3
  rotation is the polar factor of a cyclic Jacobi solve. Two details of the
  port are easy to get wrong. The depth offset is the **plain** mean z over
  all 468 points; only the 33 basis points enter the Procrustes solves. The
  pipeline also drops a face whose normalized points lie within 1e-3 of their
  mean (`too_compact`, a `None`). It costs about 8 us per face, release,
  single thread, so it does not move the `eyes_of` time (Measured). Only the
  roll is bounded (`MAX_ROLL`). A yaw past 90 deg is kept, because the sign
  was right on 16 of 18 readable far profiles. The 63 deg vertical FOV is kept
  because the FOV is not measurable from `Shot`, which lacks the 35 mm
  equivalent focal length (Measured).
  Source: [head-pose learnings, Steps 1, 2 and 4](../plans/_archived/20261007-head-pose/learnings.md).
- Do not rotate the mesh crop by YuNet's eye line to correct roll (Measured,
  declined). YuNet's two eye points are ordered by image x (`faces.rs`), so
  the eye-line roll is folded into -90..+90 deg. A head-down face therefore
  reads about -11 deg, and no eye-line rotation can fix it. On profile and
  far-turned faces the two points sit together on the visible eye or the
  nose (eye distance 0.14 of the box side or less), so the angle is bogus
  and the rotation makes a fitting mesh worse. Always-rotate broke 15 and
  fixed 7 of 131 readable truth frames and dropped the held-out AF AUC from
  0.800 to 0.783. An eye-distance guard changed no labeled outcome. Most of
  `eye_offset` > 0.10 (28.7% of frames, unchanged by rotating) is a loose
  comparison. About 27% of those frames have a mesh off the face, and about
  65% have an `on` mesh against YuNet points that are off the eyes. So
  treat a large `eye_offset` as unreliable when the YuNet eye distance is
  under 0.20, rather than as a mesh failure. The code is reachable from
  #755 (`a5ee087e`) and #758 (`c1843389`). Source:
  [mesh-roll learnings, Steps 1-3](../plans/_archived/20261010-mesh-roll/learnings.md).
- The file is a tf2onnx 1.17.0 conversion of Google's TFLite (provenance in
  `crates/core/models/LICENSE-mediapipe`); tf2onnx widens the float16
  weights to float32, so the ONNX is about twice the TFLite (4.9 MB).
  tflite2onnx 0.4.1 failed on the face meshes (an `IndexError` in its layout
  propagation) (Hit).
- It loads and optimizes with `default-features = false` and the same
  `with_ignore_value_info(true)` / `with_ignore_output_shapes(true)` as
  YuNet (Measured).
- The first call builds the plan: 156-169 ms for the first call against
  ~35 ms for a later one on the Windows 11 machine, so ~120-135 ms of plan
  build, three to four times YuNet's (Measured). A caller that times
  per-face runs warms it first, as `riffle-cli eyes` and `bench` do.
- Per face, crop and resize included: 34.5-35.5 ms mean over the 504
  labeled faces, one thread (Measured). The Step 1 survey measured 49-51 ms
  for the same function while it ran three other models on each face in
  between; treat the lower figure as the model's own cost.
- When comparing `times_eyes_of_on_real_files` before and after a change,
  build both revisions back to back and repeat the first one. A run straight
  after a ~4-minute release compile read 81-92 / 144-160 ms against
  61-68 / 111-121 ms on the same code once the cache was warm. The order alone
  fakes a ~20 ms regression (Measured). Source:
  [head-pose learnings, Step 4](../plans/_archived/20261007-head-pose/learnings.md).
- The scan's second pass runs the mesh from the rayon pool, one face per
  file (`eyes::mesh_of` from `candidate::focus_cue_unless`, through
  `scored_face`), all workers sharing the one `OnceLock` plan. With 24 workers on the 2134-ARW folder a
  face took 98.6 ms mean (p95 114.7 ms) against 28.8 ms on one thread, the
  cores being shared with the other files' decodes and detections, and the
  pass went from 12.9 to 21.5 s. It runs only on faces whose box long side
  is at least `eyes::EYES_MIN_FACE` (`candidate::meshes_face`); that skips
  13.4% of the faced files on the 2134-ARW folder and saved about 0.4 s of
  ~20.6 s, within the run-to-run spread (Measured, `docs/humans/performance.md`
  "Focus candidate pass"). The whole `riffle-cli candidates` command
  peaked at 405-411 MB with 24 threads (Measured). `mesh_of` rotates only
  the face crop of a portrait preview, not the whole image: 0.016 ms
  against 2.06 ms per call (Measured). Source:
  `docs/plans/_archived/20261008-mesh-eye-focus/learnings.md`, Steps 1 and
  4, and `docs/humans/performance.md` "Focus candidate pass".
- Gating the mesh by face size changed no candidate state on the labeled
  sets or the 2134-ARW folder. One probability moved (`_DSC2748.ARW`, a 58x49
  face, `p 0.209` -> `0.328`, `eye L only` -> `window`, still `NotCandidate`),
  so `FACES_VERSION` was not bumped (Measured). Every mesh call site must use
  `candidate::meshes_face`, including the `riffle-cli` debug subcommand that
  prints the nearest face's scored region. Otherwise its line disagrees with
  the cue printed above it. No non-ignored `riffle-core` test runs the mesh
  model, so prove a gate with a small face in the synthetic image
  (`Scored::Window`, `mesh: None`) plus a predicate check at
  `EYES_MIN_FACE` and `EYES_MIN_FACE.next_down()`. Source:
  [mesh-face-gate learnings, Step 1](../plans/_archived/20261008-mesh-face-gate/learnings.md).
- Mesh eye regions are boxes in stored coordinates, so on a portrait preview
  (orientation 6 / 8) the eyes stand on end and a box's width is the eye's
  height (a 97 px face printed `L 11x20`). Any size rule on a region (the
  margin, the minimum size, a relative edge width) must use the box's longer
  side, not its width. For an upright eye the two are the same. The regions
  are small: the median contour box with its 0.25 margin is 14-15 px on its
  longer side, so a 24 px floor sends most frames to the window fallback.
  Choose such a floor from the AUC by bucket, not from the window's minimum.
  Source: `docs/plans/_archived/20261008-mesh-eye-focus/learnings.md`, Step 1.
- Measuring the AF eye cue on the eyelid contour polygon (even-odd fill,
  dilated by a disk of 0.1-1.0 x the contour box's longer side) instead of
  the contour's margin box did not beat the rectangle: the best refit cell
  (both measures masked, dilation 0.1, a 50 px floor) scored 0.797 / 88.6% /
  95.0% held-out against the rectangle's 0.800 / 88.6% / 95.9%. Do not retry
  a mask without a new reason. The masks are also tiny: at dilation 0 the
  median window's longer side is 10 px and 89 of 600 training eyes had no
  edge width. A masked edge width can be exactly 0 (a seed on the edge of a
  window 1-2 px wide), so `MESH_LOGIT_EDGE_WIDTH * rel.ln()` would be `+inf`.
  Guard `ln(0)` in any variant that measures edges on a tight mask window
  (Measured). The primitives were removed; `maskdump.patch` targets commit
  `35b1edce`, so reproducing the dump means checking that commit out first.
  Source: `docs/plans/_archived/20261008-mesh-eye-mask/learnings.md`, Steps
  1, 2 and 4.
- Source: [closed-eyes-detection learnings](../plans/_archived/20261007-closed-eyes-detection/learnings.md),
  Steps 1 and 2.

### The image embedding: RGB NCHW, PIL's resampling, a DCT-scaled decode (Measured)

`embed.rs` runs SigLIP 2 ViT-B/32 @256 (`embed::SHIPPED`): input fact
`[1, 3, 256, 256]` (channels first, like YuNet), RGB, values
`(x / 255 - 0.5) / 0.5`, the upright image stretched to 256x256 without a
crop, output by the outlet label `pooler_output` (768-d), L2-normalized. The
other candidates' contracts are in `embed::SPECS` (CLIP mean / std, a short
side resize and a center crop for OpenCLIP and TinyCLIP).

- Every candidate's Python processor resizes through PIL, so `resample`
  reimplements PIL's `precompute_coeffs` (the kernel widened by the
  downscale factor, each pass rounded to 8 bits). With it the vectors match
  onnxruntime + PIL at cosine 0.999997 or better. A plain bilinear resize is
  not an error, it only lowers that cosine, so check a new model against a
  Python reference on real images, not only whether it loads.
- The input is the preview decoded at the DCT scale nearest 404 px
  (`embed::embed_preview`, `decode::decode_upright_near`), not the index's
  404 px thumbnail. The thumbnail's q80 4:2:0 re-encode moved the SigLIP 2
  vector to cosine 0.96 of the full preview's (top-5 neighbor agreement 0.52
  within one folder). The scaled decode keeps 0.998 (0.84) and decodes in
  9 ms against 26 ms for the full preview.
- The file is not bundled. The app downloads it on the first enable (the
  backend's `ai.rs`) and builds the plan in `embed::shared`, whose `OnceLock`
  keeps a failed build for good like `faces.rs`'s, so it is called only once the
  file is in place (`AppAi::model_if_enabled`). Once built, the plan stays
  resident (about 395 MB) for the rest of the process.
- Source: [image-embeddings models.md](../plans/_archived/20261012-image-embeddings/models.md)
  and the plan's Decision 1.

### An embedded model only weighs on the binaries that call it (Measured)

`include_bytes!` puts the model in a `static`; the linker drops it from a
binary that never reaches the code reading it. Adding `eyes.rs` grew the
release `riffle-cli` (whose `eyes` and `bench` call it) by 4.97 MB and the
release `riffle-app` (which did not call it yet) by 8 KB. Measure a model's
binary cost on a build where the app actually calls it.

Once `eyes_of` called it, the release `riffle-app` grew from 47,499,776 B to
52,494,336 B (+4,994,560 B, +10.5%), about the ONNX file, so a model's full
size lands on every binary that reaches it. Source:
[closed-eyes-detection learnings, Step 3](../plans/_archived/20261007-closed-eyes-detection/learnings.md).

## Sharing

### Build each plan once in a `OnceLock` shared by rayon workers (Inferred)

The scan runs `scan::extract` and `scan::extract_analysis` on rayon workers. `detector(input)` builds the plan
of each model input (`faces::Input`: the square, the landscape and the
portrait whole-image input) once in its own `static OnceLock` and hands every
worker the same `&'static Detector`;
nothing is rebuilt per file. The lock caches a `Result<Detector, String>`
(the error formatted with `{e:#}`), so a model that fails to build fails
every call quickly with the same message instead of being rebuilt per file.

- Rule: keep one plan per model input, built lazily on first use, with its
  failure cached too. A plan is ~35-40ms to build and is not per worker;
  the larger input's per-run buffers are (Measured: the 146-DNG folder's
  peak working set on 24 threads rose from ~510 to ~646 MB with the
  640x448 plans).

Source: [face-aware-sharpness learnings](../plans/_archived/20260922-face-aware-sharpness/learnings.md),
Steps 1 and 3.
