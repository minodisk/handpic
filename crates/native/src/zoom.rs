use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{AppContext, Bounds, Context, Corners, Pixels, RenderImage, Window, point, px, size};
use image::{Frame, RgbaImage};
use riffle_core::{arw::FocusLocation, partial, reader};

use crate::{Riffle, guarded, orient};

/// The largest crop per axis, in JPEG pixels, as the Tauri app's.
pub const CROP_MAX: usize = 1024;

/// A resize streams while the window edge is dragged; the crop is cut again
/// once it has settled this long, as the Tauri app's.
pub const CROP_RESIZE_DEBOUNCE: Duration = Duration::from_millis(150);

/// The crop size in unrotated JPEG pixels for a viewport of `width` x
/// `height` device pixels: each axis is capped, and the two are swapped for
/// the quarter turns, where the viewport's width runs along the JPEG's height.
pub fn crop_size(orientation: u16, width: usize, height: usize) -> (usize, usize) {
    let w = width.clamp(1, CROP_MAX);
    let h = height.clamp(1, CROP_MAX);
    if matches!(orientation, 6 | 8) {
        (h, w)
    } else {
        (w, h)
    }
}

/// A crop to read: for which page, at which viewport in device pixels, and
/// the `z` press it answers, if any.
#[derive(Debug, PartialEq)]
pub struct Request {
    pub seq: u64,
    pub viewport: (u32, u32),
    pub keypress: Option<Instant>,
}

/// What a crop that came back asks of the view.
#[derive(Debug, PartialEq)]
pub enum Arrived<C> {
    /// It is the current page's and is now held; `release` is the one it
    /// replaced, and `resized` says the viewport changed while it was read.
    Shown { release: Option<C>, resized: bool },
    /// It is another page's: release it and read the current one, if zoomed.
    Stale {
        release: Option<C>,
        request: Option<Request>,
    },
    /// The current page's read failed.
    Failed(String),
}

struct Held<C> {
    crop: C,
    viewport: (u32, u32),
}

/// The 1:1 view's state, a port of the Tauri frontend's `toggleZoom` /
/// `requestCrop`: one read in flight, the crop held for the current page, and
/// the current page's preview facts the placeholder needs.
pub struct Zoom<C> {
    on: bool,
    /// Bumped by every page and listing; a crop carries the one it was read
    /// for.
    seq: u64,
    in_flight: bool,
    held: Option<Held<C>>,
    shown: Option<(u64, Option<FocusLocation>, u16)>,
}

impl<C> Default for Zoom<C> {
    fn default() -> Self {
        Self {
            on: false,
            seq: 0,
            in_flight: false,
            held: None,
            shown: None,
        }
    }
}

impl<C> Zoom<C> {
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The current page's crop.
    pub fn crop(&self) -> Option<&C> {
        self.held.as_ref().map(|held| &held.crop)
    }

    /// The current page's preview is on screen, with its focus point and
    /// orientation.
    pub fn shown(&mut self, focus: Option<FocusLocation>, orientation: u16) {
        self.shown = Some((self.seq, focus, orientation));
    }

    /// The current page's focus point and orientation, once its preview is
    /// on screen.
    pub fn preview(&self) -> Option<(Option<FocusLocation>, u16)> {
        self.shown
            .filter(|(seq, _, _)| *seq == self.seq)
            .map(|(_, focus, orientation)| (focus, orientation))
    }

    fn request(&mut self, viewport: (u32, u32), keypress: Option<Instant>) -> Option<Request> {
        if self.in_flight {
            return None;
        }
        self.in_flight = true;
        Some(Request {
            seq: self.seq,
            viewport,
            keypress,
        })
    }

    /// `z`. Turning on reuses the crop held for this page and viewport, else
    /// asks for one.
    pub fn toggle(
        &mut self,
        has_files: bool,
        viewport: (u32, u32),
        now: Instant,
    ) -> Option<Request> {
        if !has_files {
            return None;
        }
        self.on = !self.on;
        if !self.on || self.held.as_ref().is_some_and(|h| h.viewport == viewport) {
            return None;
        }
        self.request(viewport, Some(now))
    }

    /// Another page or listing is current: the held crop is another file's,
    /// to release. Zoomed, the new page's crop is asked for; an empty listing
    /// leaves the view.
    pub fn page(&mut self, has_files: bool, viewport: (u32, u32)) -> (Option<C>, Option<Request>) {
        self.seq += 1;
        let release = self.held.take().map(|held| held.crop);
        if !has_files {
            self.on = false;
        }
        let request = if self.on {
            self.request(viewport, None)
        } else {
            None
        };
        (release, request)
    }

    /// A read for `seq` came back; `viewport` is the one now.
    pub fn arrived(
        &mut self,
        seq: u64,
        crop: Result<(C, (u32, u32)), String>,
        viewport: (u32, u32),
    ) -> Arrived<C> {
        self.in_flight = false;
        if seq != self.seq {
            return Arrived::Stale {
                release: crop.ok().map(|(crop, _)| crop),
                request: if self.on {
                    self.request(viewport, None)
                } else {
                    None
                },
            };
        }
        match crop {
            Ok((crop, requested)) => {
                let release = self
                    .held
                    .replace(Held {
                        crop,
                        viewport: requested,
                    })
                    .map(|held| held.crop);
                Arrived::Shown {
                    release,
                    resized: self.on && requested != viewport,
                }
            }
            Err(e) => Arrived::Failed(e),
        }
    }

    /// The viewport changed: the page to re-cut the crop for once it settles,
    /// while zoomed.
    pub fn resized(&self) -> Option<u64> {
        self.on.then_some(self.seq)
    }

    /// The viewport settled after a resize seen on page `seq`.
    pub fn settled(&mut self, seq: u64, viewport: (u32, u32)) -> Option<Request> {
        if !self.on || seq != self.seq || self.held.as_ref().is_some_and(|h| h.viewport == viewport)
        {
            return None;
        }
        self.request(viewport, None)
    }
}

/// The point of interest `(x, y)` of a `width` x `height` crop in unrotated
/// JPEG pixels, in the crop `orient::rotate_bgra` turns upright.
pub fn upright_point(orientation: u16, width: u32, height: u32, (x, y): (u32, u32)) -> (u32, u32) {
    match orientation {
        3 => (width - 1 - x, height - 1 - y),
        6 => (height - 1 - y, x),
        8 => (y, width - 1 - x),
        _ => (x, y),
    }
}

/// The device pixel the point of interest lands on: the center of the canvas
/// at `origin` with `size` (logical pixels), floored to a whole device pixel.
pub fn canvas_center(origin: (f32, f32), size: (f32, f32), scale: f32) -> (f32, f32) {
    (
        ((origin.0 + size.0 / 2.) * scale).floor(),
        ((origin.1 + size.1 / 2.) * scale).floor(),
    )
}

/// Where the upright crop's top-left goes, in device pixels, so its point of
/// interest sits on `center`.
pub fn crop_origin(center: (f32, f32), point: (u32, u32)) -> (f32, f32) {
    (center.0 - point.0 as f32, center.1 - point.1 as f32)
}

/// A rectangle around the point of interest, in device pixels: the point is
/// `x` from its left edge and `y` from its top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Where the upright preview stands in for the crop, at the full JPEG's
/// scale, a port of the Tauri `placeholderRect`. `full` is the full JPEG's
/// unrotated size from a crop of this page; without one the full JPEG is
/// taken to be the sensor size. `None` for a focus point without a sensor
/// size.
pub fn placeholder(
    preview: (u32, u32),
    orientation: u16,
    focus: FocusLocation,
    full: Option<(u32, u32)>,
) -> Option<Rect> {
    if focus.sensor_w == 0 || focus.sensor_h == 0 {
        return None;
    }
    let (pw, ph) = if matches!(orientation, 6 | 8) {
        (preview.1, preview.0)
    } else {
        preview
    };
    let (sensor_w, sensor_h) = (f32::from(focus.sensor_w), f32::from(focus.sensor_h));
    let (w, h) = match full {
        Some((w, h)) => (w as f32, h as f32),
        None => (sensor_w, ph as f32 * sensor_w / pw as f32),
    };
    let x = f32::from(focus.x) * w / sensor_w;
    let y = f32::from(focus.y) * h / sensor_h;
    Some(match orientation {
        3 => Rect {
            x: w - x,
            y: h - y,
            width: w,
            height: h,
        },
        6 => Rect {
            x: h - y,
            y: x,
            width: h,
            height: w,
        },
        8 => Rect {
            x: y,
            y: w - x,
            width: h,
            height: w,
        },
        _ => Rect {
            x,
            y,
            width: w,
            height: h,
        },
    })
}

/// One read crop, upright.
pub struct Crop {
    pub image: Arc<RenderImage>,
    /// The point of interest in the upright crop.
    pub point: (u32, u32),
    /// The full JPEG's unrotated size.
    pub full: (u32, u32),
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
}

/// The crop of `path` for a viewport of `viewport` device pixels around its
/// focus point (the frame center without one), turned upright.
pub fn read(path: &Path, viewport: (u32, u32)) -> Result<Crop, String> {
    let started = Instant::now();
    let (arw, jpeg) = reader::read_full(path).map_err(|e| e.to_string())?;
    let read = started.elapsed();
    let (w, h) = crop_size(arw.orientation, viewport.0 as usize, viewport.1 as usize);
    let started = Instant::now();
    let focus =
        partial::decode_focus_crop(&jpeg, arw.shot.focus, w, h).map_err(|e| e.to_string())?;
    let mut pixels = focus.crop.pixels;
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let decode = started.elapsed();
    let started = Instant::now();
    let (cw, ch) = (focus.crop.width as u32, focus.crop.height as u32);
    let point = upright_point(
        arw.orientation,
        cw,
        ch,
        (focus.point_x as u32, focus.point_y as u32),
    );
    let (bgra, uw, uh) = orient::rotate_bgra(pixels, cw, ch, arw.orientation);
    let orient = started.elapsed();
    let buffer = RgbaImage::from_raw(uw, uh, bgra).ok_or("the crop came up short")?;
    Ok(Crop {
        image: Arc::new(RenderImage::new(vec![Frame::new(buffer)])),
        point,
        full: (
            focus.crop.image_width as u32,
            focus.crop.image_height as u32,
        ),
        read,
        decode,
        orient,
    })
}

/// Read the crop `request` asks for on the background executor, then hand
/// it to the view.
pub fn send(request: Request, path: PathBuf, cx: &mut Context<Riffle>) {
    let Request {
        seq,
        viewport,
        keypress,
    } = request;
    cx.spawn(async move |this, cx| {
        let crop = cx
            .background_spawn(async move { guarded(|| read(&path, viewport)) })
            .await;
        this.update(cx, |this, cx| {
            this.crop_arrived(seq, crop.map(|c| (c, viewport)), keypress, cx);
        })
        .ok();
    })
    .detach();
}

/// The stages of a crop a `z` press asked for, printed once it is on screen.
pub struct Timed {
    pub key: Instant,
    pub read: Duration,
    pub decode: Duration,
    pub orient: Duration,
}

impl Timed {
    pub fn line(&self, presented: Instant) -> String {
        format!(
            "zoom read={} decode={} orient={} keypressToPixels={}",
            ms(self.read),
            ms(self.decode),
            ms(self.orient),
            ms(presented.saturating_duration_since(self.key)),
        )
    }
}

fn ms(d: Duration) -> String {
    format!("{:.2}ms", d.as_secs_f64() * 1000.0)
}

/// What the preview canvas paints while zoomed.
pub struct Paint {
    pub placeholder: Option<(Arc<RenderImage>, Rect)>,
    pub crop: Option<(Arc<RenderImage>, (u32, u32))>,
}

impl Paint {
    /// The placeholder and the crop over it, one JPEG pixel per device pixel,
    /// the point of interest at the canvas center, clipped to `bounds`.
    pub fn paint(self, bounds: Bounds<Pixels>, window: &mut Window) {
        let scale = window.scale_factor();
        let center = canvas_center(
            (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
            (f32::from(bounds.size.width), f32::from(bounds.size.height)),
            scale,
        );
        let at = |x: f32, y: f32, w: f32, h: f32| {
            Bounds::new(
                point(px(x / scale), px(y / scale)),
                size(px(w / scale), px(h / scale)),
            )
        };
        let mut draw = |image: Arc<RenderImage>, target: Bounds<Pixels>| {
            if let Err(e) = window.paint_image(bounds, target, Corners::default(), image, 0, false)
            {
                println!("error paint: {e}");
            }
        };
        if let Some((image, rect)) = self.placeholder {
            draw(
                image,
                at(
                    center.0 - rect.x,
                    center.1 - rect.y,
                    rect.width,
                    rect.height,
                ),
            );
        }
        if let Some((image, p)) = self.crop {
            let (x, y) = crop_origin(center, p);
            let s = image.size(0);
            draw(image, at(x, y, s.width.0 as f32, s.height.0 as f32));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use riffle_core::arw::FocusLocation;

    use super::{
        Arrived, CROP_MAX, Rect, Request, Zoom, canvas_center, crop_origin, crop_size, placeholder,
        upright_point,
    };
    use crate::orient::rotate_bgra;

    const VIEW: (u32, u32) = (800, 600);

    #[test]
    fn a_quarter_turn_swaps_the_viewport_and_the_cap_bounds_each_axis() {
        assert_eq!(crop_size(1, 300, 200), (300, 200));
        assert_eq!(crop_size(6, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 300, 200), (200, 300));
        assert_eq!(crop_size(8, 4000, 2000), (CROP_MAX, CROP_MAX));
        assert_eq!(crop_size(1, 0, 0), (1, 1));
    }

    fn on(z: &mut Zoom<u8>, now: Instant) -> Request {
        z.toggle(true, VIEW, now).unwrap()
    }

    #[test]
    fn toggling_on_without_a_crop_asks_for_one_with_the_keypress() {
        let mut z = Zoom::<u8>::default();
        let now = Instant::now();
        assert_eq!(
            z.toggle(true, VIEW, now),
            Some(Request {
                seq: 0,
                viewport: VIEW,
                keypress: Some(now)
            })
        );
        assert!(z.is_on());
    }

    #[test]
    fn toggling_back_on_with_the_same_page_and_viewport_reuses_the_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert_eq!(z.toggle(true, VIEW, Instant::now()), None);
        assert!(!z.is_on());
        assert_eq!(z.toggle(true, VIEW, Instant::now()), None);
        assert_eq!(z.crop(), Some(&7));
        assert!(z.toggle(true, VIEW, Instant::now()).is_none());
        assert!(z.toggle(true, (900, 600), Instant::now()).is_some());
    }

    #[test]
    fn a_page_while_zoomed_asks_for_the_new_crop_and_releases_the_old() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let (release, request) = z.page(true, VIEW);
        assert_eq!(release, Some(7));
        assert_eq!(
            request,
            Some(Request {
                seq: 1,
                viewport: VIEW,
                keypress: None
            })
        );
        assert!(z.is_on());
    }

    #[test]
    fn a_page_while_not_zoomed_drops_the_held_crop() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        z.toggle(true, VIEW, Instant::now());
        assert_eq!(z.page(true, VIEW), (Some(7), None));
        assert_eq!(z.crop(), None);
        assert!(z.toggle(true, VIEW, Instant::now()).is_some());
    }

    #[test]
    fn one_request_is_in_flight_at_a_time() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        assert_eq!(z.page(true, VIEW).1, None);
        assert_eq!(z.page(true, VIEW).1, None);
        match z.arrived(r.seq, Ok((7, VIEW)), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, Some(7));
                assert_eq!(request.map(|r| r.seq), Some(2));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(z.page(true, VIEW).1, None);
    }

    #[test]
    fn a_stale_result_is_dropped_and_re_requests_only_while_zoomed() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.toggle(true, VIEW, Instant::now());
        z.page(true, VIEW);
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), VIEW),
            Arrived::Stale {
                release: Some(7),
                request: None
            }
        );
        assert_eq!(z.crop(), None);
        let r = on(&mut z, Instant::now());
        z.page(true, VIEW);
        match z.arrived(r.seq, Err("bad".into()), VIEW) {
            Arrived::Stale { release, request } => {
                assert_eq!(release, None);
                assert_eq!(request.map(|r| r.seq), Some(2));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_current_pages_failure_is_reported() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        assert_eq!(
            z.arrived(r.seq, Err("bad".into()), VIEW),
            Arrived::Failed("bad".into())
        );
        assert!(z.page(true, VIEW).1.is_some());
    }

    #[test]
    fn a_resize_during_a_read_is_noticed_and_re_requested_once_settled() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        let now = (900, 700);
        assert_eq!(z.resized(), Some(0));
        assert_eq!(z.settled(0, now), None);
        assert_eq!(
            z.arrived(r.seq, Ok((7, VIEW)), now),
            Arrived::Shown {
                release: None,
                resized: true
            }
        );
        assert_eq!(z.settled(0, now).map(|r| r.viewport), Some(now));
        assert_eq!(
            z.arrived(0, Ok((8, now)), now),
            Arrived::Shown {
                release: Some(7),
                resized: false
            }
        );
        assert_eq!(z.settled(0, now), None);
    }

    #[test]
    fn a_settle_for_another_page_or_after_leaving_asks_for_nothing() {
        let mut z = Zoom::<u8>::default();
        assert_eq!(z.resized(), None);
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        let seq = z.resized().unwrap();
        let r = z.page(true, VIEW).1.unwrap();
        z.arrived(r.seq, Ok((8, VIEW)), VIEW);
        assert_eq!(z.settled(seq, (900, 700)), None);
        z.toggle(true, VIEW, Instant::now());
        assert_eq!(z.settled(r.seq, (900, 700)), None);
    }

    #[test]
    fn an_empty_listing_leaves_the_view() {
        let mut z = Zoom::<u8>::default();
        let r = on(&mut z, Instant::now());
        z.arrived(r.seq, Ok((7, VIEW)), VIEW);
        assert_eq!(z.page(false, VIEW), (Some(7), None));
        assert!(!z.is_on());
        assert_eq!(z.toggle(false, VIEW, Instant::now()), None);
        assert!(!z.is_on());
    }

    #[test]
    fn the_preview_facts_belong_to_the_page_they_were_shown_on() {
        let mut z = Zoom::<u8>::default();
        let focus = FocusLocation {
            sensor_w: 6000,
            sensor_h: 4000,
            x: 10,
            y: 20,
        };
        z.shown(Some(focus), 6);
        assert_eq!(z.preview(), Some((Some(focus), 6)));
        z.page(true, VIEW);
        assert_eq!(z.preview(), None);
    }

    #[test]
    fn the_point_of_interest_follows_rotate_bgra() {
        let (w, h) = (5u32, 3u32);
        for orientation in [1u16, 3, 6, 8] {
            for (x, y) in [(0, 0), (1, 2), (4, 1), (3, 0)] {
                let mut buf = vec![0u8; (w * h * 4) as usize];
                buf[((y * w + x) * 4) as usize] = 255;
                let (out, uw, _) = rotate_bgra(buf, w, h, orientation);
                let lit = out.chunks(4).position(|p| p[0] == 255).unwrap() as u32;
                assert_eq!(
                    upright_point(orientation, w, h, (x, y)),
                    (lit % uw, lit / uw),
                    "orientation {orientation} at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn the_crop_is_placed_with_its_point_on_the_canvas_center_device_pixel() {
        assert_eq!(canvas_center((0., 0.), (1600., 900.), 1.5), (1200., 675.));
        assert_eq!(canvas_center((0., 0.), (1601., 901.), 1.25), (1000., 563.));
        assert_eq!(crop_origin((1200., 675.), (512, 300)), (688., 375.));
        assert_eq!(crop_origin((10., 10.), (512, 300)), (-502., -290.));
    }

    const FOCUS: FocusLocation = FocusLocation {
        sensor_w: 6000,
        sensor_h: 4000,
        x: 1500,
        y: 1000,
    };

    #[test]
    fn the_placeholder_takes_the_sensor_size_until_a_crop_gives_the_full_size() {
        assert_eq!(
            placeholder((1600, 1000), 1, FOCUS, None),
            Some(Rect {
                x: 1500.,
                y: 937.5,
                width: 6000.,
                height: 3750.
            })
        );
        assert_eq!(
            placeholder((1600, 1000), 1, FOCUS, Some((3000, 2000))),
            Some(Rect {
                x: 750.,
                y: 500.,
                width: 3000.,
                height: 2000.
            })
        );
    }

    #[test]
    fn the_placeholder_turns_with_the_orientation() {
        let full = Some((3000, 2000));
        assert_eq!(
            placeholder((1500, 1000), 3, FOCUS, full),
            Some(Rect {
                x: 2250.,
                y: 1500.,
                width: 3000.,
                height: 2000.
            })
        );
        assert_eq!(
            placeholder((1000, 1500), 6, FOCUS, full),
            Some(Rect {
                x: 1500.,
                y: 750.,
                width: 2000.,
                height: 3000.
            })
        );
        assert_eq!(
            placeholder((1000, 1500), 8, FOCUS, full),
            Some(Rect {
                x: 500.,
                y: 2250.,
                width: 2000.,
                height: 3000.
            })
        );
        let no_sensor = FocusLocation {
            sensor_w: 0,
            ..FOCUS
        };
        assert_eq!(placeholder((1000, 1500), 1, no_sensor, None), None);
    }
}
