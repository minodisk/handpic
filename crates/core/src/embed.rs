//! A CLIP-style image embedding of a preview, run through `tract`.
//!
//! The candidates' `ModelSpec`s are in `SPECS`; the app runs `SHIPPED`, whose
//! file it downloads on the first enable (`MODEL_URL`, checked against
//! `MODEL_SHA256`) rather than bundling it, and builds its plan once
//! (`shared`). The image is rotated
//! upright by its Exif orientation, resized with PIL's resampling filters
//! (the Python processors of every candidate resize through PIL), optionally
//! center-cropped, normalized per channel and fed as NCHW; the output vector
//! is L2-normalized.

use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};
use tract_onnx::prelude::*;

use crate::decode::decode_upright_near;
use crate::faces::upright_rgb;

/// How the upright image becomes the square model input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resize {
    /// Resize both sides to the input size, ignoring the aspect ratio.
    Stretch,
    /// Resize the short side to the input size, then cut the centered square.
    ShortSideCrop,
}

/// PIL's resampling filter of the model's processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Bilinear,
    Bicubic,
}

/// The input contract of one embedding model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSpec {
    pub name: &'static str,
    /// The side of the square input, in pixels.
    pub size: usize,
    pub resize: Resize,
    pub filter: Filter,
    /// Per channel in the model's channel order, on values 0..1.
    pub mean: [f32; 3],
    pub std: [f32; 3],
    /// Whether the channels are fed as BGR rather than RGB.
    pub bgr: bool,
    /// The outlet label of the embedding output.
    pub output: &'static str,
}

const CLIP_MEAN: [f32; 3] = [0.481_454_66, 0.457_827_5, 0.408_210_73];
const CLIP_STD: [f32; 3] = [0.268_629_54, 0.261_302_6, 0.275_777_1];

/// The measured candidates, from each export's processor config.
pub const SPECS: [ModelSpec; 4] = [
    // onnx-community/siglip2-base-patch32-256-ONNX `vision_model*.onnx`.
    ModelSpec {
        name: "siglip2",
        size: 256,
        resize: Resize::Stretch,
        filter: Filter::Bilinear,
        mean: [0.5; 3],
        std: [0.5; 3],
        bgr: false,
        output: "pooler_output",
    },
    // immich-app/ViT-B-32__laion2b-s34b-b79k `visual/model.onnx`.
    ModelSpec {
        name: "openclip",
        size: 224,
        resize: Resize::ShortSideCrop,
        filter: Filter::Bicubic,
        mean: CLIP_MEAN,
        std: CLIP_STD,
        bgr: false,
        output: "embedding",
    },
    // The vision part of onnx-community/TinyCLIP-ViT-40M-32-Text-19M-LAION400M-ONNX.
    ModelSpec {
        name: "tinyclip",
        size: 224,
        resize: Resize::ShortSideCrop,
        filter: Filter::Bicubic,
        mean: CLIP_MEAN,
        std: CLIP_STD,
        bgr: false,
        output: "image_embeds",
    },
    // Xenova/mobileclip_s2 `vision_model.onnx`; reference only, not shippable.
    ModelSpec {
        name: "mobileclip",
        size: 256,
        resize: Resize::ShortSideCrop,
        filter: Filter::Bilinear,
        mean: [0.0; 3],
        std: [1.0; 3],
        bgr: false,
        output: "image_embeds",
    },
];

/// The candidate spec named `name`.
pub fn spec(name: &str) -> Option<&'static ModelSpec> {
    SPECS.iter().find(|s| s.name == name)
}

/// The model the app runs (the plan's Decision 1): SigLIP 2 ViT-B/32 @256
/// FP32, `onnx-community/siglip2-base-patch32-256-ONNX`
/// `onnx/vision_model.onnx`, Apache-2.0 (`crates/core/models/LICENSE-siglip2`).
pub const SHIPPED: &ModelSpec = &SPECS[0];

/// The version of what `embed_preview` stores: the model (`SHIPPED`, its file
/// pinned by `MODEL_SHA256`), its preprocessing (`input_tensor`: a 256x256
/// stretch with PIL's bilinear filter, mean / std 0.5, RGB, `pooler_output`
/// L2-normalized) and its input (the preview decoded at the DCT scale nearest
/// `INPUT_LONG_EDGE`). Bump it on any change to one of them; the app then
/// embeds every file again.
pub const EMBED_VERSION: i64 = 1;

/// The long edge the preview is decoded near, with no re-encode: the stored
/// 404 px thumbnail's q80 re-encode moves the vector too far (the plan's
/// Decision 1).
pub const INPUT_LONG_EDGE: usize = 404;

/// Where the app downloads `SHIPPED`'s file from, at a pinned revision.
pub const MODEL_URL: &str = "https://huggingface.co/onnx-community/siglip2-base-patch32-256-ONNX/resolve/7efccb86b5b6601bfe9e14326e1c11b40b68d44c/onnx/vision_model.onnx";

/// The SHA-256 of the file at `MODEL_URL`, in lowercase hex.
pub const MODEL_SHA256: &str = "0ddb7507cfcb54c2cd503029949add9dc6586de67164bb69c2c09588cdcc9719";

/// The size of the file at `MODEL_URL`, in bytes.
pub const MODEL_SIZE: u64 = 378_478_689;

/// Where the downloaded file lives, relative to the app's local data dir.
pub const MODEL_FILE: &str = "models/siglip2-base-patch32-256/vision_model.onnx";

type Plan = Arc<TypedRunnableModel>;

/// An optimized plan of one embedding model, shareable across threads.
pub struct Model {
    plan: Plan,
    /// Output index of `spec.output`.
    output: usize,
    spec: ModelSpec,
}

impl Model {
    /// Build the plan of the ONNX `bytes` for the input `spec` describes.
    pub fn load(bytes: &[u8], spec: &ModelSpec) -> Result<Model> {
        let model = tract_onnx::onnx()
            .with_ignore_value_info(true)
            .with_ignore_output_shapes(true)
            .model_for_read(&mut Cursor::new(bytes))?;
        Model::build(model, spec)
    }

    /// `load` of the ONNX file at `path`, which tract maps rather than
    /// reads, so the file's bytes are not held next to the parsed model.
    pub fn load_path(path: &Path, spec: &ModelSpec) -> Result<Model> {
        let model = tract_onnx::onnx()
            .with_ignore_value_info(true)
            .with_ignore_output_shapes(true)
            .model_for_path(path)?;
        Model::build(model, spec)
    }

    fn build(model: InferenceModel, spec: &ModelSpec) -> Result<Model> {
        let model = model.with_input_fact(0, f32::fact([1, 3, spec.size, spec.size]).into())?;
        let names: Vec<String> = model
            .output_outlets()?
            .iter()
            .map(|o| model.outlet_label(*o).unwrap_or_default().to_string())
            .collect();
        let output = names
            .iter()
            .position(|n| n == spec.output)
            .with_context(|| format!("no output {} in {names:?}", spec.output))?;
        let plan = model.into_optimized()?.into_runnable()?;
        Ok(Model {
            plan,
            output,
            spec: spec.clone(),
        })
    }

    pub fn spec(&self) -> &ModelSpec {
        &self.spec
    }
}

/// The plan of `SHIPPED` loaded from `path`, built on the first call and
/// shared by every later one, whatever path it passes; a failed build is
/// kept too, so it fails every call quickly with the same message. Call it
/// only once the file is in place, or the missing file's error sticks.
pub fn shared(path: &Path) -> Result<&'static Model, String> {
    static MODEL: OnceLock<Result<Model, String>> = OnceLock::new();
    MODEL
        .get_or_init(|| Model::load_path(path, SHIPPED).map_err(|e| format!("{e:#}")))
        .as_ref()
        .map_err(Clone::clone)
}

/// The embedding of a preview JPEG of Exif `orientation`, decoded upright at
/// the DCT scale nearest `INPUT_LONG_EDGE`, as the app stores it.
pub fn embed_preview(model: &Model, jpeg: &[u8], orientation: u16) -> Result<Vec<f32>> {
    let up = decode_upright_near(jpeg, orientation, INPUT_LONG_EDGE)?;
    embed_with(model, &up.rgb, up.width, up.height, 1)
}

/// The L2-normalized embedding of the stored `width` x `height` RGB image of
/// Exif `orientation`.
pub fn embed_with(
    model: &Model,
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
) -> Result<Vec<f32>> {
    let input = input_tensor(&model.spec, rgb, width, height, orientation)?;
    let outputs = model.plan.run(tvec!(input.into()))?;
    let mut v = outputs[model.output]
        .try_as_plain_ram()?
        .as_slice::<f32>()?
        .to_vec();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if !norm.is_normal() {
        bail!("embedding has norm {norm}");
    }
    v.iter_mut().for_each(|x| *x /= norm);
    Ok(v)
}

/// The `[1, 3, size, size]` model input of an image: rotated upright,
/// resized per `spec.resize` with `spec.filter`, normalized per channel.
pub fn input_tensor(
    spec: &ModelSpec,
    rgb: &[u8],
    width: usize,
    height: usize,
    orientation: u16,
) -> Result<Tensor> {
    if width == 0 || height == 0 || rgb.len() < width * height * 3 {
        bail!("bad image {width}x{height} with {} bytes", rgb.len());
    }
    let (up, w, h) = upright_rgb(&rgb[..width * height * 3], width, height, orientation);
    let s = spec.size;
    let (rw, rh) = match spec.resize {
        Resize::Stretch => (s, s),
        Resize::ShortSideCrop if w <= h => (s, (s * h / w).max(s)),
        Resize::ShortSideCrop => ((s * w / h).max(s), s),
    };
    let resized = resample(&up, w, h, rw, rh, spec.filter, 3);
    let (x0, y0) = ((rw - s) / 2, (rh - s) / 2);
    let mut out = vec![0f32; 3 * s * s];
    for y in 0..s {
        for x in 0..s {
            let i = ((y + y0) * rw + x + x0) * 3;
            for c in 0..3 {
                let src = if spec.bgr { 2 - c } else { c };
                let v = f32::from(resized[i + src]) / 255.0;
                out[(c * s + y) * s + x] = (v - spec.mean[c]) / spec.std[c];
            }
        }
    }
    Ok(tract_ndarray::Array4::from_shape_vec((1, 3, s, s), out)?.into_tensor())
}

/// PIL's `Image.resize` of an 8-bit image of interleaved `channels`: a
/// separable convolution, horizontal then vertical, whose kernel widens with
/// the downscale (antialiasing), each pass rounded and clamped to 8 bits as
/// PIL does.
pub(crate) fn resample(
    pixels: &[u8],
    w: usize,
    h: usize,
    ow: usize,
    oh: usize,
    filter: Filter,
    channels: usize,
) -> Vec<u8> {
    let horizontal = if ow == w {
        pixels.to_vec()
    } else {
        let coeffs = coefficients(w, ow, filter);
        let mut out = vec![0u8; ow * h * channels];
        for y in 0..h {
            for (ox, (start, k)) in coeffs.iter().enumerate() {
                let o = (y * ow + ox) * channels;
                for c in 0..channels {
                    let sum: f64 = k
                        .iter()
                        .enumerate()
                        .map(|(j, wgt)| f64::from(pixels[(y * w + start + j) * channels + c]) * wgt)
                        .sum();
                    out[o + c] = sum.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        out
    };
    if oh == h {
        return horizontal;
    }
    let coeffs = coefficients(h, oh, filter);
    let mut out = vec![0u8; ow * oh * channels];
    for (oy, (start, k)) in coeffs.iter().enumerate() {
        for x in 0..ow {
            let o = (oy * ow + x) * channels;
            for c in 0..channels {
                let sum: f64 = k
                    .iter()
                    .enumerate()
                    .map(|(j, wgt)| {
                        f64::from(horizontal[((start + j) * ow + x) * channels + c]) * wgt
                    })
                    .sum();
                out[o + c] = sum.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// PIL's `precompute_coeffs`: for each output pixel the first input index and
/// the normalized weights of its window.
fn coefficients(input: usize, output: usize, filter: Filter) -> Vec<(usize, Vec<f64>)> {
    let (support, kernel): (f64, fn(f64) -> f64) = match filter {
        Filter::Bilinear => (1.0, |x| {
            let x = x.abs();
            if x < 1.0 {
                1.0 - x
            } else {
                0.0
            }
        }),
        Filter::Bicubic => (2.0, |x| {
            let a = -0.5;
            let x = x.abs();
            if x < 1.0 {
                ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
            } else if x < 2.0 {
                (((x - 5.0) * x + 8.0) * x - 4.0) * a
            } else {
                0.0
            }
        }),
    };
    let scale = input as f64 / output as f64;
    let filterscale = scale.max(1.0);
    let support = support * filterscale;
    (0..output)
        .map(|o| {
            let center = (o as f64 + 0.5) * scale;
            let min = ((center - support + 0.5).floor().max(0.0)) as usize;
            let max = ((center + support + 0.5).floor() as usize).min(input);
            let mut k: Vec<f64> = (min..max)
                .map(|x| kernel((x as f64 - center + 0.5) / filterscale))
                .collect();
            let total: f64 = k.iter().sum();
            if total != 0.0 {
                k.iter_mut().for_each(|v| *v /= total);
            }
            (min, k)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(t: &Tensor) -> Vec<f32> {
        t.try_as_plain_ram()
            .unwrap()
            .as_slice::<f32>()
            .unwrap()
            .to_vec()
    }

    #[test]
    fn a_flat_image_normalizes_to_the_spec_values_at_the_input_size() {
        for spec in &SPECS {
            let rgb = [255u8, 0, 128].repeat(300 * 200);
            let t = input_tensor(spec, &rgb, 300, 200, 1).unwrap();
            assert_eq!(t.shape(), &[1, 3, spec.size, spec.size], "{}", spec.name);
            let v = values(&t);
            let plane = spec.size * spec.size;
            for (c, raw) in [255.0f32, 0.0, 128.0].into_iter().enumerate() {
                let want = (raw / 255.0 - spec.mean[c]) / spec.std[c];
                assert!(
                    v[c * plane..(c + 1) * plane]
                        .iter()
                        .all(|x| (x - want).abs() < 1e-5),
                    "{} channel {c}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn bgr_swaps_the_first_and_last_planes() {
        let spec = ModelSpec {
            bgr: true,
            mean: [0.0; 3],
            std: [1.0; 3],
            ..spec("siglip2").unwrap().clone()
        };
        let rgb = [255u8, 0, 0].repeat(8 * 8);
        let v = values(&input_tensor(&spec, &rgb, 8, 8, 1).unwrap());
        let plane = spec.size * spec.size;
        assert!(v[..plane].iter().all(|x| *x == 0.0));
        assert!(v[2 * plane..].iter().all(|x| *x == 1.0));
    }

    /// A 400x200 image, left half red and right half blue. Rotated upright
    /// by orientation 6 (90 degrees clockwise) red ends up on top, by 8 at
    /// the bottom, by 3 on the right; a stretch keeps every half whole.
    #[test]
    fn the_image_is_rotated_upright_before_the_resize() {
        let (w, h) = (400, 200);
        let rgb: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                if i % w < w / 2 {
                    [255, 0, 0]
                } else {
                    [0, 0, 255]
                }
            })
            .collect();
        let spec = ModelSpec {
            mean: [0.0; 3],
            std: [1.0; 3],
            ..spec("siglip2").unwrap().clone()
        };
        let s = spec.size;
        let red = |v: &[f32], x: usize, y: usize| v[y * s + x];
        let cases = [
            (1, (10, s / 2), (s - 10, s / 2)),
            (3, (s - 10, s / 2), (10, s / 2)),
            (6, (s / 2, 10), (s / 2, s - 10)),
            (8, (s / 2, s - 10), (s / 2, 10)),
        ];
        for (orientation, red_at, blue_at) in cases {
            let v = values(&input_tensor(&spec, &rgb, w, h, orientation).unwrap());
            assert_eq!(
                red(&v, red_at.0, red_at.1),
                1.0,
                "orientation {orientation}"
            );
            assert_eq!(
                red(&v, blue_at.0, blue_at.1),
                0.0,
                "orientation {orientation}"
            );
        }
    }

    #[test]
    fn short_side_crop_keeps_the_center_square() {
        // 300x100, a green middle third: resized to 672x224, its center
        // 224x224 square is all green.
        let (w, h) = (300, 100);
        let rgb: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                if (100..200).contains(&(i % w)) {
                    [0, 255, 0]
                } else {
                    [255, 0, 0]
                }
            })
            .collect();
        let spec = ModelSpec {
            mean: [0.0; 3],
            std: [1.0; 3],
            ..spec("openclip").unwrap().clone()
        };
        let v = values(&input_tensor(&spec, &rgb, w, h, 1).unwrap());
        let plane = spec.size * spec.size;
        let green = &v[plane..2 * plane];
        // The bicubic kernel spans the 3 px either side of the edges.
        let s = spec.size;
        assert!((0..s).all(|y| (4..s - 4).all(|x| (green[y * s + x] - 1.0).abs() < 1e-6)));
    }

    #[test]
    fn coefficients_match_pil_on_a_downscale() {
        // PIL's bilinear 4 -> 2: the triangle kernel widened to 2 px either
        // side of the center, clipped at the edge and normalized.
        let k = coefficients(4, 2, Filter::Bilinear);
        assert_eq!(k[0].0, 0);
        let want = [3.0 / 7.0, 3.0 / 7.0, 1.0 / 7.0];
        assert!(k[0].1.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12));
        assert_eq!(k[1].0, 1);
    }
}
