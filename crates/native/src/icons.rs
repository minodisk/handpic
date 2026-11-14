//! The Lucide icons the app draws, served to GPUI's `svg().path(..)` by
//! `Icons`, the application's one `AssetSource`, which falls back to
//! `gpui_kit_assets::Assets` for the icons gpui-component's controls draw.

// Lucide `star`, `square`, `rectangle-vertical`, `rectangle-horizontal` and
// `scan-face` icons, lucide-static v1.48.0
// (https://github.com/lucide-icons/lucide), ISC License, Copyright (c) 2026
// Lucide Icons and Contributors.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// The Face focus Low icon is `scan-face` with its mouth turned down, and the
// filled star and square are `star` and `square` with
// `fill="currentColor"`, derivatives.
//
// `square` is derived from the Feather project, MIT License, Copyright (c)
// 2013-present Cole Bemis.
//
// Full notice in `crates/native/LICENSE-lucide`.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_kit_assets::Assets;

pub const STAR: &str = "icons/star.svg";
pub const STAR_FILLED: &str = "icons/star-filled.svg";
pub const SQUARE: &str = "icons/square.svg";
pub const SQUARE_FILLED: &str = "icons/square-filled.svg";
pub const RECTANGLE_VERTICAL: &str = "icons/rectangle-vertical.svg";
pub const RECTANGLE_HORIZONTAL: &str = "icons/rectangle-horizontal.svg";
pub const SCAN_FACE: &str = "icons/scan-face.svg";
pub const SCAN_FACE_FROWN: &str = "icons/scan-face-frown.svg";

const STAR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/></svg>"#;

const STAR_FILLED_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/></svg>"#;

const SQUARE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/></svg>"#;

const SQUARE_FILLED_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/></svg>"#;

const RECTANGLE_VERTICAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="12" height="20" x="6" y="2" rx="2"/></svg>"#;

const RECTANGLE_HORIZONTAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="12" x="2" y="6" rx="2"/></svg>"#;

const SCAN_FACE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7V5a2 2 0 0 1 2-2h2"/><path d="M17 3h2a2 2 0 0 1 2 2v2"/><path d="M21 17v2a2 2 0 0 1-2 2h-2"/><path d="M7 21H5a2 2 0 0 1-2-2v-2"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><path d="M9 9h.01"/><path d="M15 9h.01"/></svg>"#;

const SCAN_FACE_FROWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7V5a2 2 0 0 1 2-2h2"/><path d="M17 3h2a2 2 0 0 1 2 2v2"/><path d="M21 17v2a2 2 0 0 1-2 2h-2"/><path d="M7 21H5a2 2 0 0 1-2-2v-2"/><path d="M8 16s1.5-2 4-2 4 2 4 2"/><path d="M9 9h.01"/><path d="M15 9h.01"/></svg>"#;

const SVGS: [(&str, &str); 8] = [
    (STAR, STAR_SVG),
    (STAR_FILLED, STAR_FILLED_SVG),
    (SQUARE, SQUARE_SVG),
    (SQUARE_FILLED, SQUARE_FILLED_SVG),
    (RECTANGLE_VERTICAL, RECTANGLE_VERTICAL_SVG),
    (RECTANGLE_HORIZONTAL, RECTANGLE_HORIZONTAL_SVG),
    (SCAN_FACE, SCAN_FACE_SVG),
    (SCAN_FACE_FROWN, SCAN_FACE_FROWN_SVG),
];

pub struct Icons;

impl AssetSource for Icons {
    /// The own paths first; any other from the components' set, a path in
    /// neither loading nothing (`Assets` errs on it).
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match SVGS.iter().find(|(known, _)| *known == path) {
            Some((_, svg)) => Ok(Some(Cow::Borrowed(svg.as_bytes()))),
            None => Ok(Assets.load(path).unwrap_or(None)),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = SVGS
            .iter()
            .filter(|(known, _)| known.starts_with(path))
            .map(|(known, _)| SharedString::from(*known))
            .collect();
        for other in Assets.list(path)? {
            if !paths.contains(&other) {
                paths.push(other);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_path_loads_an_svg() {
        for path in [
            STAR,
            STAR_FILLED,
            SQUARE,
            SQUARE_FILLED,
            RECTANGLE_VERTICAL,
            RECTANGLE_HORIZONTAL,
            SCAN_FACE,
            SCAN_FACE_FROWN,
        ] {
            let bytes = Icons.load(path).unwrap().expect(path);
            assert!(bytes.starts_with(b"<svg "), "{path}");
            assert!(bytes.ends_with(b"</svg>"), "{path}");
        }
    }

    #[test]
    fn an_own_path_wins_over_the_components_one() {
        let bytes = Icons.load(STAR).unwrap().unwrap();
        assert_eq!(bytes.as_ref(), STAR_SVG.as_bytes());
    }

    #[test]
    fn a_component_icon_loads_from_their_set() {
        let bytes = Icons.load("icons/check.svg").unwrap().expect("check");
        assert!(bytes.starts_with(b"<svg"));
    }

    #[test]
    fn an_unknown_path_loads_nothing() {
        assert!(Icons.load("icons/no-such-icon.svg").unwrap().is_none());
        assert!(Icons.load("").unwrap().is_none());
    }

    #[test]
    fn the_listing_names_each_icon_under_the_prefix_once() {
        let listed = Icons.list("icons/").unwrap();
        for (path, _) in SVGS {
            assert_eq!(listed.iter().filter(|p| p.as_ref() == path).count(), 1);
        }
        assert!(listed.iter().any(|p| p.as_ref() == "icons/check.svg"));
        assert!(Icons.list("fonts/").unwrap().is_empty());
    }
}
