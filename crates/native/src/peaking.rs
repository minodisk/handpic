use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{AppContext, Bounds, Context, Corners, ImageId, Pixels, RenderImage, Window};
use image::{
    Frame, ImageBuffer, Rgba, RgbaImage,
    imageops::{self, FilterType},
};

use crate::Riffle;

/// A pixel is lit when `|gx| + |gy|` of the 3x3 Sobel on its luminance
/// reaches this, as the Tauri app's.
pub const PEAKING_THRESHOLD: f64 = 160.;
/// The lit pixels' color, as RGB.
pub const PEAKING_COLOR: [u8; 3] = [255, 32, 32];

/// The focus peaking overlay of a `width` x `height` BGRA image: BGRA of the
/// same size, opaque `PEAKING_COLOR` where the edge clears
/// `PEAKING_THRESHOLD` and transparent elsewhere, the one-pixel border
/// unlit. The same arithmetic as the Tauri `peaking.ts`.
pub fn peaking(bgra: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut overlay = vec![0u8; width * height * 4];
    if width < 3 || height < 3 {
        return overlay;
    }
    let luma: Vec<f32> = bgra
        .as_chunks::<4>()
        .0
        .iter()
        .take(width * height)
        .map(|p| {
            (0.299 * f64::from(p[2]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[0])) as f32
        })
        .collect();
    let l = |i: usize| f64::from(luma[i]);
    let [red, green, blue] = PEAKING_COLOR;
    for y in 1..height - 1 {
        let row = y * width;
        for x in 1..width - 1 {
            let above = row - width + x;
            let here = row + x;
            let below = row + width + x;
            let gx = l(above + 1) - l(above - 1) + 2. * (l(here + 1) - l(here - 1)) + l(below + 1)
                - l(below - 1);
            let gy = l(below - 1) + 2. * l(below) + l(below + 1)
                - l(above - 1)
                - 2. * l(above)
                - l(above + 1);
            if gx.abs() + gy.abs() >= PEAKING_THRESHOLD {
                overlay[here * 4..here * 4 + 4].copy_from_slice(&[blue, green, red, 255]);
            }
        }
    }
    overlay
}

/// The two images peaking draws over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Fit,
    Crop,
}

/// One overlay to compute: `source` drawn at `width` x `height` device
/// pixels, answered with `tag`.
#[derive(Debug, PartialEq)]
pub struct Job<S> {
    pub tag: u64,
    pub source: S,
    pub width: u32,
    pub height: u32,
}

struct Layer<S, O> {
    source: S,
    size: (u32, u32),
    /// 0 until the layer is sent.
    tag: u64,
    /// When it was made, so the newest unsent layer goes next.
    made: u64,
    overlay: Option<O>,
}

/// The peaking layers, a port of the Tauri `peakingOverlay`: one per view,
/// keyed by its source image and drawn size, with one job in flight. A layer
/// replaced while a job is out is never sent, and its overlay is handed out
/// only for the source and size it was computed for.
pub struct Peaking<S, O> {
    on: bool,
    layers: [Option<Layer<S, O>>; 2],
    next: u64,
    busy: bool,
}

impl<S, O> Default for Peaking<S, O> {
    fn default() -> Self {
        Self {
            on: false,
            layers: [None, None],
            next: 0,
            busy: false,
        }
    }
}

impl<S: Clone + PartialEq, O> Peaking<S, O> {
    /// `e`. Turning off clears both layers and returns their overlays.
    pub fn toggle(&mut self) -> Vec<O> {
        self.on = !self.on;
        if self.on {
            return Vec::new();
        }
        self.layers
            .iter_mut()
            .filter_map(|layer| layer.take().and_then(|layer| layer.overlay))
            .collect()
    }

    /// `view` draws `source` at `size` device pixels: the overlay this
    /// replaces, and the job to send now.
    pub fn want(&mut self, view: View, source: S, size: (u32, u32)) -> (Option<O>, Option<Job<S>>) {
        if !self.on || size.0 < 1 || size.1 < 1 {
            return (None, None);
        }
        let slot = &mut self.layers[view as usize];
        let mut release = None;
        if slot
            .as_ref()
            .is_none_or(|layer| layer.source != source || layer.size != size)
        {
            self.next += 1;
            release = slot
                .replace(Layer {
                    source,
                    size,
                    tag: 0,
                    made: self.next,
                    overlay: None,
                })
                .and_then(|layer| layer.overlay);
        }
        (release, self.send())
    }

    /// The newest unsent layer, unless a job is out.
    fn send(&mut self) -> Option<Job<S>> {
        if self.busy {
            return None;
        }
        let layer = self
            .layers
            .iter_mut()
            .flatten()
            .filter(|layer| layer.tag == 0)
            .max_by_key(|layer| layer.made)?;
        self.next += 1;
        layer.tag = self.next;
        self.busy = true;
        Some(Job {
            tag: layer.tag,
            source: layer.source.clone(),
            width: layer.size.0,
            height: layer.size.1,
        })
    }

    /// The job `tag` came back: the overlay to release when its layer is
    /// gone, and the next job.
    pub fn arrived(&mut self, tag: u64, overlay: O) -> (Option<O>, Option<Job<S>>) {
        self.busy = false;
        let release = match self.layers.iter_mut().flatten().find(|l| l.tag == tag) {
            Some(layer) => layer.overlay.replace(overlay),
            None => Some(overlay),
        };
        (release, self.send())
    }

    /// The layer of `view`: its source, size and overlay once computed. The
    /// overlay is drawn only while that source is drawn at that size.
    pub fn layer(&self, view: View) -> Option<(&S, (u32, u32), Option<&O>)> {
        self.layers[view as usize]
            .as_ref()
            .map(|layer| (&layer.source, layer.size, layer.overlay.as_ref()))
    }
}

/// What the preview canvas needs of one view's layer.
#[derive(Clone, Default)]
pub struct Paint {
    on: bool,
    held: Option<(ImageId, (u32, u32))>,
    overlay: Option<Arc<RenderImage>>,
}

impl Peaking<Arc<RenderImage>, Arc<RenderImage>> {
    pub fn paint(&self, view: View) -> Paint {
        let layer = self.layer(view);
        Paint {
            on: self.on,
            held: layer.map(|(source, size, _)| (source.id, size)),
            overlay: layer.and_then(|(_, _, overlay)| overlay.cloned()),
        }
    }
}

impl Paint {
    /// Paint the overlay of `source` drawn at `target`, clipped to `clip`,
    /// when it was computed for that image and size. The size to ask for
    /// when the layer is another's.
    pub fn paint(
        &self,
        source: &RenderImage,
        clip: Bounds<Pixels>,
        target: Bounds<Pixels>,
        window: &mut Window,
    ) -> Option<(u32, u32)> {
        if !self.on {
            return None;
        }
        let scale = window.scale_factor();
        let size = (
            (f32::from(target.size.width) * scale).round() as u32,
            (f32::from(target.size.height) * scale).round() as u32,
        );
        if self.held != Some((source.id, size)) {
            return Some(size);
        }
        if let Some(overlay) = self.overlay.clone()
            && let Err(e) = window.paint_image(clip, target, Corners::default(), overlay, 0, false)
        {
            println!("error paint: {e}");
        }
        None
    }
}

/// One computed overlay.
pub struct Overlay {
    pub image: Arc<RenderImage>,
    pub size: (u32, u32),
    pub resize: Duration,
    pub compute: Duration,
}

impl Overlay {
    pub fn line(&self) -> String {
        format!(
            "peaking {}x{} resize={} compute={}",
            self.size.0,
            self.size.1,
            ms(self.resize),
            ms(self.compute),
        )
    }
}

fn ms(d: Duration) -> String {
    format!("{:.2}ms", d.as_secs_f64() * 1000.0)
}

/// The overlay of `source` drawn at `width` x `height`: resized to that
/// size first when it differs (bilinear, the nearest to a canvas
/// `drawImage`).
pub fn compute(source: &RenderImage, width: u32, height: u32) -> Overlay {
    let s = source.size(0);
    let (sw, sh) = (s.width.0 as u32, s.height.0 as u32);
    let bytes = source.as_bytes(0).unwrap_or_default();
    let started = Instant::now();
    let resized = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(sw, sh, bytes)
        .filter(|_| (sw, sh) != (width, height))
        .map(|buffer| imageops::resize(&buffer, width, height, FilterType::Triangle).into_raw());
    let resize = started.elapsed();
    let started = Instant::now();
    let (pixels, (w, h)) = match &resized {
        Some(pixels) => (pixels.as_slice(), (width, height)),
        None => (bytes, (sw, sh)),
    };
    let overlay = peaking(pixels, w as usize, h as usize);
    let compute = started.elapsed();
    let buffer = RgbaImage::from_raw(w, h, overlay).unwrap_or_default();
    Overlay {
        image: Arc::new(RenderImage::new(vec![Frame::new(buffer)])),
        size: (w, h),
        resize,
        compute,
    }
}

/// Compute the overlay `job` asks for on the background executor, then hand
/// it to the view.
pub fn send(job: Job<Arc<RenderImage>>, cx: &mut Context<Riffle>) {
    let Job {
        tag,
        source,
        width,
        height,
    } = job;
    cx.spawn(async move |this, cx| {
        let overlay = cx
            .background_spawn(async move { compute(&source, width, height) })
            .await;
        this.update(cx, |this, cx| this.peaking_arrived(tag, overlay, cx))
            .ok();
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{Job, PEAKING_COLOR, PEAKING_THRESHOLD, Peaking, View, peaking};

    /// A BGRA image whose every pixel is `bgr(x, y)`.
    fn image(width: usize, height: usize, bgr: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            for x in 0..width {
                let [b, g, r] = bgr(x, y);
                pixels.extend_from_slice(&[b, g, r, 255]);
            }
        }
        pixels
    }

    fn gray(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> Vec<u8> {
        image(width, height, |x, y| [value(x, y); 3])
    }

    fn lit(overlay: &[u8], width: usize) -> Vec<(usize, usize)> {
        overlay
            .chunks(4)
            .enumerate()
            .filter(|(_, p)| p[3] != 0)
            .map(|(i, _)| (i % width, i / width))
            .collect()
    }

    fn sorted(values: impl Iterator<Item = usize>) -> Vec<usize> {
        let mut values: Vec<usize> = values.collect();
        values.sort_unstable();
        values.dedup();
        values
    }

    #[test]
    fn a_flat_image_lights_nothing() {
        assert!(lit(&peaking(&gray(8, 8, |_, _| 128), 8, 8), 8).is_empty());
    }

    #[test]
    fn a_vertical_step_lights_the_columns_on_each_side_of_it() {
        let overlay = peaking(&gray(8, 6, |x, _| if x < 4 { 0 } else { 255 }), 8, 6);
        let pixels = lit(&overlay, 8);
        assert_eq!(sorted(pixels.iter().map(|p| p.0)), [3, 4]);
        assert_eq!(sorted(pixels.iter().map(|p| p.1)), [1, 2, 3, 4]);
        assert_eq!(pixels.len(), 8);
    }

    #[test]
    fn a_horizontal_step_lights_the_rows_on_each_side_of_it() {
        let overlay = peaking(&gray(6, 8, |_, y| if y < 4 { 255 } else { 0 }), 6, 8);
        let pixels = lit(&overlay, 6);
        assert_eq!(sorted(pixels.iter().map(|p| p.1)), [3, 4]);
        assert_eq!(sorted(pixels.iter().map(|p| p.0)), [1, 2, 3, 4]);
        assert_eq!(pixels.len(), 8);
    }

    #[test]
    fn a_step_below_the_threshold_lights_nothing() {
        let step = (PEAKING_THRESHOLD / 4.) as u8 - 1;
        let overlay = peaking(
            &gray(8, 8, |x, _| if x < 4 { 100 } else { 100 + step }),
            8,
            8,
        );
        assert!(lit(&overlay, 8).is_empty());
    }

    #[test]
    fn the_one_pixel_border_stays_unlit() {
        let checker = gray(8, 8, |x, y| if (x + y) % 2 == 0 { 0 } else { 255 });
        for (x, y) in lit(&peaking(&checker, 8, 8), 8) {
            assert!((1..7).contains(&x) && (1..7).contains(&y), "({x}, {y})");
        }
        let border = gray(3, 3, |x, _| if x == 0 { 0 } else { 255 });
        assert_eq!(lit(&peaking(&border, 3, 3), 3), [(1, 1)]);
    }

    #[test]
    fn the_overlay_is_bgra_of_the_images_size_lit_pixels_opaque_in_the_peaking_color() {
        let overlay = peaking(&gray(5, 7, |x, _| if x < 2 { 0 } else { 255 }), 5, 7);
        assert_eq!(overlay.len(), 5 * 7 * 4);
        let pixels = lit(&overlay, 5);
        assert!(!pixels.is_empty());
        let [r, g, b] = PEAKING_COLOR;
        for (x, y) in pixels {
            let o = (y * 5 + x) * 4;
            assert_eq!(overlay[o..o + 4], [b, g, r, 255]);
        }
    }

    #[test]
    fn tiny_images_do_not_panic() {
        assert_eq!(peaking(&gray(1, 1, |_, _| 0), 1, 1).len(), 4);
        assert_eq!(peaking(&gray(2, 2, |x, _| x as u8 * 255), 2, 2).len(), 16);
    }

    #[test]
    fn the_channel_order_is_bgra() {
        // Next to a step of 200, |gx| is 4 * 0.299 * 200 = 239 through red,
        // over the threshold, and 4 * 0.114 * 200 = 91 through blue, under it.
        let step = |bgr: fn(u8) -> [u8; 3]| {
            let pixels = image(8, 8, |x, _| bgr(if x < 4 { 0 } else { 200 }));
            lit(&peaking(&pixels, 8, 8), 8).len()
        };
        assert_eq!(step(|v| [0, 0, v]), 12);
        assert_eq!(step(|v| [v, 0, 0]), 0);
    }

    const A: (u32, u32) = (800, 600);
    const B: (u32, u32) = (640, 480);

    /// What `Paint::paint` draws: the overlay of `view` when it was
    /// computed for `source` at `size`.
    fn overlay<'a>(
        p: &'a Peaking<u32, &'static str>,
        view: View,
        source: u32,
        size: (u32, u32),
    ) -> Option<&'a &'static str> {
        p.layer(view)
            .filter(|(s, at, _)| **s == source && *at == size)
            .and_then(|(_, _, overlay)| overlay)
    }

    fn on() -> Peaking<u32, &'static str> {
        let mut p = Peaking::default();
        assert!(p.toggle().is_empty());
        p
    }

    fn job(tag: u64, source: u32, size: (u32, u32)) -> Option<Job<u32>> {
        Some(Job {
            tag,
            source,
            width: size.0,
            height: size.1,
        })
    }

    #[test]
    fn nothing_is_asked_for_while_off() {
        let mut p = Peaking::<u32, &str>::default();
        assert_eq!(p.want(View::Fit, 1, A), (None, None));
        assert!(p.layer(View::Fit).is_none());
    }

    #[test]
    fn a_layer_is_sent_once_and_its_overlay_handed_out_for_its_source_and_size_only() {
        let mut p = on();
        let (_, sent) = p.want(View::Fit, 1, A);
        assert_eq!(sent, job(2, 1, A));
        assert_eq!(p.want(View::Fit, 1, A), (None, None));
        assert_eq!(overlay(&p, View::Fit, 1, A), None);
        assert_eq!(p.arrived(2, "fit"), (None, None));
        assert_eq!(overlay(&p, View::Fit, 1, A), Some(&"fit"));
        assert_eq!(overlay(&p, View::Fit, 1, B), None);
        assert_eq!(overlay(&p, View::Fit, 2, A), None);
        assert_eq!(overlay(&p, View::Crop, 1, A), None);
        assert_eq!(p.want(View::Fit, 1, A), (None, None));
    }

    #[test]
    fn the_views_hold_one_layer_each() {
        let mut p = on();
        let (_, fit) = p.want(View::Fit, 1, A);
        p.arrived(fit.unwrap().tag, "fit");
        let (_, crop) = p.want(View::Crop, 2, B);
        p.arrived(crop.unwrap().tag, "crop");
        assert_eq!(overlay(&p, View::Fit, 1, A), Some(&"fit"));
        assert_eq!(overlay(&p, View::Crop, 2, B), Some(&"crop"));
    }

    #[test]
    fn one_job_is_in_flight_and_a_layer_replaced_meanwhile_is_never_sent() {
        let mut p = on();
        let (_, first) = p.want(View::Fit, 1, A);
        let first = first.unwrap();
        assert_eq!(p.want(View::Fit, 2, A), (None, None));
        assert_eq!(p.want(View::Fit, 3, A), (None, None));
        assert_eq!(p.want(View::Crop, 4, B), (None, None));
        let (release, next) = p.arrived(first.tag, "one");
        assert_eq!(release, Some("one"));
        let next = next.unwrap();
        assert_eq!((next.source, next.width), (4, B.0));
        let (_, last) = p.arrived(next.tag, "four");
        assert_eq!(last.map(|j| j.source), Some(3));
        assert_eq!(overlay(&p, View::Crop, 4, B), Some(&"four"));
    }

    #[test]
    fn a_replaced_layer_releases_its_overlay() {
        let mut p = on();
        let (_, sent) = p.want(View::Fit, 1, A);
        p.arrived(sent.unwrap().tag, "one");
        let (release, sent) = p.want(View::Fit, 1, B);
        assert_eq!(release, Some("one"));
        assert_eq!(sent.map(|j| (j.source, j.width)), Some((1, B.0)));
    }

    #[test]
    fn turning_off_clears_both_layers_and_returns_their_overlays() {
        let mut p = on();
        let (_, fit) = p.want(View::Fit, 1, A);
        p.arrived(fit.unwrap().tag, "fit");
        let (_, crop) = p.want(View::Crop, 2, B);
        p.arrived(crop.unwrap().tag, "crop");
        let mut released = p.toggle();
        released.sort_unstable();
        assert_eq!(released, ["crop", "fit"]);
        assert_eq!(p.want(View::Fit, 1, A), (None, None));
        assert!(p.layer(View::Fit).is_none() && p.layer(View::Crop).is_none());
    }

    #[test]
    fn a_reply_after_turning_off_is_released_and_the_next_layer_waits_for_it() {
        let mut p = on();
        let (_, sent) = p.want(View::Fit, 1, A);
        p.toggle();
        p.toggle();
        assert_eq!(p.want(View::Fit, 1, A), (None, None));
        let (release, next) = p.arrived(sent.unwrap().tag, "stale");
        assert_eq!(release, Some("stale"));
        assert_eq!(next.map(|j| j.source), Some(1));
    }

    #[test]
    fn a_zero_size_asks_for_nothing() {
        let mut p = on();
        assert_eq!(p.want(View::Fit, 1, (0, 600)), (None, None));
        assert!(p.layer(View::Fit).is_none());
    }
}
