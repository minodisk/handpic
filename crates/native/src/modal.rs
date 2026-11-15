//! The shell the confirmation dialogs share as gpui-component `Dialog`s in
//! `Root`'s layer, and the plumbing that opens and closes one to follow its
//! flow's model.
//!
//! The shell turns off every way the component closes itself (Escape and
//! Enter through its `Dialog` key context, a backdrop press, the close
//! button, `on_ok` / `on_cancel`), so only the flow closes it, and its
//! entrance and exit animations. Its own backdrop is off too: on Windows the
//! component's backdrop is a window drag area, so a press on it would move
//! the window. The view draws `backdrop` in its place.

use gpui::{
    AnyElement, AnyWindowHandle, App, DispatchPhase, MouseDownEvent, Pixels, Window, canvas,
    deferred, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, WindowExt,
    dialog::{Dialog, DialogEntrance},
};

/// The boxes' width, as the hand-drawn layers had it.
const WIDTH: Pixels = px(640.);

/// Whether a flow's dialog is open in `Root`'s layer. The open and the close
/// land after the current update, through the window's handle, so a flow
/// that changes from an event without a `Window` follows the same path.
pub struct Modal {
    window: AnyWindowHandle,
    open: bool,
}

impl Modal {
    pub fn new(window: AnyWindowHandle) -> Self {
        Self {
            window,
            open: false,
        }
    }

    /// Open the dialog `build` fills, inside the shell, unless it is open.
    /// `build` runs again on every rebuild of the dialog layer.
    pub fn open(
        &mut self,
        cx: &mut App,
        build: impl Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    ) {
        if self.open {
            return;
        }
        self.open = true;
        let window = self.window;
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    window.open_dialog(cx, move |dialog, window, cx| {
                        let dialog = shell(dialog, cx);
                        build(dialog, window, cx)
                    });
                })
                .ok();
        });
    }

    /// Close the dialog if it is open. The component gives the focus back to
    /// what had it when the dialog opened.
    pub fn close(&mut self, cx: &mut App) {
        if !self.open {
            return;
        }
        self.open = false;
        let window = self.window;
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| window.close_dialog(cx))
                .ok();
        });
    }
}

fn shell(dialog: Dialog, cx: &App) -> Dialog {
    dialog
        .overlay(false)
        .overlay_closable(false)
        .keyboard(false)
        .close_button(false)
        .entrance(DialogEntrance::None)
        .on_ok(|_, _, _| false)
        .on_cancel(|_, _, _| false)
        .w(WIDTH)
        .px_4()
        .py_3()
        .gap_2()
        .bg(cx.theme().popover)
        .text_color(cx.theme().popover_foreground)
        .text_size(px(13.))
}

/// The dimmed window under an open dialog, drawn by the view while its flow
/// asks or runs. The dialog's layer occludes the window, so nothing under it
/// is pressed; this also keeps a press anywhere from moving the focus, which
/// the dialog's host would otherwise take from its focused button.
pub fn backdrop(cx: &App) -> AnyElement {
    deferred(
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(cx.theme().overlay)
            .child(
                canvas(
                    |_, _, _| {},
                    |_, _, window, _| {
                        window.on_mouse_event(|_: &MouseDownEvent, phase, window, _| {
                            if phase == DispatchPhase::Capture {
                                window.prevent_default();
                            }
                        });
                    },
                )
                .size_full(),
            ),
    )
    .into_any_element()
}
