//! gpui-component's global `Theme`, dark and mapped to the app's shadcn/ui
//! Neutral dark tokens, and the app's own `Roles` beside it: the one source
//! of the colors the app paints over its themed surfaces.

use gpui::{App, Global, Hsla, Pixels, px, rgb, rgba};
use gpui_component::{ActiveTheme, Theme, ThemeColor, ThemeMode};

/// `style.css`'s Neutral dark tokens: `--background`, which nothing outside
/// the components paints, `--foreground`, `--card` / `--popover` (the panes),
/// `--primary`, `--secondary` / `--muted` / `--accent`, `--muted-foreground`,
/// `--destructive`, `--border`, `--input`, `--ring` (also `--sidebar-ring`,
/// the tree cursor's outline), the backdrop behind the dialogs and
/// `.button.outline`'s fill.
const BACKGROUND: u32 = 0x0a0a0a;
const FOREGROUND: u32 = 0xfafafa;
const POPOVER: u32 = 0x171717;
const PRIMARY: u32 = 0xe5e5e5;
const ACCENT: u32 = 0x262626;
const MUTED_FOREGROUND: u32 = 0xa1a1a1;
const DANGER: u32 = 0xff6467;
const BORDER: u32 = 0xffffff1a;
const INPUT: u32 = 0xffffff26;
const RING: u32 = 0x737373ff;
const OVERLAY: u32 = 0x00000099;
const BUTTON: u32 = 0xffffff0b;
/// `style.css`'s destructive button on hover, `--destructive` at 90 % over
/// the card.
const DESTRUCTIVE_HOVER: u32 = 0xe85c5f;
/// The window's rem, which `Root` sets from it every frame: GPUI's default,
/// the one the app's rem-based sizes were drawn at.
const FONT_SIZE: Pixels = px(16.);
/// `rounded_md`, the buttons' and menus' radius, and `rounded_lg`, the dialog
/// boxes'.
const RADIUS: Pixels = px(6.);
const RADIUS_LG: Pixels = px(8.);

/// The colors the app paints that equal no theme token, set with the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roles {
    /// The window's surface behind the strip and the status row, and its text.
    pub surface: Hsla,
    pub text: Hsla,
    /// A strip cell's file name off a label, and its scan failure.
    pub name: Hsla,
    pub failure: Hsla,
    /// `.cell.burst::before`'s band, and the band of a rejected cell.
    pub band: Hsla,
    pub band_dimmed: Hsla,
    /// `.cell.rejected`'s `opacity: 0.45` over the strip's surface.
    pub rejected_dim: Hsla,
}

impl Global for Roles {}

impl Roles {
    /// Today's values for Neutral dark, else derived from the theme's tokens.
    pub fn of(colors: &ThemeColor, neutral_dark: bool) -> Self {
        if neutral_dark {
            return Self {
                surface: rgb(0x000000).into(),
                text: rgb(0xffffff).into(),
                name: rgb(0xd4d4d4).into(),
                failure: rgb(0xa3a3a3).into(),
                band: rgba(0xffffff14).into(),
                // The band at the rejected cell's 0.45 opacity: 0.08 x 0.45.
                band_dimmed: rgba(0xffffff09).into(),
                rejected_dim: rgba(0x0000008c).into(),
            };
        }
        Self {
            surface: colors.background,
            text: colors.foreground,
            name: colors.foreground,
            failure: colors.muted_foreground,
            band: colors.foreground.alpha(0.08),
            band_dimmed: colors.foreground.alpha(0.036),
            rejected_dim: colors.background.alpha(0.55),
        }
    }
}

/// The app's roles under the theme in place.
pub fn roles(cx: &App) -> Roles {
    *cx.global::<Roles>()
}

/// Sets the theme to dark and maps it, leaving it there whatever the OS
/// appearance, and sets the roles. Runs after `gpui_component::init`, which
/// leaves it light.
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
    let roles = Roles::of(&cx.theme().colors, true);
    cx.set_global(roles);
}

fn map(colors: &mut ThemeColor) {
    colors.background = rgb(BACKGROUND).into();
    colors.foreground = rgb(FOREGROUND).into();
    colors.popover = rgb(POPOVER).into();
    colors.popover_foreground = rgb(FOREGROUND).into();
    colors.primary = rgb(PRIMARY).into();
    colors.primary_foreground = rgb(POPOVER).into();
    colors.secondary = rgb(ACCENT).into();
    colors.secondary_foreground = rgb(FOREGROUND).into();
    colors.muted = rgb(ACCENT).into();
    colors.muted_foreground = rgb(MUTED_FOREGROUND).into();
    colors.accent = rgb(ACCENT).into();
    colors.accent_foreground = rgb(FOREGROUND).into();
    colors.danger = rgb(DANGER).into();
    colors.border = rgba(BORDER).into();
    colors.input = rgba(INPUT).into();
    colors.ring = rgba(RING).into();
    colors.overlay = rgba(OVERLAY).into();
    colors.scrollbar_thumb = rgba(BORDER).into();
    colors.status_bar = rgb(POPOVER).into();
    colors.status_bar_border = rgba(BORDER).into();
    // `TitleBar`'s surface and its border, as the panes'.
    colors.title_bar = rgb(POPOVER).into();
    colors.title_bar_border = rgba(BORDER).into();
    // The settings' `Sidebar`, on the settings box's own surface, its
    // selected page lit as the hand-drawn tab list's was.
    colors.sidebar = rgb(POPOVER).into();
    colors.sidebar_foreground = rgb(FOREGROUND).into();
    colors.sidebar_accent = rgb(ACCENT).into();
    colors.sidebar_accent_foreground = rgb(FOREGROUND).into();
    colors.sidebar_border = rgba(BORDER).into();
    colors.sidebar_primary = rgb(PRIMARY).into();
    colors.sidebar_primary_foreground = rgb(POPOVER).into();
    // `Button`'s default variant. `Default Dark` derives these at load and
    // `update` does not derive them again from the colors above.
    colors.button = rgba(BUTTON).into();
    colors.button_hover = rgb(ACCENT).into();
    colors.button_active = rgb(ACCENT).into();
    colors.button_foreground = rgb(FOREGROUND).into();
    // `ButtonVariants::danger`, derived at load the same way.
    colors.button_danger = rgb(DANGER).into();
    colors.button_danger_hover = rgb(DESTRUCTIVE_HOVER).into();
    colors.button_danger_active = rgb(DESTRUCTIVE_HOVER).into();
    colors.button_danger_foreground = rgb(FOREGROUND).into();
}

#[cfg(test)]
mod tests {
    use gpui::{Hsla, rgba};
    use gpui_component::ThemeColor;

    use super::{Roles, map};

    fn hex(value: u32) -> Hsla {
        rgba(value).into()
    }

    fn mapped() -> ThemeColor {
        let mut colors = ThemeColor::default();
        map(&mut colors);
        colors
    }

    #[test]
    fn maps_the_neutral_dark_tokens() {
        let colors = mapped();
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
        assert_eq!(colors.status_bar, hex(0x171717ff));
        assert_eq!(colors.status_bar_border, hex(0xffffff1a));
        assert_eq!(colors.title_bar, hex(0x171717ff));
        assert_eq!(colors.title_bar_border, hex(0xffffff1a));
        assert_eq!(colors.sidebar, hex(0x171717ff));
        assert_eq!(colors.sidebar_foreground, hex(0xfafafaff));
        assert_eq!(colors.sidebar_accent, hex(0x262626ff));
        assert_eq!(colors.sidebar_accent_foreground, hex(0xfafafaff));
        assert_eq!(colors.sidebar_border, hex(0xffffff1a));
        assert_eq!(colors.sidebar_primary, hex(0xe5e5e5ff));
        assert_eq!(colors.sidebar_primary_foreground, hex(0x171717ff));
        assert_eq!(colors.button, hex(0xffffff0b));
        assert_eq!(colors.button_hover, hex(0x262626ff));
        assert_eq!(colors.button_active, hex(0x262626ff));
        assert_eq!(colors.button_foreground, hex(0xfafafaff));
        assert_eq!(colors.button_danger, hex(0xff6467ff));
        assert_eq!(colors.button_danger_hover, hex(0xe85c5fff));
        assert_eq!(colors.button_danger_active, hex(0xe85c5fff));
        assert_eq!(colors.button_danger_foreground, hex(0xfafafaff));
    }

    #[test]
    fn neutral_dark_roles_are_todays_colors() {
        let roles = Roles::of(&mapped(), true);
        assert_eq!(roles.surface, hex(0x000000ff));
        assert_eq!(roles.text, hex(0xffffffff));
        assert_eq!(roles.name, hex(0xd4d4d4ff));
        assert_eq!(roles.failure, hex(0xa3a3a3ff));
        assert_eq!(roles.band, hex(0xffffff14));
        assert_eq!(roles.band_dimmed, hex(0xffffff09));
        assert_eq!(roles.rejected_dim, hex(0x0000008c));
    }

    #[test]
    fn other_themes_derive_the_roles_from_their_tokens() {
        let colors = mapped();
        let roles = Roles::of(&colors, false);
        assert_eq!(roles.surface, colors.background);
        assert_eq!(roles.text, colors.foreground);
        assert_eq!(roles.name, colors.foreground);
        assert_eq!(roles.failure, colors.muted_foreground);
        assert_eq!(roles.band, colors.foreground.alpha(0.08));
        assert_eq!(roles.band_dimmed, colors.foreground.alpha(0.036));
        assert_eq!(roles.rejected_dim, colors.background.alpha(0.55));
    }
}
