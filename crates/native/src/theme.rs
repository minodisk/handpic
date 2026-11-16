//! gpui-component's global `Theme`, in the mode the stored `theme` setting
//! resolves to: that mode's chosen theme, either Neutral (the built-in one
//! mapped to the app's shadcn/ui Neutral tokens) or one of gpui-kit's
//! theme files embedded from `themes/`, and the app's own `Roles` beside it:
//! the one source of the colors the app paints over its themed surfaces.

use std::{rc::Rc, sync::LazyLock};

use gpui_kit::component::{
    ActiveTheme, Theme, ThemeColor, ThemeConfig, ThemeMode, ThemeRegistry, ThemeSet,
};
use gpui_kit::{App, Global, Hsla, Pixels, WindowAppearance, px, rgb, rgba};
use serde_json::{Value, json};

/// The app's own theme in either mode, offered first in both lists.
pub const NEUTRAL: &str = "Neutral";

/// gpui-kit's theme files at the `gpui-component` 0.9.1 release, unchanged
/// (`themes/LICENSE`); a bump of the crate copies them again.
const FILES: [&str; 21] = [
    include_str!("../themes/adventure.json"),
    include_str!("../themes/alduin.json"),
    include_str!("../themes/asciinema.json"),
    include_str!("../themes/aurora.json"),
    include_str!("../themes/ayu.json"),
    include_str!("../themes/catppuccin.json"),
    include_str!("../themes/everforest.json"),
    include_str!("../themes/fahrenheit.json"),
    include_str!("../themes/flexoki.json"),
    include_str!("../themes/gruvbox.json"),
    include_str!("../themes/harper.json"),
    include_str!("../themes/hybrid.json"),
    include_str!("../themes/jellybeans.json"),
    include_str!("../themes/kibble.json"),
    include_str!("../themes/macos-classic.json"),
    include_str!("../themes/mellifluous.json"),
    include_str!("../themes/molokai.json"),
    include_str!("../themes/solarized.json"),
    include_str!("../themes/spaceduck.json"),
    include_str!("../themes/tokyonight.json"),
    include_str!("../themes/twilight.json"),
];

/// Every theme the files hold, in file order.
static SHIPPED: LazyLock<Vec<ThemeConfig>> = LazyLock::new(|| {
    FILES
        .iter()
        .flat_map(|file| {
            serde_json::from_str::<ThemeSet>(file)
                .expect("an embedded theme file parses")
                .themes
        })
        .collect()
});

/// The shipped theme of `mode` named `name`.
fn shipped(name: &str, mode: ThemeMode) -> Option<&'static ThemeConfig> {
    SHIPPED
        .iter()
        .find(|config| config.mode == mode && config.name.as_ref() == name)
}

/// The names the Appearance tab offers for `mode`: Neutral, then the
/// shipped themes of that mode.
pub fn names(mode: ThemeMode) -> impl Iterator<Item = &'static str> {
    std::iter::once(NEUTRAL).chain(
        SHIPPED
            .iter()
            .filter(move |config| config.mode == mode)
            .map(|config| config.name.as_ref()),
    )
}

/// One mode's Neutral values, each `ThemeColor` field `map` pins.
struct Palette {
    background: u32,
    foreground: u32,
    /// `--card` / `--popover`, the panes, the status and title bars and the
    /// settings' sidebar.
    popover: u32,
    primary: u32,
    primary_foreground: u32,
    /// `--secondary` / `--muted` / `--accent`.
    accent: u32,
    accent_foreground: u32,
    muted_foreground: u32,
    /// `--destructive`.
    danger: u32,
    /// `style.css`'s destructive button on hover, `--destructive` at 90 %
    /// over the card.
    danger_hover: u32,
    danger_foreground: u32,
    border: u32,
    input: u32,
    /// Also `--sidebar-ring`, the tree cursor's outline.
    ring: u32,
    /// The backdrop behind the dialogs.
    overlay: u32,
    /// `.button.outline`'s fill.
    button: u32,
}

/// `style.css`'s Neutral dark tokens.
const DARK: Palette = Palette {
    background: 0x0a0a0aff,
    foreground: 0xfafafaff,
    popover: 0x171717ff,
    primary: 0xe5e5e5ff,
    primary_foreground: 0x171717ff,
    accent: 0x262626ff,
    accent_foreground: 0xfafafaff,
    muted_foreground: 0xa1a1a1ff,
    danger: 0xff6467ff,
    danger_hover: 0xe85c5fff,
    danger_foreground: 0xfafafaff,
    border: 0xffffff1a,
    input: 0xffffff26,
    ring: 0x737373ff,
    overlay: 0x00000099,
    button: 0xffffff0b,
};

/// shadcn/ui's Neutral light tokens (`apps/v4/public/r/colors/neutral.json`,
/// its oklch values as Tailwind's hex), the backdrop kept as the dark one's
/// and the outline button's fill `--background`, as shadcn's `outline`.
const LIGHT: Palette = Palette {
    background: 0xffffffff,
    foreground: 0x0a0a0aff,
    popover: 0xffffffff,
    primary: 0x171717ff,
    primary_foreground: 0xfafafaff,
    accent: 0xf5f5f5ff,
    accent_foreground: 0x171717ff,
    muted_foreground: 0x737373ff,
    danger: 0xe7000bff,
    danger_hover: 0xe91a23ff,
    danger_foreground: 0xfafafaff,
    border: 0xe5e5e5ff,
    input: 0xe5e5e5ff,
    ring: 0xa1a1a1ff,
    overlay: 0x00000099,
    button: 0xffffffff,
};

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

/// The stored `theme` setting's mode: the OS appearance, or one forced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    System,
    Light,
    #[default]
    Dark,
}

impl Mode {
    pub fn setting(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_setting(value: Option<&str>) -> Option<Self> {
        match value {
            Some("system") => Some(Self::System),
            Some("light") => Some(Self::Light),
            Some("dark") => Some(Self::Dark),
            _ => None,
        }
    }
}

/// The value stored under `theme`, an object read field by field: a missing
/// or invalid field is its default, whatever the others hold. `light` and
/// `dark` are the theme of each mode, Neutral unless a shipped theme of that
/// mode has the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub mode: Mode,
    pub light: &'static str,
    pub dark: &'static str,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            light: NEUTRAL,
            dark: NEUTRAL,
        }
    }
}

impl Setting {
    pub fn from_setting(value: Option<&Value>) -> Self {
        let field = |name: &str| value.and_then(|v| v.get(name)).and_then(Value::as_str);
        let theme = |name: &str, mode| {
            field(name)
                .and_then(|name| shipped(name, mode))
                .map_or(NEUTRAL, |config| config.name.as_ref())
        };
        Self {
            mode: Mode::from_setting(field("mode")).unwrap_or_default(),
            light: theme("light", ThemeMode::Light),
            dark: theme("dark", ThemeMode::Dark),
        }
    }

    pub fn setting(self) -> Value {
        json!({"mode": self.mode.setting(), "light": self.light, "dark": self.dark})
    }

    /// The theme chosen for `mode`.
    pub fn name(self, mode: ThemeMode) -> &'static str {
        if mode.is_dark() {
            self.dark
        } else {
            self.light
        }
    }
}

/// The mode `setting` puts in place under the OS `appearance`.
pub fn resolve(setting: Setting, appearance: WindowAppearance) -> ThemeMode {
    match setting.mode {
        Mode::System => appearance.into(),
        Mode::Light => ThemeMode::Light,
        Mode::Dark => ThemeMode::Dark,
    }
}

/// Installs the theme `setting` chooses for the mode it resolves to under the
/// OS `appearance`, maps it when it is Neutral and sets the roles. Runs after
/// `gpui_kit::init`, which leaves it light, and again on a change of
/// the setting or, in System mode, of the appearance.
pub fn apply(setting: Setting, appearance: WindowAppearance, cx: &mut App) {
    let mode = resolve(setting, appearance);
    let registry = ThemeRegistry::global(cx);
    let config = |mode: ThemeMode, default: &Rc<ThemeConfig>| {
        shipped(setting.name(mode), mode).map_or_else(|| default.clone(), |c| Rc::new(c.clone()))
    };
    let light = config(ThemeMode::Light, registry.default_light_theme());
    let dark = config(ThemeMode::Dark, registry.default_dark_theme());
    // `change` loads the mode's config from these slots, Neutral's the
    // built-in `Default Light` / `Default Dark`. It reloads over color edits
    // made in the same `update`, so the map comes after it. `shadow` is reset
    // to its default first: `change` writes it only for a config that sets it,
    // so a theme that turned it off would otherwise leave it off.
    Theme::update(cx, |theme| {
        theme.light_theme = light;
        theme.dark_theme = dark;
        theme.shadow = true;
    });
    Theme::change(mode, None, cx);
    let neutral = setting.name(mode) == NEUTRAL;
    Theme::update(cx, |theme| {
        if neutral {
            map(&mut theme.colors, palette(mode));
        }
        theme.font_size = FONT_SIZE;
        theme.radius = RADIUS;
        theme.radius_lg = RADIUS_LG;
    });
    let roles = Roles::of(&cx.theme().colors, neutral && mode.is_dark());
    cx.set_global(roles);
}

fn palette(mode: ThemeMode) -> &'static Palette {
    if mode.is_dark() { &DARK } else { &LIGHT }
}

fn map(colors: &mut ThemeColor, p: &Palette) {
    colors.background = rgba(p.background).into();
    colors.foreground = rgba(p.foreground).into();
    colors.popover = rgba(p.popover).into();
    colors.popover_foreground = rgba(p.foreground).into();
    colors.primary = rgba(p.primary).into();
    colors.primary_foreground = rgba(p.primary_foreground).into();
    colors.secondary = rgba(p.accent).into();
    colors.secondary_foreground = rgba(p.accent_foreground).into();
    colors.muted = rgba(p.accent).into();
    colors.muted_foreground = rgba(p.muted_foreground).into();
    colors.accent = rgba(p.accent).into();
    colors.accent_foreground = rgba(p.accent_foreground).into();
    colors.danger = rgba(p.danger).into();
    colors.border = rgba(p.border).into();
    colors.input = rgba(p.input).into();
    colors.ring = rgba(p.ring).into();
    colors.overlay = rgba(p.overlay).into();
    colors.scrollbar_thumb = rgba(p.border).into();
    colors.status_bar = rgba(p.popover).into();
    colors.status_bar_border = rgba(p.border).into();
    // `TitleBar`'s surface and its border, as the panes'.
    colors.title_bar = rgba(p.popover).into();
    colors.title_bar_border = rgba(p.border).into();
    // The settings' `Sidebar`, on the settings box's own surface, its
    // selected page lit as the hand-drawn tab list's was.
    colors.sidebar = rgba(p.popover).into();
    colors.sidebar_foreground = rgba(p.foreground).into();
    colors.sidebar_accent = rgba(p.accent).into();
    colors.sidebar_accent_foreground = rgba(p.accent_foreground).into();
    colors.sidebar_border = rgba(p.border).into();
    colors.sidebar_primary = rgba(p.primary).into();
    colors.sidebar_primary_foreground = rgba(p.primary_foreground).into();
    // `ListItem`'s selected row, hover and cursor outline (the folder tree's
    // open row, hover and keyboard cursor), which the built-in theme gives in
    // blue; `selection` also paints `Input`'s selected text.
    colors.list_active = rgba(p.accent).into();
    colors.list_hover = rgba(p.accent).into();
    colors.selection = rgba(p.ring).into();
    // `Button`'s default variant. The registered theme derives these at load
    // and `update` does not derive them again from the colors above.
    colors.button = rgba(p.button).into();
    colors.button_hover = rgba(p.accent).into();
    colors.button_active = rgba(p.accent).into();
    colors.button_foreground = rgba(p.foreground).into();
    // `ButtonVariants::danger`, derived at load the same way.
    colors.button_danger = rgba(p.danger).into();
    colors.button_danger_hover = rgba(p.danger_hover).into();
    colors.button_danger_active = rgba(p.danger_hover).into();
    colors.button_danger_foreground = rgba(p.danger_foreground).into();
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use gpui_kit::component::{ThemeColor, ThemeMode, ThemeSet};
    use gpui_kit::{Hsla, WindowAppearance, rgba};
    use serde_json::json;

    use super::{
        DARK, FILES, LIGHT, Mode, NEUTRAL, Palette, Roles, SHIPPED, Setting, map, names, resolve,
    };

    fn hex(value: u32) -> Hsla {
        rgba(value).into()
    }

    fn mapped_with(palette: &Palette) -> ThemeColor {
        let mut colors = ThemeColor::default();
        map(&mut colors, palette);
        colors
    }

    fn mapped() -> ThemeColor {
        mapped_with(&DARK)
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
        assert_eq!(colors.list_active, hex(0x262626ff));
        assert_eq!(colors.list_hover, hex(0x262626ff));
        assert_eq!(colors.selection, hex(0x737373ff));
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

    #[test]
    fn maps_the_neutral_light_tokens() {
        let colors = mapped_with(&LIGHT);
        assert_eq!(colors.background, hex(0xffffffff));
        assert_eq!(colors.foreground, hex(0x0a0a0aff));
        assert_eq!(colors.popover, hex(0xffffffff));
        assert_eq!(colors.popover_foreground, hex(0x0a0a0aff));
        assert_eq!(colors.primary, hex(0x171717ff));
        assert_eq!(colors.primary_foreground, hex(0xfafafaff));
        assert_eq!(colors.secondary, hex(0xf5f5f5ff));
        assert_eq!(colors.secondary_foreground, hex(0x171717ff));
        assert_eq!(colors.muted, hex(0xf5f5f5ff));
        assert_eq!(colors.muted_foreground, hex(0x737373ff));
        assert_eq!(colors.accent, hex(0xf5f5f5ff));
        assert_eq!(colors.accent_foreground, hex(0x171717ff));
        assert_eq!(colors.danger, hex(0xe7000bff));
        assert_eq!(colors.border, hex(0xe5e5e5ff));
        assert_eq!(colors.input, hex(0xe5e5e5ff));
        assert_eq!(colors.ring, hex(0xa1a1a1ff));
        assert_eq!(colors.overlay, hex(0x00000099));
        assert_eq!(colors.scrollbar_thumb, hex(0xe5e5e5ff));
        assert_eq!(colors.status_bar, hex(0xffffffff));
        assert_eq!(colors.status_bar_border, hex(0xe5e5e5ff));
        assert_eq!(colors.title_bar, hex(0xffffffff));
        assert_eq!(colors.title_bar_border, hex(0xe5e5e5ff));
        assert_eq!(colors.sidebar, hex(0xffffffff));
        assert_eq!(colors.sidebar_foreground, hex(0x0a0a0aff));
        assert_eq!(colors.sidebar_accent, hex(0xf5f5f5ff));
        assert_eq!(colors.sidebar_accent_foreground, hex(0x171717ff));
        assert_eq!(colors.sidebar_border, hex(0xe5e5e5ff));
        assert_eq!(colors.sidebar_primary, hex(0x171717ff));
        assert_eq!(colors.sidebar_primary_foreground, hex(0xfafafaff));
        assert_eq!(colors.list_active, hex(0xf5f5f5ff));
        assert_eq!(colors.list_hover, hex(0xf5f5f5ff));
        assert_eq!(colors.selection, hex(0xa1a1a1ff));
        assert_eq!(colors.button, hex(0xffffffff));
        assert_eq!(colors.button_hover, hex(0xf5f5f5ff));
        assert_eq!(colors.button_active, hex(0xf5f5f5ff));
        assert_eq!(colors.button_foreground, hex(0x0a0a0aff));
        assert_eq!(colors.button_danger, hex(0xe7000bff));
        assert_eq!(colors.button_danger_hover, hex(0xe91a23ff));
        assert_eq!(colors.button_danger_active, hex(0xe91a23ff));
        assert_eq!(colors.button_danger_foreground, hex(0xfafafaff));
    }

    #[test]
    fn every_embedded_file_is_a_theme_set() {
        for file in FILES {
            let set: ThemeSet = serde_json::from_str(file).unwrap();
            assert!(!set.name.is_empty());
            assert!(!set.themes.is_empty(), "{}", set.name);
        }
        assert_eq!(FILES.len(), 21);
    }

    #[test]
    fn the_shipped_names_are_unique_and_none_is_neutral() {
        let mut seen = HashSet::new();
        for config in SHIPPED.iter() {
            assert!(!config.name.is_empty());
            assert_ne!(config.name.as_ref(), NEUTRAL);
            assert!(seen.insert(config.name.as_ref()), "{}", config.name);
        }
    }

    #[test]
    fn each_list_is_neutral_then_the_themes_of_its_mode() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let listed: Vec<_> = names(mode).collect();
            assert_eq!(listed[0], NEUTRAL);
            let of_mode: Vec<_> = SHIPPED
                .iter()
                .filter(|config| config.mode == mode)
                .map(|config| config.name.as_ref())
                .collect();
            assert!(!of_mode.is_empty());
            assert_eq!(listed[1..], of_mode[..]);
        }
        assert!(names(ThemeMode::Light).any(|name| name == "Catppuccin Latte"));
        assert!(names(ThemeMode::Dark).any(|name| name == "Catppuccin Mocha"));
    }

    #[test]
    fn a_missing_or_invalid_setting_is_neutral_dark() {
        let dark = Setting::default();
        assert_eq!(
            dark,
            Setting {
                mode: Mode::Dark,
                light: NEUTRAL,
                dark: NEUTRAL
            }
        );
        assert_eq!(Setting::from_setting(None), dark);
        for value in [
            json!({}),
            json!("light"),
            json!(null),
            json!({"mode": "sepia"}),
            json!({"mode": 1}),
            json!({"mode": "Light"}),
            json!({"light": 1, "dark": null}),
        ] {
            assert_eq!(Setting::from_setting(Some(&value)), dark, "{value}");
        }
    }

    #[test]
    fn an_unknown_or_wrong_mode_name_is_neutral() {
        for value in [
            json!({"light": "Sepia", "dark": "Sepia"}),
            json!({"light": "Catppuccin Mocha", "dark": "Catppuccin Latte"}),
            json!({"light": "catppuccin latte", "dark": "tokyo night"}),
            json!({"light": "Default Light", "dark": "Default Dark"}),
        ] {
            let setting = Setting::from_setting(Some(&value));
            assert_eq!((setting.light, setting.dark), (NEUTRAL, NEUTRAL), "{value}");
        }
    }

    #[test]
    fn each_field_falls_back_on_its_own() {
        let value = json!({"mode": "sepia", "light": "Ayu Light", "dark": "Ayu Light"});
        assert_eq!(
            Setting::from_setting(Some(&value)),
            Setting {
                mode: Mode::Dark,
                light: "Ayu Light",
                dark: NEUTRAL
            }
        );
    }

    #[test]
    fn each_mode_and_name_is_read_and_round_trips() {
        for (stored, mode) in [
            ("system", Mode::System),
            ("light", Mode::Light),
            ("dark", Mode::Dark),
        ] {
            let value = json!({"mode": stored, "light": "Catppuccin Latte", "dark": "Tokyo Night"});
            let setting = Setting::from_setting(Some(&value));
            assert_eq!(
                setting,
                Setting {
                    mode,
                    light: "Catppuccin Latte",
                    dark: "Tokyo Night"
                }
            );
            assert_eq!(setting.setting(), value);
            assert_eq!(Setting::from_setting(Some(&setting.setting())), setting);
        }
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            for name in names(mode) {
                let setting = Setting {
                    mode: Mode::System,
                    light: if mode.is_dark() { NEUTRAL } else { name },
                    dark: if mode.is_dark() { name } else { NEUTRAL },
                };
                assert_eq!(Setting::from_setting(Some(&setting.setting())), setting);
                assert_eq!(setting.name(mode), name);
            }
        }
    }

    #[test]
    fn system_follows_the_os_appearance_and_the_others_are_forced() {
        let of = |mode| Setting {
            mode,
            ..Setting::default()
        };
        for (appearance, follows) in [
            (WindowAppearance::Light, ThemeMode::Light),
            (WindowAppearance::VibrantLight, ThemeMode::Light),
            (WindowAppearance::Dark, ThemeMode::Dark),
            (WindowAppearance::VibrantDark, ThemeMode::Dark),
        ] {
            assert_eq!(resolve(of(Mode::System), appearance), follows);
            assert_eq!(resolve(of(Mode::Light), appearance), ThemeMode::Light);
            assert_eq!(resolve(of(Mode::Dark), appearance), ThemeMode::Dark);
        }
    }
}
