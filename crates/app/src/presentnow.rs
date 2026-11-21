use gpui_kit::Window;

/// The window's paint processed at once rather than on the next tick of
/// GPUI's Windows vsync thread, so a keyed page's frame is presented before
/// the next DWM composition instead of after it. Holds the window's HWND on
/// Windows; nothing elsewhere, nor for a window with no Win32 handle (the UI
/// tests' `TestAppContext`), where `paint` does nothing.
///
/// `paint` runs GPUI's frame callback synchronously, which touches the
/// `App`: call it from a foreground task's body, never inside an update,
/// `cx.defer` or `window.defer`, where the `App` is still borrowed.
#[derive(Clone, Copy)]
pub struct PresentNow(#[cfg_attr(not(windows), allow(dead_code))] Option<isize>);

impl PresentNow {
    #[cfg(windows)]
    pub fn of(window: &Window) -> Self {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Win32(handle)) => Self(Some(handle.hwnd.get())),
            _ => Self(None),
        }
    }

    #[cfg(not(windows))]
    pub fn of(_window: &Window) -> Self {
        Self(None)
    }

    #[cfg(windows)]
    pub fn paint(self) {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow};
        let Some(hwnd) = self.0 else { return };
        let hwnd = HWND(hwnd as *mut _);
        // SAFETY: the handle is the app window's, read from GPUI; a handle
        // whose window is gone makes the call fail, which is only logged.
        let done = unsafe { RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE | RDW_UPDATENOW) };
        if !done.as_bool() {
            log::debug!(
                "RedrawWindow failed: {}",
                windows::core::Error::from_thread()
            );
        }
    }

    #[cfg(not(windows))]
    pub fn paint(self) {}
}
