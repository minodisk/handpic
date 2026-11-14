//! gpui-component's global `Theme`, dark and mapped to the app's shadcn/ui
//! Neutral dark tokens from `main.rs`'s color constants.

use gpui::{App, Pixels, px, rgb, rgba};
use gpui_component::{Theme, ThemeColor, ThemeMode};

use crate::{
    ACCENT_COLOR, BORDER_COLOR, ERROR_COLOR, FOREGROUND_COLOR, INPUT_BORDER, MUTED_COLOR,
    MUTED_FILL, PANE_COLOR, PRIMARY_COLOR, RING_COLOR,
};

/// `style.css`'s `--background`, which nothing outside the components paints.
const BACKGROUND_COLOR: u32 = 0x0a0a0a;
/// The backdrop behind the app's dialogs and the settings modal.
const OVERLAY_COLOR: u32 = 0x00000099;
/// The text size of the app's controls.
const FONT_SIZE: Pixels = px(13.);
/// `rounded_md`, the buttons' and menus' radius, and `rounded_lg`, the dialog
/// boxes'.
const RADIUS: Pixels = px(6.);
const RADIUS_LG: Pixels = px(8.);

/// Sets the theme to dark and maps it, leaving it there whatever the OS
/// appearance. Runs after `gpui_component::init`, which leaves it light.
pub fn apply(cx: &mut App) {
    // A mode switch reloads the registered theme over color edits made in
    // the same `update`, so the switch comes first.
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        map(&mut theme.colors);
        theme.font_size = FONT_SIZE;
        theme.radius = RADIUS;
        theme.radius_lg = RADIUS_LG;
    });
}

fn map(colors: &mut ThemeColor) {
    colors.background = rgb(BACKGROUND_COLOR).into();
    colors.foreground = rgb(FOREGROUND_COLOR).into();
    colors.popover = rgb(PANE_COLOR).into();
    colors.popover_foreground = rgb(FOREGROUND_COLOR).into();
    colors.primary = rgb(PRIMARY_COLOR).into();
    colors.primary_foreground = rgb(PANE_COLOR).into();
    colors.secondary = rgb(ACCENT_COLOR).into();
    colors.secondary_foreground = rgb(FOREGROUND_COLOR).into();
    colors.muted = rgb(MUTED_FILL).into();
    colors.muted_foreground = rgb(MUTED_COLOR).into();
    colors.accent = rgb(ACCENT_COLOR).into();
    colors.accent_foreground = rgb(FOREGROUND_COLOR).into();
    colors.danger = rgb(ERROR_COLOR).into();
    colors.border = rgba(BORDER_COLOR).into();
    colors.input = rgba(INPUT_BORDER).into();
    colors.ring = rgba(RING_COLOR).into();
    colors.overlay = rgba(OVERLAY_COLOR).into();
    colors.scrollbar_thumb = rgba(BORDER_COLOR).into();
}

#[cfg(test)]
mod tests {
    use gpui::{Hsla, rgba};
    use gpui_component::ThemeColor;

    use super::map;

    fn hex(value: u32) -> Hsla {
        rgba(value).into()
    }

    #[test]
    fn maps_the_neutral_dark_tokens() {
        let mut colors = ThemeColor::default();
        map(&mut colors);
        assert_eq!(colors.background, hex(0x0a0a0aff));
        assert_eq!(colors.foreground, hex(0xfafafaff));
        assert_eq!(colors.popover, hex(0x171717ff));
        assert_eq!(colors.popover_foreground, hex(0xfafafaff));
        assert_eq!(colors.primary, hex(0xe5e5e5ff));
        assert_eq!(colors.primary_foreground, hex(0x171717ff));
        assert_eq!(colors.secondary, hex(0x262626ff));
        assert_eq!(colors.secondary_foreground, hex(0xfafafaff));
        assert_eq!(colors.muted, hex(0x262626ff));
        assert_eq!(colors.muted_foreground, hex(0xa1a1a1ff));
        assert_eq!(colors.accent, hex(0x262626ff));
        assert_eq!(colors.accent_foreground, hex(0xfafafaff));
        assert_eq!(colors.danger, hex(0xff6467ff));
        assert_eq!(colors.border, hex(0xffffff1a));
        assert_eq!(colors.input, hex(0xffffff26));
        assert_eq!(colors.ring, hex(0x737373ff));
        assert_eq!(colors.overlay, hex(0x00000099));
        assert_eq!(colors.scrollbar_thumb, hex(0xffffff1a));
    }
}
