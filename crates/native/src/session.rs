//! What a launch restores of the previous session: the folder (`lastFolder`,
//! the Tauri app's key and plain-string value) and the window's geometry
//! (`windowBounds`), with the rules kept free of GPUI types.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The folder a launch opens: the command line's, else the remembered one
/// while it is still a directory and restoring is not suppressed (`--auto`).
pub fn startup_folder(
    cli: Option<PathBuf>,
    remembered: Option<&str>,
    suppressed: bool,
    is_dir: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if cli.is_some() {
        return cli;
    }
    if suppressed {
        return None;
    }
    remembered
        .map(PathBuf::from)
        .filter(|folder| is_dir(folder))
}

/// A rectangle in GPUI's logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Whether the two overlap by a positive area, as GPUI's
    /// `Bounds::intersects`.
    fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Shrunk to fit `area` and moved to lie within it.
    fn within(&self, area: &Rect) -> Rect {
        let width = self.width.min(area.width);
        let height = self.height.min(area.height);
        Rect {
            x: self.x.clamp(area.x, area.x + area.width - width),
            y: self.y.clamp(area.y, area.y + area.height - height),
            width,
            height,
        }
    }
}

/// The window's restore bounds, whether it was maximized, and the UUID of
/// the display it was on.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub bounds: Rect,
    pub maximized: bool,
    pub display: Option<String>,
}

impl Geometry {
    /// The stored `windowBounds`; anything malformed, a non-finite position
    /// or size, or a size that is not positive reads as nothing.
    pub fn from_setting(value: Option<&Value>) -> Option<Geometry> {
        let value = value?.as_object()?;
        let number = |key: &str| {
            value
                .get(key)?
                .as_f64()
                .map(|n| n as f32)
                .filter(|n| n.is_finite())
        };
        let bounds = Rect {
            x: number("x")?,
            y: number("y")?,
            width: number("width").filter(|n| *n > 0.)?,
            height: number("height").filter(|n| *n > 0.)?,
        };
        Some(Geometry {
            bounds,
            maximized: value.get("maximized")?.as_bool()?,
            display: match value.get("display") {
                None | Some(Value::Null) => None,
                Some(display) => Some(display.as_str()?.to_owned()),
            },
        })
    }

    /// Whether the size is positive: a closing Windows window reports its
    /// frame insets around an empty rectangle, a negative size.
    pub fn is_usable(&self) -> bool {
        self.bounds.width > 0. && self.bounds.height > 0.
    }

    pub fn setting(&self) -> Value {
        json!({
            "x": self.bounds.x,
            "y": self.bounds.y,
            "width": self.bounds.width,
            "height": self.bounds.height,
            "maximized": self.maximized,
            "display": self.display,
        })
    }
}

/// One display as `place` sees it.
pub struct Screen {
    pub uuid: Option<String>,
    pub bounds: Rect,
    pub visible: Rect,
}

/// The display the saved window reopens on, by its index in `screens`, and
/// its bounds moved and shrunk into that display's visible area: the saved
/// display when the bounds intersect it, else the first display they
/// intersect, else `None` (the caller's default). Lying within the display,
/// the bounds pass the Windows check that their center is on it.
pub fn place(saved: &Geometry, screens: &[Screen]) -> Option<(usize, Rect)> {
    let on = |screen: &Screen| saved.bounds.intersects(&screen.bounds);
    let index = saved
        .display
        .as_ref()
        .and_then(|uuid| {
            screens
                .iter()
                .position(|screen| screen.uuid.as_ref() == Some(uuid) && on(screen))
        })
        .or_else(|| screens.iter().position(on))?;
    Some((index, saved.bounds.within(&screens[index].visible)))
}

#[cfg(test)]
mod tests {
    use super::{Geometry, Rect, Screen, place, startup_folder};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn screen(uuid: &str, bounds: Rect) -> Screen {
        Screen {
            uuid: Some(uuid.into()),
            bounds,
            visible: Rect {
                height: bounds.height - 40.,
                ..bounds
            },
        }
    }

    fn saved(bounds: Rect, display: Option<&str>) -> Geometry {
        Geometry {
            bounds,
            maximized: false,
            display: display.map(Into::into),
        }
    }

    #[test]
    fn the_command_line_wins_over_the_remembered_folder() {
        let any = |_: &Path| true;
        assert_eq!(
            startup_folder(Some("D:/cli".into()), Some("D:/old"), false, any),
            Some(PathBuf::from("D:/cli"))
        );
        assert_eq!(
            startup_folder(Some("D:/cli".into()), Some("D:/old"), true, any),
            Some(PathBuf::from("D:/cli"))
        );
    }

    #[test]
    fn the_remembered_folder_opens_only_while_a_directory_and_not_suppressed() {
        let is = |dir: &'static str| move |p: &Path| p == Path::new(dir);
        assert_eq!(
            startup_folder(None, Some("D:/old"), false, is("D:/old")),
            Some(PathBuf::from("D:/old"))
        );
        assert_eq!(
            startup_folder(None, Some("D:/old"), false, is("D:/x")),
            None
        );
        assert_eq!(
            startup_folder(None, Some("D:/old"), true, is("D:/old")),
            None
        );
        assert_eq!(startup_folder(None, None, false, |_| true), None);
    }

    #[test]
    fn a_geometry_round_trips_through_its_setting() {
        for geometry in [
            Geometry {
                bounds: rect(-1910.5, 12., 1600., 1000.),
                maximized: true,
                display: Some("6c1a…".into()),
            },
            saved(rect(0., 0., 800., 600.), None),
        ] {
            assert_eq!(
                Geometry::from_setting(Some(&geometry.setting())),
                Some(geometry)
            );
        }
    }

    #[test]
    fn a_malformed_or_degenerate_setting_reads_as_nothing() {
        let valid = json!({"x": 0, "y": 0, "width": 10, "height": 10, "maximized": false});
        assert!(Geometry::from_setting(Some(&valid)).is_some());
        for value in [
            json!("1600x1000"),
            json!({"x": 0, "y": 0, "width": 10, "maximized": false}),
            json!({"x": "0", "y": 0, "width": 10, "height": 10, "maximized": false}),
            json!({"x": 0, "y": 0, "width": 0, "height": 10, "maximized": false}),
            json!({"x": 0, "y": 0, "width": 10, "height": -1, "maximized": false}),
            json!({"x": 0, "y": 0, "width": 1e300, "height": 10, "maximized": false}),
            json!({"x": 0, "y": 0, "width": 10, "height": 10, "maximized": "no"}),
            json!({"x": 0, "y": 0, "width": 10, "height": 10, "maximized": false, "display": 3}),
        ] {
            assert_eq!(Geometry::from_setting(Some(&value)), None, "{value}");
        }
        assert_eq!(Geometry::from_setting(None), None);
    }

    #[test]
    fn only_a_positive_size_is_usable() {
        assert!(saved(rect(-8., -8., 10., 10.), None).is_usable());
        assert!(!saved(rect(8., 19., -16., -39.), None).is_usable());
        assert!(!saved(rect(0., 0., 0., 10.), None).is_usable());
    }

    #[test]
    fn the_saved_display_comes_first() {
        let screens = [
            screen("a", rect(0., 0., 1920., 1080.)),
            screen("b", rect(1920., 0., 1920., 1080.)),
        ];
        let window = rect(1800., 100., 800., 600.);
        assert_eq!(
            place(&saved(window, Some("b")), &screens),
            Some((1, rect(1920., 100., 800., 600.)))
        );
        assert_eq!(
            place(&saved(window, Some("a")), &screens),
            Some((0, rect(1120., 100., 800., 600.)))
        );
    }

    #[test]
    fn a_missing_display_falls_back_to_an_intersecting_one() {
        let screens = [
            screen("a", rect(0., 0., 1920., 1080.)),
            screen("b", rect(1920., 0., 1920., 1080.)),
        ];
        let window = rect(2000., 100., 800., 600.);
        assert_eq!(
            place(&saved(window, Some("gone")), &screens),
            Some((1, window))
        );
        assert_eq!(place(&saved(window, None), &screens), Some((1, window)));
        assert_eq!(
            place(&saved(window, Some("a")), &screens),
            Some((1, window))
        );
    }

    #[test]
    fn bounds_off_every_display_place_nothing() {
        let screens = [screen("a", rect(0., 0., 1920., 1080.))];
        let window = rect(5000., 5000., 800., 600.);
        assert_eq!(place(&saved(window, Some("a")), &screens), None);
        assert_eq!(place(&saved(window, None), &[]), None);
    }

    #[test]
    fn the_placement_lies_within_the_visible_area() {
        let screens = [screen("a", rect(0., 0., 1920., 1080.))];
        assert_eq!(
            place(&saved(rect(-100., -50., 800., 600.), Some("a")), &screens),
            Some((0, rect(0., 0., 800., 600.)))
        );
        assert_eq!(
            place(&saved(rect(1500., 900., 800., 600.), Some("a")), &screens),
            Some((0, rect(1120., 440., 800., 600.)))
        );
        assert_eq!(
            place(&saved(rect(10., 10., 4000., 3000.), Some("a")), &screens),
            Some((0, rect(0., 0., 1920., 1040.)))
        );
    }
}
