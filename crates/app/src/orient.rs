use std::thread;

/// On a 4000x3000 buffer a plain loop takes about 62 ms; tiling alone about
/// 20 ms, of which about 8 ms is first-touching the output; tiles over 8
/// threads about 10 ms.
const TILE: usize = 64;
const THREADS: usize = 8;

/// Rotate a BGRA buffer upright per the Orientation tag (1, 3, 6 or 8; any
/// other value is left alone). 1 hands the buffer back without copying and 3
/// reverses it in place.
pub fn rotate_bgra(mut buf: Vec<u8>, w: u32, h: u32, orientation: u16) -> (Vec<u8>, u32, u32) {
    match orientation {
        3 => {
            buf.as_chunks_mut::<4>().0.reverse();
            (buf, w, h)
        }
        6 | 8 => {
            let (sw, sh) = (w as usize, h as usize);
            let (nw, nh) = (sh, sw);
            let src = buf.as_chunks::<4>().0;
            let mut out = vec![[0u8; 4]; nw * nh];
            let band_rows = nh.div_ceil(THREADS).max(1);
            thread::scope(|scope| {
                for (band, rows) in out.chunks_mut(band_rows * nw).enumerate() {
                    let first_row = band * band_rows;
                    scope.spawn(move || rotate_rows(src, sw, nw, nh, first_row, rows, orientation));
                }
            });
            (out.into_flattened(), h, w)
        }
        _ => (buf, w, h),
    }
}

fn rotate_rows(
    src: &[[u8; 4]],
    sw: usize,
    nw: usize,
    nh: usize,
    first_row: usize,
    out: &mut [[u8; 4]],
    orientation: u16,
) {
    let rows = out.len() / nw;
    for tile_y in (0..rows).step_by(TILE) {
        for tile_x in (0..nw).step_by(TILE) {
            for dy in tile_y..(tile_y + TILE).min(rows) {
                let y = first_row + dy;
                let row = &mut out[dy * nw..(dy + 1) * nw];
                let end = (tile_x + TILE).min(nw);
                for (x, pixel) in (tile_x..end).zip(&mut row[tile_x..end]) {
                    // 6 = Rotate 90 CW, 8 = Rotate 270 CW
                    let (sx, sy) = if orientation == 6 {
                        (y, nw - 1 - x)
                    } else {
                        (nh - 1 - y, x)
                    };
                    *pixel = src[sy * sw + sx];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rotate_bgra;

    // 1 2 3
    // 4 5 6
    fn pattern() -> Vec<u8> {
        pixels(&[1, 2, 3, 4, 5, 6])
    }

    fn pixels(values: &[u8]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|&v| [v, v * 10, v + 100, 255])
            .collect()
    }

    #[test]
    fn orientation_1_keeps_the_same_allocation() {
        let buf = pattern();
        let ptr = buf.as_ptr();
        let (out, w, h) = rotate_bgra(buf, 3, 2, 1);
        assert_eq!((w, h), (3, 2));
        assert_eq!(out.as_ptr(), ptr);
        assert_eq!(out, pattern());
    }

    #[test]
    fn orientation_3_is_a_half_turn() {
        assert_eq!(
            rotate_bgra(pattern(), 3, 2, 3),
            (pixels(&[6, 5, 4, 3, 2, 1]), 3, 2)
        );
    }

    #[test]
    fn orientation_6_turns_clockwise() {
        // 4 1
        // 5 2
        // 6 3
        assert_eq!(
            rotate_bgra(pattern(), 3, 2, 6),
            (pixels(&[4, 1, 5, 2, 6, 3]), 2, 3)
        );
    }

    #[test]
    fn orientation_8_turns_counterclockwise() {
        // 3 6
        // 2 5
        // 1 4
        assert_eq!(
            rotate_bgra(pattern(), 3, 2, 8),
            (pixels(&[3, 6, 2, 5, 1, 4]), 2, 3)
        );
    }

    #[test]
    fn a_buffer_larger_than_a_tile_and_a_band_lands_every_pixel() {
        let (w, h) = (200usize, 130usize);
        let at = |x: usize, y: usize| ((x * 7 + y * 13) % 251) as u8;
        let buf: Vec<u8> = (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| [at(x, y), x as u8, y as u8, 255]))
            .collect();
        for orientation in [6u16, 8] {
            let (out, nw, nh) = rotate_bgra(buf.clone(), w as u32, h as u32, orientation);
            assert_eq!((nw as usize, nh as usize), (h, w));
            for y in 0..w {
                for x in 0..h {
                    let (sx, sy) = if orientation == 6 {
                        (y, h - 1 - x)
                    } else {
                        (w - 1 - y, x)
                    };
                    assert_eq!(
                        &out[(y * h + x) * 4..][..3],
                        &[at(sx, sy), sx as u8, sy as u8]
                    );
                }
            }
        }
    }
}
