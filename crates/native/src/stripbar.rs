use std::{cell::Cell, rc::Rc};

use gpui::{Bounds, Pixels, Point, Size, point, px, size};
use gpui_component::scroll::ScrollbarHandle;

/// The strip's scrollbar handle over `strip.rs`'s model. The view writes the
/// model's offset and content width with `sync` before the strip is drawn;
/// an offset the scrollbar sets waits here until the view `take`s it.
#[derive(Clone, Default)]
pub struct StripBar(Rc<Cell<Bar>>);

#[derive(Clone, Copy, Default)]
struct Bar {
    offset: f32,
    content: f32,
    requested: Option<f32>,
}

impl StripBar {
    pub fn sync(&self, offset: f32, content: f32) {
        let bar = self.0.get();
        self.0.set(Bar {
            offset,
            content,
            ..bar
        });
    }

    /// The offset the scrollbar last set, as the strip's positive offset,
    /// handed over once.
    pub fn take(&self) -> Option<f32> {
        let mut bar = self.0.get();
        let requested = bar.requested.take();
        self.0.set(bar);
        requested
    }
}

impl ScrollbarHandle for StripBar {
    /// Not read: the scrollbar's viewport is the strip `div`'s layout.
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        Bounds::default()
    }

    fn offset(&self) -> Point<Pixels> {
        point(px(-self.0.get().offset), px(0.))
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        let mut bar = self.0.get();
        bar.requested = Some(-f32::from(offset.x));
        self.0.set(bar);
    }

    fn content_size(&self) -> Size<Pixels> {
        size(px(self.0.get().content), px(0.))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px};
    use gpui_component::scroll::ScrollbarHandle;

    use super::StripBar;
    use crate::strip::{CELL_PITCH, Strip};

    fn strip(len: usize) -> Strip {
        Strip::new(
            &(0..len)
                .map(|i| format!("D:/a/{i}.ARW"))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn reports_the_strips_offset_negated() {
        let bar = StripBar::default();
        bar.sync(308., 0.);
        assert_eq!(bar.offset(), point(px(-308.), px(0.)));
    }

    #[test]
    fn content_is_the_cells_times_the_pitch() {
        let strip = strip(10);
        let bar = StripBar::default();
        bar.sync(strip.offset(), strip.content_width());
        assert_eq!(bar.content_size().width, px(10. * CELL_PITCH));
    }

    #[test]
    fn a_set_offset_is_handed_over_once_as_positive() {
        let bar = StripBar::default();
        bar.set_offset(point(px(-200.), px(0.)));
        assert_eq!(bar.take(), Some(200.));
        assert_eq!(bar.take(), None);
    }

    #[test]
    fn a_sync_is_not_a_request() {
        let bar = StripBar::default();
        bar.sync(500., 1540.);
        assert_eq!(bar.take(), None);
    }

    #[test]
    fn a_sync_keeps_a_pending_request() {
        let bar = StripBar::default();
        bar.set_offset(point(px(-40.), px(0.)));
        bar.sync(0., 1540.);
        assert_eq!(bar.take(), Some(40.));
    }
}
