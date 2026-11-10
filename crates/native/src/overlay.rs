//! The `f` overlay over the preview: what it draws for the current file and
//! the painting of the AF focus mark.

use std::sync::{Arc, Mutex};

use gpui::{
    AppContext, Bounds, Context, Hsla, PathBuilder, Pixels, Point, Window, hsla, point, px, rgb,
};
use riffle_backend::index::{self, Focus, Index};

use crate::{
    Riffle,
    mark::{self, FocusMark},
};

/// The crosshair's arm length and the gap left around the point, as the
/// Tauri app's `FOCUS_MARK_ARM` / `FOCUS_MARK_GAP`.
const ARM: f32 = 8.;
const GAP: f32 = 4.;
const OUTLINE_WIDTH: f32 = 4.;
const LINE_WIDTH: f32 = 2.;

fn outline_color() -> Hsla {
    hsla(0., 0., 0., 0.8)
}

/// What the overlay draws on the current file, once its image is shown.
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub focus: Focus,
    pub orientation: u16,
}

/// The toggle, the current file's stored `Focus` and the file the shown
/// image belongs to.
#[derive(Default)]
pub struct Marks {
    on: bool,
    /// Bumped by every read, so only the latest read's answer is kept.
    read: u64,
    /// The file a read answered for and its focus, `None` for a file with no
    /// focus (a JPEG, no row).
    focus: Option<(String, Option<Focus>)>,
    /// The file the shown image belongs to and its orientation.
    shown: Option<(String, u16)>,
}

impl Marks {
    /// Flip the overlay; whether it is now on.
    pub fn toggle(&mut self) -> bool {
        self.on = !self.on;
        self.on
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// A read of the current file's focus starts: its ticket.
    pub fn reading(&mut self) -> u64 {
        self.read += 1;
        self.read
    }

    /// A read came back; dropped unless it is the latest.
    pub fn answer(&mut self, ticket: u64, path: String, focus: Option<Focus>) -> bool {
        if ticket != self.read {
            return false;
        }
        self.focus = Some((path, focus));
        true
    }

    /// The image of `path`, turned upright from `orientation`, is shown, or
    /// none is.
    pub fn show(&mut self, shown: Option<(String, u16)>) {
        self.shown = shown;
    }

    /// The mark to draw while the overlay is on and both the shown image and
    /// the focus read belong to `current`.
    pub fn mark(&self, current: Option<&str>) -> Option<Mark> {
        let current = current.filter(|_| self.on)?;
        let (shown, orientation) = self.shown.as_ref()?;
        let (read, focus) = self.focus.as_ref()?;
        (shown == current && read == current).then_some(())?;
        Some(Mark {
            focus: (*focus)?,
            orientation: *orientation,
        })
    }
}

/// Read `path`'s stored focus through the index's read-only connection on
/// the background executor and hand it to the view, unless `path` is no
/// longer current by then.
pub fn read(reader: Arc<Mutex<Index>>, path: String, ticket: u64, cx: &mut Context<Riffle>) {
    cx.spawn(async move |this, cx| {
        let entry = {
            let path = path.clone();
            cx.background_spawn(async move { index::lock(&reader).entry(&path) })
                .await
        };
        let focus = entry
            .unwrap_or_else(|e| {
                log::warn!("failed to read the focus of {path}: {e}");
                None
            })
            .and_then(|e| e.focus);
        this.update(cx, |this, cx| {
            if this.current_path().as_deref() == Some(path.as_str())
                && this.marks.answer(ticket, path, focus)
            {
                cx.notify();
            }
        })
        .ok();
    })
    .detach();
}

/// Paint `mark` over the image drawn upright into `image`: the crosshair and
/// the AF frame, stroked first in a dark outline and then in the state color.
pub fn paint(mark: Option<&Mark>, image: Bounds<Pixels>, window: &mut Window) {
    let Some(&Mark { focus, orientation }) = mark else {
        return;
    };
    let (width, height) = (f32::from(image.size.width), f32::from(image.size.height));
    let (draw_width, draw_height) = if matches!(orientation, 6 | 8) {
        (height, width)
    } else {
        (width, height)
    };
    let Some(focus_mark) = mark::focus_mark(Some(&focus), draw_width, draw_height) else {
        return;
    };
    let center = image.center();
    let color: Hsla = rgb(focus_mark.state.color()).into();
    for (width, color) in [(OUTLINE_WIDTH, outline_color()), (LINE_WIDTH, color)] {
        let mut path = PathBuilder::stroke(px(width));
        for (from, to) in segments(&focus_mark, width) {
            path.move_to(at(from, orientation, center));
            path.line_to(at(to, orientation, center));
        }
        if let Some(rect) = focus_mark.rect {
            let corners = [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y),
                (rect.x + rect.width, rect.y + rect.height),
                (rect.x, rect.y + rect.height),
            ];
            let corners = corners.map(|c| at(c, orientation, center));
            path.add_polygon(&corners, true);
        }
        match path.build() {
            Ok(path) => window.paint_path(path, color),
            Err(e) => log::warn!("failed to build the focus mark: {e}"),
        }
    }
}

fn at(p: (f32, f32), orientation: u16, center: Point<Pixels>) -> Point<Pixels> {
    let (x, y) = mark::upright_point(p, orientation);
    center + point(px(x), px(y))
}

/// The four crosshair arms on the unrotated drawing. GPUI's stroke has butt
/// caps, so each arm is lengthened by half the stroke `width` at both ends,
/// where the Tauri mark's square caps reach.
fn segments(m: &FocusMark, width: f32) -> [((f32, f32), (f32, f32)); 4] {
    let (x, y, cap) = (m.x, m.y, width / 2.);
    let (near, far) = (GAP - cap, GAP + ARM + cap);
    [
        ((x - far, y), (x - near, y)),
        ((x + near, y), (x + far, y)),
        ((x, y - far), (x, y - near)),
        ((x, y + near), (x, y + far)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::MarkState;
    use riffle_backend::index::{FocusPoint, StoredEyes};
    use riffle_core::candidate::FocusCandidate;

    fn focus() -> Focus {
        Focus {
            point: Some(FocusPoint {
                sensor_w: 100,
                sensor_h: 100,
                x: 50,
                y: 50,
                frame: None,
            }),
            manual_focus: false,
            eye_focus: None,
            candidate: FocusCandidate::Unknown,
            eyes: StoredEyes::new(None, None, None, None, None),
        }
    }

    fn shown(path: &str) -> Marks {
        let mut marks = Marks::default();
        marks.toggle();
        marks.show(Some((path.into(), 6)));
        let ticket = marks.reading();
        marks.answer(ticket, path.into(), Some(focus()));
        marks
    }

    #[test]
    fn marks_the_current_file_once_its_image_and_focus_are_in() {
        let marks = shown("a");
        assert_eq!(marks.mark(Some("a")).map(|m| m.orientation), Some(6));
    }

    #[test]
    fn no_mark_while_off() {
        let mut marks = shown("a");
        assert!(!marks.toggle());
        assert!(marks.mark(Some("a")).is_none());
    }

    #[test]
    fn no_mark_on_the_previous_files_image() {
        let mut marks = shown("a");
        let ticket = marks.reading();
        marks.answer(ticket, "b".into(), Some(focus()));
        assert!(marks.mark(Some("b")).is_none());
        marks.show(Some(("b".into(), 1)));
        assert!(marks.mark(Some("b")).is_some());
        marks.show(None);
        assert!(marks.mark(Some("b")).is_none());
    }

    #[test]
    fn no_mark_before_the_current_files_focus_is_read_or_without_one() {
        let mut marks = shown("a");
        marks.show(Some(("b".into(), 1)));
        assert!(marks.mark(Some("b")).is_none());
        let ticket = marks.reading();
        marks.answer(ticket, "b".into(), None);
        assert!(marks.mark(Some("b")).is_none());
    }

    #[test]
    fn only_the_latest_read_is_kept() {
        let mut marks = shown("a");
        let older = marks.reading();
        let newer = marks.reading();
        let pass2 = Focus {
            candidate: FocusCandidate::NotCandidate,
            ..focus()
        };
        assert!(marks.answer(newer, "a".into(), Some(pass2)));
        assert!(!marks.answer(older, "a".into(), Some(focus())));
        let state = marks
            .mark(Some("a"))
            .map(|m| mark::af_eye_state(Some(&m.focus)));
        assert_eq!(state, Some(MarkState::Low));
    }

    #[test]
    fn arms_reach_where_square_caps_would() {
        let m = FocusMark {
            x: 0.,
            y: 0.,
            rect: None,
            state: MarkState::Neutral,
        };
        assert_eq!(
            segments(&m, 4.),
            [
                ((-14., 0.), (-2., 0.)),
                ((2., 0.), (14., 0.)),
                ((0., -14.), (0., -2.)),
                ((0., 2.), (0., 14.)),
            ]
        );
    }

    #[test]
    fn a_point_lands_on_the_upright_image_around_its_center() {
        let center = point(px(500.), px(300.));
        assert_eq!(at((10., 20.), 1, center), point(px(510.), px(320.)));
        assert_eq!(at((10., 20.), 6, center), point(px(480.), px(310.)));
    }
}
