//! PIL's resampling filters, which the full-size eye sharpness resizes its
//! eye crops through (the prototype it ports resized with PIL).

/// PIL's resampling filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Bilinear,
    Bicubic,
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
